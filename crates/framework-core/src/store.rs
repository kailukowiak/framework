use crate::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use ts_rs::TS;
use uuid::Uuid;

/// How many edits undo reaches back through.
///
/// Bounded rather than endless, now that an edit can be a write to a data
/// file. Every step still reachable by undo is a version of that file the
/// document has to be able to reproduce, so depth is not free the way it is
/// when history is only a list of small document changes — and an undo
/// stack long enough to reach an edit from an hour ago is one nobody trusts
/// enough to use anyway.
const UNDO_DEPTH: usize = 10;

/// One undoable edit, kept as operations rather than as a document.
///
/// `inverse` puts the document back and is computed against the state
/// before `forward` applied; `forward` is what re-applies it, which is all
/// redo needs because undo has by then restored the state it was resolved
/// against.
#[derive(Debug, Clone)]
pub(crate) struct HistoryEntry {
    forward: Vec<ReplicatedOperation>,
    inverse: Vec<ReplicatedOperation>,
}

#[derive(Debug, Clone)]
pub struct Store {
    pub(crate) document: Document,
    pub(crate) version_vector: VersionVector,
    pub(crate) tutorial_version: Option<u32>,
    pub(crate) undo: Vec<HistoryEntry>,
    pub(crate) redo: Vec<HistoryEntry>,
    pub(crate) sorted_page_cache: SortedPageCache,
}

impl Store {
    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn new(mut document: Document) -> Self {
        document.normalize_frame_names();
        Self {
            document,
            version_vector: VersionVector::new(),
            tutorial_version: None,
            undo: Vec::new(),
            redo: Vec::new(),
            sorted_page_cache: SortedPageCache::default(),
        }
    }

    /// Starts a workbook that must stay in lockstep with this build's lessons.
    pub fn new_tutorial(document: Document) -> Self {
        let mut store = Self::new(document);
        store.tutorial_version = Some(FRAMEWORK_TUTORIAL_VERSION);
        store
    }

    pub fn load_or_demo(path: &Path) -> Self {
        Self::load(path).unwrap_or_else(|_| Self::new(Document::demo()))
    }

    pub fn load(path: &Path) -> Result<Self, CoreError> {
        let json = fs::read_to_string(path).map_err(|error| CoreError::Load(error.to_string()))?;
        let value: serde_json::Value =
            serde_json::from_str(&json).map_err(|error| CoreError::Load(error.to_string()))?;
        let (document, version_vector, tutorial_version) = if value.get("format").is_some() {
            let file: FrameworkDocumentFile = serde_json::from_value(value)
                .map_err(|error| CoreError::Load(error.to_string()))?;
            if file.format != FRAMEWORK_FILE_FORMAT {
                return Err(CoreError::Load(format!(
                    "Unsupported document format '{}'",
                    file.format
                )));
            }
            if file.format_version != FRAMEWORK_FILE_VERSION {
                return Err(CoreError::Load(format!(
                    "Unsupported FrameWork document version {}",
                    file.format_version
                )));
            }
            if let Some(version) = file.tutorial_version
                && version != FRAMEWORK_TUTORIAL_VERSION
            {
                let age = if version < FRAMEWORK_TUTORIAL_VERSION {
                    "older"
                } else {
                    "newer"
                };
                return Err(CoreError::Load(format!(
                    "This tutorial was made for an {age} FrameWork build. Reset tutorials in the Library to get a compatible copy"
                )));
            }
            (file.document, file.version_vector, file.tutorial_version)
        } else {
            (
                serde_json::from_value(value)
                    .map_err(|error| CoreError::Load(error.to_string()))?,
                VersionVector::new(),
                None,
            )
        };
        let mut document = document;
        document.normalize_frame_names();
        // A relative path is relative to this file; an absolute one was
        // written by an older build, or points somewhere of the user's own,
        // and is only rewritten if it no longer resolves.
        document.resolve_artifact_paths(path);
        document.relink_artifacts(path);
        Ok(Self {
            document,
            version_vector,
            tutorial_version,
            undo: Vec::new(),
            redo: Vec::new(),
            sorted_page_cache: SortedPageCache::default(),
        })
    }

    pub fn save(&self, path: &Path) -> Result<(), CoreError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| CoreError::Persistence(error.to_string()))?;
        }
        // What goes on disk names its artifacts relative to itself, so the
        // document and its sidecar can be moved together. In memory they
        // stay absolute: everything that reads a parquet does so without
        // knowing where the document lives, or whether it has been saved at
        // all.
        let mut document = self.document.clone();
        document.relativize_artifact_paths(path);
        let json = if is_framework_document_path(path) {
            CollaborationPaths::for_document(path, &self.document.id)?.ensure_exists()?;
            serde_json::to_string_pretty(&FrameworkDocumentFile {
                format: FRAMEWORK_FILE_FORMAT.into(),
                format_version: FRAMEWORK_FILE_VERSION,
                tutorial_version: self.tutorial_version,
                document,
                version_vector: self.version_vector.clone(),
            })
        } else {
            // Keep explicit `.json` paths backward compatible for existing MCP
            // configurations and debugging workflows.
            serde_json::to_string_pretty(&document)
        }
        .map_err(|error| CoreError::Persistence(error.to_string()))?;
        write_replacing(path, json.as_bytes())
    }

    pub fn document_id(&self) -> &str {
        &self.document.id
    }

    pub fn frame_connector(&self, frame_id: &str) -> Option<&ConnectorRecipe> {
        self.document
            .frame(frame_id)
            .ok()
            .and_then(|frame| frame.connector.as_ref())
    }

    pub fn frame_artifact_id(&self, frame_id: &str) -> Option<&str> {
        self.document
            .frame(frame_id)
            .ok()
            .and_then(|frame| frame.artifact.as_ref())
            .map(|artifact| artifact.id.as_str())
    }

    /// Whether this frame's snapshot has fallen behind what it reads from.
    pub fn snapshot_is_stale(&self, frame_id: &str) -> bool {
        self.document.snapshot_is_stale(frame_id)
    }

    /// Whether anything this frame reads from is serving a stale snapshot.
    pub fn upstream_snapshot_is_stale(&self, frame_id: &str) -> bool {
        self.document.upstream_snapshot_is_stale(frame_id)
    }

    /// Every cached frame, ordered so each comes after the frames it reads
    /// from — the sequence a document-wide refresh walks.
    ///
    /// The list is every snapshot rather than only the stale ones, and that
    /// is deliberate: refreshing a parent moves its children's fingerprints,
    /// so a child that looked fresh when the pass started is stale by the
    /// time the pass reaches it. Callers walk the whole list and ask
    /// [`Store::snapshot_is_stale`] at each step, which sees the document as
    /// it stands right then. Asking up front for "the stale ones" would
    /// answer for a document that no longer exists by the second refresh.
    pub fn snapshot_refresh_order(&self) -> Vec<Id> {
        self.document.snapshot_refresh_order()
    }

    /// Computes a derived frame's rows once and caches them to a parquet
    /// snapshot beside the document, so later reads scan the snapshot
    /// instead of re-running the transformation. Refreshing is the same
    /// call: it recomputes and replaces the snapshot.
    ///
    /// Materializing deliberately reads *live* even if a snapshot already
    /// exists, otherwise a refresh would just copy the stale snapshot back
    /// onto itself.
    pub fn materialize_frame(
        &mut self,
        frame_id: &str,
        data_directory: &Path,
    ) -> Result<DocumentView, CoreError> {
        let artifact = self.write_frame_snapshot(frame_id, data_directory)?;
        self.apply(Operation::SetFrameMaterialization {
            frame_id: frame_id.to_string(),
            artifact,
        })
    }

    /// Works out a value from live data and writes the answer down.
    ///
    /// The cheap half of the bargain a frame snapshot offers: an explicitly
    /// captured answer records a few bytes instead of forcing the whole frame
    /// to be recorded. Scratchwork does not require this to read live data;
    /// its ordinary contract is to recompute.
    ///
    /// Refreshing is this same call again. It reads live deliberately, so a
    /// refresh cannot copy the existing answer back over itself.
    pub fn freeze_value(
        &mut self,
        object_id: &str,
        data_directory: &Path,
    ) -> Result<DocumentView, CoreError> {
        let frozen = self.write_value_snapshot(object_id, data_directory)?;
        self.apply(Operation::SetFrozenValue {
            object_id: object_id.to_string(),
            frozen: Some(frozen),
        })
    }

    /// The computing and writing half, without recording it — so a caller
    /// that journals its own edits can do the write first and hand the
    /// finished answer to the operation, the way materializing a frame does.
    pub fn write_value_snapshot(
        &self,
        object_id: &str,
        data_directory: &Path,
    ) -> Result<FrozenValue, CoreError> {
        let expression = self.document.value_expression(object_id)?;
        // Deliberately the live evaluator rather than the one the view uses:
        // that one prefers a frozen answer, so a refresh would copy the old
        // answer back over itself.
        let is_line = self.document.block_line(object_id).is_some();
        let (_, series) = match is_line {
            true => self.document.evaluate_scratchwork_series(&expression),
            false => self.document.evaluate_to_series(&expression),
        }
        .map_err(CoreError::Formula)?;
        // A line may write down a list, because a line may *be* a list — the
        // scratchpad is the one surface where an answer is not required to
        // fold to a single value, and reading a column of a frame with no
        // snapshot is exactly where that matters. A value card is one value
        // by construction, so it keeps the refusal.
        let name = self.document.value_name(object_id);
        // A block line is not an object in its own right — it is held by the
        // block — so failing to find one is what says this is a line.
        if series.len() != 1 && !is_line {
            return Err(CoreError::Formula(format!(
                "‘{name}’ holds one value, and this is {}. Fold it down — \
                 .sum(), .mean(), .max() — or keep it on a scratchpad line, \
                 which may hold a list.",
                series.len()
            )));
        }
        let frame = crate::data::import::frame_of_series(&name, series)?;
        Ok(FrozenValue {
            artifact: crate::data::import::write_frame_artifact(frame, data_directory, &name)?,
            fingerprint: self.document.value_fingerprint(&expression),
            taken_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Lets the value go back to being worked out every time.
    pub fn thaw_value(&mut self, object_id: &str) -> Result<DocumentView, CoreError> {
        self.apply(Operation::SetFrozenValue {
            object_id: object_id.to_string(),
            frozen: None,
        })
    }

    /// Deletes the artifact files nothing points at any more.
    ///
    /// Editing owned data writes a new parquet each time, and refreshing a
    /// connector or a snapshot does the same, so a document that has been
    /// worked in accumulates versions of files it no longer reads. They are
    /// kept on purpose while anything can still reach them — undo does, which
    /// is what makes going back cheap — and they are worth reclaiming once
    /// nothing can.
    ///
    /// Reachability is computed rather than guessed at, from three places.
    /// The document names what it reads. The undo and redo stacks name what
    /// stepping through them would need. And the journal names what an event
    /// *not yet applied here* would need — another writer's import, sitting in
    /// their event file waiting to be merged, points at a file this document
    /// has never heard of. Events already applied are not consulted: a merge
    /// only ever replays what comes after this store's version vector, so an
    /// artifact from an event already folded in is reachable only if the
    /// document still reads it.
    ///
    /// Anything in the directory that is not an artifact is left alone.
    pub fn collect_unreferenced_artifacts(
        &self,
        journal: &EventJournal,
        data_directory: &Path,
    ) -> Result<ArtifactSweep, CoreError> {
        let mut referenced = HashSet::new();
        for artifact in self.document.artifacts() {
            referenced.insert(artifact.id.clone());
        }
        for entry in self.undo.iter().chain(self.redo.iter()) {
            for operation in entry.forward.iter().chain(entry.inverse.iter()) {
                collect_strings(
                    &serde_json::to_value(operation)
                        .map_err(|error| CoreError::Persistence(error.to_string()))?,
                    &mut referenced,
                );
            }
        }
        for event in journal.read_after(&self.version_vector)? {
            collect_strings(
                &serde_json::to_value(&event)
                    .map_err(|error| CoreError::Persistence(error.to_string()))?,
                &mut referenced,
            );
        }

        let mut sweep = ArtifactSweep::default();
        let Ok(entries) = fs::read_dir(data_directory) else {
            return Ok(sweep);
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(id) = artifact_file_id(&path) else {
                continue;
            };
            if referenced.contains(&id) {
                continue;
            }
            let size = entry.metadata().map(|data| data.len()).unwrap_or_default();
            if fs::remove_file(&path).is_ok() {
                sweep.files += 1;
                sweep.bytes += size;
            }
        }
        Ok(sweep)
    }

    /// Writes the frame's current values as data the document owns.
    ///
    /// The same parquet a snapshot would write, with one difference that is
    /// the whole point: the columns are named the way an import names them,
    /// so what comes back is an ordinary imported frame rather than a cache
    /// with a fingerprint. Nothing refreshes over it, which is what makes it
    /// editable.
    pub fn write_owned_frame_data(
        &self,
        frame_id: &str,
        data_directory: &Path,
    ) -> Result<DataArtifact, CoreError> {
        let frame = self.document.frame(frame_id)?;
        let source_name = frame.name.clone();
        let mut data_frame = self
            .document
            .materialize_frame_frame(frame_id, Layer::Data, &mut HashSet::new())
            .map_err(CoreError::Persistence)?;
        // The plan names its columns by id; an import's parquet names them
        // the way a person does, and that is the shape this is becoming.
        for column in &frame.columns {
            if data_frame.column(column.id.as_str()).is_ok() {
                data_frame
                    .rename(&column.id, column.name.as_str().into())
                    .map_err(|error| CoreError::Persistence(error.to_string()))?;
            }
        }
        write_frame_artifact(data_frame, data_directory, &source_name)
    }

    /// Computes the frame and writes the snapshot without recording
    /// anything, leaving the caller to apply
    /// [`Operation::SetFrameMaterialization`] however it records history —
    /// the desktop host journals it like any other edit.
    pub fn write_frame_snapshot(
        &self,
        frame_id: &str,
        data_directory: &Path,
    ) -> Result<DataArtifact, CoreError> {
        let frame = self.document.frame(frame_id)?;
        if frame.derivation.is_none() {
            return Err(CoreError::InvalidOperation(
                "Only a derived frame can be cached to a snapshot".into(),
            ));
        }
        let source_name = frame.name.clone();
        // Read live even when a snapshot already exists, or refreshing
        // would just copy the stale snapshot back over itself. This frame's
        // snapshot alone is set aside; the document is otherwise untouched,
        // so a formula somewhere below that reads this very frame still
        // finds the recorded value rather than a hole where it used to be.
        let frame = self
            .document
            .recompute_data_layer(frame_id, &mut HashSet::new())
            .and_then(|plan| {
                plan.collect()
                    .map_err(|error| format!("Polars error materializing frame: {error}"))
            })
            .map_err(CoreError::Persistence)?;
        write_frame_artifact(frame, data_directory, &source_name)
    }

    pub fn save_as(&mut self, path: &Path) -> Result<(), CoreError> {
        // The document takes the new file's name. Letting the two drift is
        // how you end up with "AccountingTest" living inside
        // `AccountingTest22.fw` and no way to tell two documents apart at a
        // glance -- which matters most in exactly the situation Save As
        // creates, where a second copy of the same document now exists.
        if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
            && !stem.trim().is_empty()
        {
            self.document.name = stem.to_string();
        }
        self.save_copy(path)
    }

    /// Copies the complete document package without changing its human name.
    ///
    /// This is deliberately distinct from Save As: an application-created
    /// working copy of a sample gets a UUID-suffixed private filename, but
    /// that storage detail must not rename the document the person chose.
    /// Imported data still has to travel with it, so a plain file copy is not
    /// sufficient.
    pub fn save_copy(&mut self, path: &Path) -> Result<(), CoreError> {
        let data_directory = CollaborationPaths::for_document(path, &self.document.id)?
            .root
            .join("data");
        let original_paths = self
            .document
            .artifacts()
            .map(|artifact| (artifact.id.clone(), artifact.path.clone()))
            .collect::<HashMap<_, _>>();
        // Every artifact a frame owns, snapshots included. A copy that
        // brought the imported parquet but left its caches pointing into the
        // original's sidecar would read the original's numbers, and go on
        // reading them until someone refreshed — or break outright the day
        // the original was deleted.
        let copied = self.document.artifacts_mut().try_for_each(|artifact| {
            fs::create_dir_all(&data_directory)
                .map_err(|error| CoreError::Persistence(error.to_string()))?;
            let destination = data_directory.join(format!("{}.parquet", artifact.id));
            if Path::new(&artifact.path) != destination && !destination.exists() {
                fs::copy(&artifact.path, &destination)
                    .map_err(|error| CoreError::Persistence(error.to_string()))?;
            }
            artifact.path = destination.display().to_string();
            Ok::<(), CoreError>(())
        });
        if let Err(error) = copied.and_then(|()| self.save(path)) {
            for artifact in self.document.artifacts_mut() {
                if let Some(original) = original_paths.get(&artifact.id) {
                    artifact.path.clone_from(original);
                }
            }
            return Err(error);
        }
        Ok(())
    }

    /// Write a frame's materialized values to `path` as CSV.
    ///
    /// Derived frames evaluate through the same recursive Polars plan the
    /// canvas uses, and values stay raw — ISO dates and plain numbers rather
    /// than display formatting.
    pub fn export_frame_csv(&self, frame_id: &str, path: &Path) -> Result<(), CoreError> {
        self.document.export_frame_csv(frame_id, path)
    }

    /// Write selected frames and every named scalar answer to an Excel
    /// workbook. This is a values-only handoff: FrameWork remains the place
    /// where formulas live, while the workbook receives their current answers.
    pub fn export_excel(&self, frame_ids: &[Id], path: &Path) -> Result<(), CoreError> {
        self.document.export_excel(frame_ids, path)
    }

    pub fn get_frame_page(
        &self,
        frame_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<FramePage, CoreError> {
        self.document
            .get_frame_page(frame_id, offset, limit, &self.sorted_page_cache)
    }

    pub fn get_block_line_page(
        &self,
        block_id: &str,
        line_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<BlockLinePage, CoreError> {
        self.document
            .block_line_page(block_id, line_id, offset, limit)
    }

    /// The columns the step at `step_index` can see, as (id, type) pairs.
    /// Resolved from the plan, so it costs no scan.
    pub fn get_step_schema(
        &self,
        frame_id: &str,
        step_index: usize,
    ) -> Result<Vec<(Id, DataType)>, CoreError> {
        self.document.schema_at_step(frame_id, step_index)
    }

    /// The Polars plan behind a frame as it is displayed, as text, for the
    /// query-plan panel. Building the plan does not run it.
    pub fn get_frame_query_plan(&self, frame_id: &str) -> Result<FrameQueryPlan, CoreError> {
        self.document.frame_query_plan(frame_id)
    }

    /// The schema at every position in a chain the editor is drafting.
    ///
    /// Read-only and unsaved by design: the editor asks what its draft would
    /// produce, gets back the columns each step leaves behind, and finds out
    /// which step it cannot get past. A chain that stops partway is an
    /// ordinary answer rather than an error — the step that stopped it is
    /// named, and the schemas before it still hold, which is what makes a
    /// broken step something you can see and delete rather than a wall.
    pub fn preview_frame_pipeline(
        &self,
        frame_id: &str,
        steps: Vec<FrameStepInput>,
    ) -> Result<PipelineSchema, CoreError> {
        let (walk, failure) = self.document.walk_pipeline(frame_id, steps, None)?;
        Ok(PipelineSchema {
            frame_id: frame_id.to_string(),
            input_columns: walk.input_columns,
            steps: walk
                .schemas
                .into_iter()
                .map(|columns| StepSchema { columns })
                .collect(),
            failed_step: failure.as_ref().map(|(index, _)| *index),
            error: failure.map(|(_, error)| error.to_string()),
        })
    }

    /// Number of full sort+filter computations the sorted-page cache has
    /// performed so far (i.e. cache misses). Exposed for tests to assert
    /// that repeated page fetches under an unchanged sort reuse the cached
    /// permutation instead of re-sorting.
    pub fn sorted_page_cache_computations(&self) -> usize {
        self.sorted_page_cache.computations()
    }

    /// Read-only, type-aware autocomplete for the formula editor. Never errors: an
    /// unparseable receiver just degrades to untyped suggestions.
    pub fn complete_formula(
        &self,
        frame_id: &str,
        formula_text: &str,
        cursor_pos: usize,
    ) -> CompletionResult {
        crate::formula::complete::complete_formula(
            &self.document,
            frame_id,
            formula_text,
            cursor_pos,
        )
    }

    /// The distinct values a conditional-formatting rule's formula produces,
    /// commonest first — how the Rules panel fills a case list from the data
    /// instead of making somebody type the values they already have.
    ///
    /// A read, not an operation: what comes back is a list of labels, and it
    /// is the panel that decides what they should look like and writes the
    /// rule. That split is deliberate — colors are chosen where every other
    /// color in this application is chosen, and the operation stays a plain
    /// record of the mapping somebody ended up with.
    pub fn frame_formula_values(
        &self,
        frame_id: &str,
        formula: &str,
        limit: usize,
    ) -> Result<Vec<String>, CoreError> {
        self.document
            .frame_formula_values(frame_id, formula, limit.clamp(1, 200))
    }

    /// The first `limit` rows as they stand after one step of a draft chain.
    ///
    /// This one runs the query, unlike the schema preview beside it — the
    /// limit is pushed into the plan, so it reads what it needs rather than
    /// the whole frame, but it is still work. One more row than asked for
    /// is fetched, which is how "there is more below this" gets answered
    /// without a count.
    pub fn sample_frame_step(
        &self,
        frame_id: &str,
        steps: Vec<FrameStepInput>,
        step_index: usize,
        limit: usize,
    ) -> Result<StepSample, CoreError> {
        let limit = limit.clamp(1, 500);
        let (walk, failure) = self
            .document
            .walk_pipeline(frame_id, steps, Some(step_index))?;
        if let Some((index, error)) = failure
            && index <= step_index
        {
            return Err(error);
        }
        let columns = match step_index.checked_sub(1) {
            _ if walk.schemas.is_empty() => walk.input_columns.clone(),
            _ => walk
                .schemas
                .get(step_index)
                .or_else(|| walk.schemas.last())
                .cloned()
                .unwrap_or_else(|| walk.input_columns.clone()),
        };
        let frame = walk
            .plan
            .limit(limit as u32 + 1)
            .collect()
            .map_err(|error| CoreError::Import(error.to_string()))?;
        let truncated = frame.height() > limit;
        let rows = (0..frame.height().min(limit))
            .map(|row| {
                columns
                    .iter()
                    .map(|column| {
                        frame
                            .column(&column.id)
                            .ok()
                            .and_then(|series| {
                                polars_value_at(series.as_materialized_series(), row).ok()
                            })
                            .map(scalar_value_to_raw)
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .collect();
        Ok(StepSample {
            frame_id: frame_id.to_string(),
            step_index,
            columns,
            rows,
            truncated,
        })
    }

    /// Completion for a formula at a position in a chain being drafted.
    ///
    /// A step sees what the steps before it leave behind — so after a
    /// summarize the source columns are gone and the aggregates are what
    /// exist. Completing against the frame's own columns instead is how the
    /// editor ends up suggesting names the formula cannot use.
    ///
    /// `step_index` counts every step, pass-through ones included, and is
    /// the position the formula is being written *at*: the scope is what
    /// the steps before it produce.
    pub fn complete_step_formula(
        &self,
        frame_id: &str,
        mut steps: Vec<FrameStepInput>,
        step_index: usize,
        formula_text: &str,
        cursor_pos: usize,
    ) -> CompletionResult {
        // Completion asks what the current formula can read, which is only
        // what the steps above it produce. Walking the current and later
        // steps is not merely wasted validation: a later pivot may inspect
        // the data to discover its schema, turning every cursor movement in
        // an unrelated formula into a scan of a live frame. Truncating the
        // draft also gives index zero its proper meaning -- start directly
        // from the input schema without applying any transformation.
        steps.truncate(step_index.min(steps.len()));
        let Ok((walk, _)) = self.document.walk_pipeline(frame_id, steps, None) else {
            return self.complete_formula(frame_id, formula_text, cursor_pos);
        };
        let columns = walk
            .schemas
            .last()
            .cloned()
            .unwrap_or_else(|| walk.input_columns.clone());
        let scope_id = walk
            .source_frame_id
            .clone()
            .unwrap_or_else(|| frame_id.to_string());
        let Ok(scope_frame) = self.document.frame(&scope_id) else {
            return self.complete_formula(frame_id, formula_text, cursor_pos);
        };
        let scope = FrameObject {
            columns,
            rows: Vec::new(),
            steps: Vec::new(),
            display: FrameDisplay::default(),
            base_columns: Vec::new(),
            derivation: None,
            summaries: Vec::new(),
            ..scope_frame.clone()
        };
        crate::formula::complete::complete_formula_in_scope(
            &self.document,
            &scope,
            &scope_id,
            formula_text,
            cursor_pos,
        )
    }

    pub fn view(&self) -> DocumentView {
        let document = self.document.materialized_for_view();
        DocumentView {
            computed_frames: document.compute_frames(),
            computed_results: document.compute_results(),
            computed_blocks: document.compute_blocks(),
            computed_texts: document.compute_texts(),
            document,
            formula_functions: formula_function_catalog(),
            can_undo: !self.undo.is_empty(),
            can_redo: !self.redo.is_empty(),
        }
    }

    pub fn version_vector(&self) -> &VersionVector {
        &self.version_vector
    }

    pub fn prepare_event(
        &self,
        writer_id: &str,
        operation: Operation,
    ) -> Result<OperationEvent, CoreError> {
        Uuid::parse_str(writer_id)
            .map_err(|_| CoreError::InvalidEvent("writer ID is not a valid UUID".into()))?;
        let sequence = self
            .version_vector
            .get(writer_id)
            .copied()
            .unwrap_or_default()
            .checked_add(1)
            .ok_or_else(|| CoreError::InvalidEvent("writer sequence overflowed".into()))?;
        let event = OperationEvent::new(
            self.document.id.clone(),
            writer_id.into(),
            sequence,
            self.version_vector.clone(),
            self.prepare_operation(operation)?,
        );
        // Publication is irreversible, so fully validate against a clone
        // before the writer creates the immutable event file. Apply to the
        // document directly: `Store::apply_event` also constructs the whole
        // UI view, and doing that here meant every local edit computed that
        // view once for validation and then immediately computed it again
        // for the real write. The event metadata above was minted locally;
        // the state transition is the part that needs proving here.
        let mut validation_document = self.document.clone();
        validation_document.apply_replicated(event.operation.clone())?;
        Ok(event)
    }

    pub fn apply(&mut self, operation: Operation) -> Result<DocumentView, CoreError> {
        let operation = self.prepare_operation(operation)?;
        self.apply_replicated(operation)
    }

    pub fn prepare_operation(
        &self,
        operation: Operation,
    ) -> Result<ReplicatedOperation, CoreError> {
        self.document.prepare_operation(operation)
    }

    pub fn apply_replicated(
        &mut self,
        operation: ReplicatedOperation,
    ) -> Result<DocumentView, CoreError> {
        self.apply_replicated_with_history(operation, true)
    }

    pub(crate) fn apply_replicated_with_history(
        &mut self,
        operation: ReplicatedOperation,
        record_undo: bool,
    ) -> Result<DocumentView, CoreError> {
        // Read the inverse first: it describes the state this is about to
        // overwrite, so there is exactly one moment it can be taken.
        let inverse = if record_undo {
            self.document.invert(&operation)?
        } else {
            Vec::new()
        };
        let before = self.document.clone();
        if let Err(error) = self.document.apply_replicated(operation.clone()) {
            self.document = before;
            return Err(error);
        }
        self.document.revision += 1;
        if record_undo {
            self.undo.push(HistoryEntry {
                forward: vec![operation],
                inverse,
            });
            // Oldest first out. Draining rather than truncating keeps the
            // most recent edits, which are the ones anybody wants back.
            if self.undo.len() > UNDO_DEPTH {
                self.undo.drain(..self.undo.len() - UNDO_DEPTH);
            }
        }
        // A remote edit no longer clears the undo stack — that was forced by
        // snapshot history, where restoring one could erase the remote edit
        // wholesale. An inverse touches only what its own edit touched.
        // Redo still goes: a new edit of any origin ends the branch that
        // redo would have replayed onto.
        self.redo.clear();
        Ok(self.view())
    }

    /// Applies operations that are themselves history, so they record none.
    ///
    /// The document is restored on the first failure: an undo that half
    /// happened would leave a state neither stack describes.
    fn apply_history(&mut self, operations: &[ReplicatedOperation]) -> Result<(), CoreError> {
        let before = self.document.clone();
        for operation in operations {
            if let Err(error) = self.document.apply_replicated(operation.clone()) {
                self.document = before;
                return Err(error);
            }
        }
        self.document.revision = before.revision.saturating_add(1);
        Ok(())
    }

    pub fn apply_event(&mut self, event: &OperationEvent) -> Result<DocumentView, CoreError> {
        self.apply_event_with_history(event, true)
    }

    pub(crate) fn apply_imported_event(
        &mut self,
        event: &OperationEvent,
    ) -> Result<DocumentView, CoreError> {
        self.apply_event_with_history(event, false)
    }

    pub(crate) fn apply_event_with_history(
        &mut self,
        event: &OperationEvent,
        record_undo: bool,
    ) -> Result<DocumentView, CoreError> {
        if event.format_version != OPERATION_EVENT_VERSION {
            return Err(CoreError::InvalidEvent(format!(
                "unsupported event version {}",
                event.format_version
            )));
        }
        if event.document_id != self.document.id {
            return Err(CoreError::InvalidEvent(
                "event belongs to a different document".into(),
            ));
        }
        Uuid::parse_str(&event.event_id.writer_id)
            .map_err(|_| CoreError::InvalidEvent("writer ID is not a valid UUID".into()))?;
        let applied_sequence = self
            .version_vector
            .get(&event.event_id.writer_id)
            .copied()
            .unwrap_or_default();
        if event.event_id.sequence <= applied_sequence {
            return Ok(self.view());
        }
        if event.event_id.sequence != applied_sequence + 1 {
            return Err(CoreError::InvalidEvent(format!(
                "writer sequence {} is not ready after {}",
                event.event_id.sequence, applied_sequence
            )));
        }
        if event.dependencies.iter().any(|(writer_id, sequence)| {
            self.version_vector
                .get(writer_id)
                .copied()
                .unwrap_or_default()
                < *sequence
        }) {
            return Err(CoreError::InvalidEvent(
                "event dependencies have not been applied".into(),
            ));
        }
        if event
            .dependencies
            .get(&event.event_id.writer_id)
            .copied()
            .unwrap_or_default()
            != applied_sequence
        {
            return Err(CoreError::InvalidEvent(
                "event does not follow its writer's causal sequence".into(),
            ));
        }
        let view = self.apply_replicated_with_history(event.operation.clone(), record_undo)?;
        self.version_vector
            .insert(event.event_id.writer_id.clone(), event.event_id.sequence);
        Ok(view)
    }

    /// Reverses the last edit by applying its inverse forward.
    ///
    /// An inverse that no longer applies — a remote writer deleted the
    /// frame it names, say — is dropped rather than retried. That entry is
    /// the only thing lost; the rest of the stack still describes edits
    /// that have nothing to do with it, which is the whole gain over
    /// snapshots, where one such conflict voided the lot.
    pub fn undo(&mut self) -> DocumentView {
        while let Some(entry) = self.undo.pop() {
            if self.apply_history(&entry.inverse).is_ok() {
                self.redo.push(entry);
                break;
            }
        }
        self.view()
    }

    pub fn redo(&mut self) -> DocumentView {
        while let Some(entry) = self.redo.pop() {
            if self.apply_history(&entry.forward).is_ok() {
                self.undo.push(entry);
                break;
            }
        }
        self.view()
    }
}

/// What a sweep of the data directory reclaimed.
#[derive(Debug, Clone, Copy, Default, serde::Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ArtifactSweep {
    pub files: usize,
    #[ts(as = "u32")]
    pub bytes: u64,
}

/// The artifact id a data file is named for, or `None` for anything that is
/// not one.
///
/// Artifact files are named by the SHA-256 of their contents, so the test is
/// the shape of the name: sixty-four hex characters and a `.parquet`
/// extension. A sweep that deleted by extension alone would take out whatever
/// else somebody had put in the directory.
fn artifact_file_id(path: &Path) -> Option<String> {
    if path.extension()?.to_str()? != "parquet" {
        return None;
    }
    let stem = path.file_stem()?.to_str()?;
    (stem.len() == 64 && stem.chars().all(|character| character.is_ascii_hexdigit()))
        .then(|| stem.to_string())
}

/// Every string anywhere in a serialized value.
///
/// Reachability is read this way rather than by matching on each operation
/// that might carry an artifact: there are forty of them, they carry whole
/// frames and whole documents, and a hand-written walk that misses one
/// deletes a file something still needs. Every id is a string somewhere in
/// the JSON, so collecting all strings finds them all, and over-collecting
/// only ever means keeping a file too long.
fn collect_strings(value: &serde_json::Value, into: &mut HashSet<String>) {
    match value {
        serde_json::Value::String(text) => {
            into.insert(text.clone());
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_strings(item, into);
            }
        }
        serde_json::Value::Object(fields) => {
            for field in fields.values() {
                collect_strings(field, into);
            }
        }
        _ => {}
    }
}
