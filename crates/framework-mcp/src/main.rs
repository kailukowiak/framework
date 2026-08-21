use framework_core::{
    DataObject, DataType, Document, DocumentView, EventJournal, ExistingFormulaInput, FrameObject,
    FramePage, FrameStepInput, Operation, RenderedDerivedExpression, RenderedFrameStep,
    ScalarValue, SortInput, Store, SummaryOperation, is_framework_document_path,
};
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::wrapper::{Json, Parameters},
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    env,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};
use uuid::Uuid;

mod operation_catalog;

const DEFAULT_DOCUMENT: &str = "framework.fw";
const FRAMEWORK_APP_IDENTIFIER: &str = "com.framework.canvas";
const MCP_ENABLED_NAME: &str = "mcp-enabled";

fn mcp_enabled_at(data_directory: &Path) -> bool {
    data_directory
        .join(FRAMEWORK_APP_IDENTIFIER)
        .join(MCP_ENABLED_NAME)
        .is_file()
}

#[cfg(not(test))]
fn ensure_mcp_enabled() -> Result<(), String> {
    let data_directory = dirs::data_dir()
        .ok_or_else(|| "This machine has no application-data directory".to_string())?;
    if mcp_enabled_at(&data_directory) {
        Ok(())
    } else {
        Err("FrameWork MCP access is disabled. Enable it in FrameWork Settings.".into())
    }
}

// Unit tests exercise the tool contract in isolated temporary documents; a
// preference belonging to the developer's actual machine must not decide
// whether that contract can be tested.
#[cfg(test)]
fn ensure_mcp_enabled() -> Result<(), String> {
    Ok(())
}

#[derive(Debug)]
struct Session {
    store: Store,
    path: PathBuf,
    journal: Option<EventJournal>,
    writer_id: String,
    snapshot_dirty: bool,
}

impl Session {
    fn apply(&mut self, operation: Operation) -> Result<DocumentView, String> {
        let view = if let Some(journal) = &self.journal {
            let event = self
                .store
                .prepare_event(&self.writer_id, operation)
                .map_err(|error| error.to_string())?;
            journal.append(&event).map_err(|error| error.to_string())?;
            self.store
                .apply_event(&event)
                .map_err(|error| error.to_string())?
        } else {
            self.store
                .apply(operation)
                .map_err(|error| error.to_string())?
        };
        self.persist()?;
        Ok(view)
    }

    fn rescan(&mut self) -> Result<(), String> {
        let Some(journal) = self.journal.clone() else {
            return Ok(());
        };
        let merge = journal
            .merge_into(&mut self.store)
            .map_err(|error| error.to_string())?;
        if merge.applied > 0 || self.snapshot_dirty {
            self.persist()?;
        }
        Ok(())
    }

    fn persist(&mut self) -> Result<(), String> {
        self.snapshot_dirty = true;
        self.store
            .save(&self.path)
            .map_err(|error| error.to_string())?;
        self.snapshot_dirty = false;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct FrameworkMcp {
    session: Arc<Mutex<Session>>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct DocumentSummary {
    id: String,
    name: String,
    revision: u64,
    document_path: String,
    can_undo: bool,
    can_redo: bool,
    objects: Vec<ObjectSummary>,
    /// Where the formula vocabulary lives now. The catalog used to ride
    /// along here — 287 entries on every inspect, which blew response
    /// caps and buried the objects the call was for.
    formula_reference: String,
}

#[derive(Debug, Serialize, PartialEq, schemars::JsonSchema)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
enum ApiScalarValue {
    Null,
    Number(f64),
    String(String),
    Boolean(bool),
    Date(String),
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ObjectSummary {
    id: String,
    name: String,
    kind: String,
    value: Option<String>,
    data_type: Option<String>,
    row_count: Option<usize>,
    columns: Vec<ColumnSummary>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ColumnSummary {
    id: String,
    name: String,
    data_type: String,
    categories: Vec<String>,
    formula: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct FrameSnapshot {
    id: String,
    name: String,
    revision: u64,
    total_row_count: usize,
    returned_row_count: usize,
    columns: Vec<ColumnSummary>,
    rows: Vec<RowSnapshot>,
    summaries: Vec<SummarySnapshot>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct RowSnapshot {
    id: String,
    index: usize,
    cells: Vec<CellSnapshot>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct CellSnapshot {
    column_id: String,
    column_name: String,
    raw: String,
    display: String,
    numeric_value: Option<f64>,
    typed_value: ApiScalarValue,
    formula: Option<String>,
    is_override: bool,
    error: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SummarySnapshot {
    id: String,
    label: String,
    column_id: String,
    column_name: String,
    operation: String,
    display: String,
    numeric_value: Option<f64>,
    typed_value: ApiScalarValue,
    error: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct MutationReceipt {
    revision: u64,
    message: String,
    affected_object_id: Option<String>,
    affected_column_id: Option<String>,
    affected_row_id: Option<String>,
    can_undo: bool,
    can_redo: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct GetFrameArgs {
    /// Frame name or stable frame ID.
    frame: String,
    /// Maximum rows to return. Defaults to 100 and is capped at 1000.
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct CompleteFormulaArgs {
    /// Frame name or stable frame ID the formula is written against.
    frame: String,
    /// The formula text typed so far.
    formula: String,
    /// Character offset of the cursor within `formula`.
    cursor_pos: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct CreateGeneratorFrameArgs {
    name: String,
    /// The rule the rows grow from, as formula text in scalar scope:
    /// `sequence(0, 16)`, `sequence(2026-01-01, 2026-02-01, 1d)`, or bounds
    /// that name a value — `sequence(`Anchor`.dt.month_start(), `Anchor` + 1)`
    /// — so editing the value regrows the frame. Stop is excluded, like
    /// Python's range.
    formula: String,
    /// What to call the generated column. Defaults to the frame's name.
    column_name: Option<String>,
    /// Optional canvas x coordinate.
    x: Option<f64>,
    /// Optional canvas y coordinate.
    y: Option<f64>,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SetFrameGeneratorArgs {
    /// Generated frame name or stable frame ID.
    frame: String,
    /// The replacement rule, same language as `create_generator_frame`.
    formula: String,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct AddEntryColumnArgs {
    /// Computed (derived or generated) frame name or stable frame ID.
    frame: String,
    /// Name for the new entry column.
    name: String,
    /// Data type: number, integer, string, boolean, date, currency, percentage.
    data_type: String,
    /// Names or IDs of the columns whose values identify a row. They are
    /// enforced unique as part of the add; data with duplicates under them
    /// refuses the column.
    key_columns: Vec<String>,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SetEntryValueArgs {
    /// Frame name or stable frame ID.
    frame: String,
    /// Entry column name or stable column ID.
    column: String,
    /// The row's key column raw values, in the entry column's key order —
    /// exactly as get_frame shows them (dates as 2026-09-16, integers bare).
    key: Vec<String>,
    /// The value to enter. Empty text removes the entry.
    raw: String,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ExpandFrameArgs {
    /// Frame whose rows are multiplied — the entry lines, the scenarios.
    frame: String,
    /// Frame whose rows multiply them — typically a generator of dates.
    against: String,
    /// Name for the new expanded frame.
    name: String,
    x: Option<f64>,
    y: Option<f64>,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SetCrosstabArgs {
    /// Frame name or stable frame ID.
    frame: String,
    /// Column whose values become the wide column headers (dates, usually).
    names_column: Option<String>,
    /// Column that fills the cells (an entry column, for an editable grid).
    values_column: Option<String>,
    /// True folds the view back to ordinary rows.
    off: Option<bool>,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SetUniqueKeyArgs {
    /// Frame name or stable frame ID.
    frame: String,
    /// Names or IDs of the columns that together identify a row.
    columns: Vec<String>,
    /// Omit or true to declare the key; false clears a key over exactly
    /// these columns.
    enabled: Option<bool>,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct RefreshFramePipelineArgs {
    /// Frame name or stable frame ID whose chain should re-discover its
    /// data-baked outputs (pivot columns especially).
    frame: String,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SearchFunctionsArgs {
    /// Words to look for — matched case-insensitively against each
    /// function's name, aliases, category, and description. Aliases carry
    /// the vocabulary people arrive with ("range", "vlookup", "Excel
    /// SEQUENCE"), so search with the word you would have used in Excel or
    /// Polars, not only the word you guess FrameWork uses. Omit to list the
    /// whole catalog.
    query: Option<String>,
    /// Maximum entries to return. Defaults to 25 when searching and to the
    /// full catalog when listing.
    limit: Option<usize>,
}

/// One formula function as the searchable catalog reports it. This is the
/// same catalog completion reads, so what search finds is exactly what the
/// compiler accepts — there is no MCP-only vocabulary to fall out of date.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct FunctionEntry {
    /// How the function is written: `sequence(...)` for a root call,
    /// `.cast` for a method on a value.
    name: String,
    signature: String,
    description: String,
    category: String,
    /// Other names people know this function by, including Excel names.
    aliases: Vec<String>,
    /// Per-argument guidance, in call order.
    arguments: Vec<FunctionArgumentEntry>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct FunctionArgumentEntry {
    name: String,
    required: bool,
    description: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct FunctionSearchResult {
    /// Entries in relevance order: name and alias hits before category and
    /// description hits.
    functions: Vec<FunctionEntry>,
    /// How many entries matched before `limit` was applied.
    total_matches: usize,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct OperationCatalog {
    format: String,
    instructions: String,
    type_script: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ApplyOperationArgs {
    /// A serialized public Operation object. Call describe_operations for
    /// the complete discriminated union and every nested input type.
    // A bare `serde_json::Value` publishes a schema with no `type` at all,
    // and an MCP client reading that schema is entitled to — and in
    // practice does — send the payload as a JSON *string*. That one
    // omission silently killed every apply_operation call from a real
    // client while the Rust tests, which hand the value over directly,
    // stayed green. So the schema says `object` explicitly, and
    // `operation_value` below is the tolerance for clients that
    // stringify anyway.
    #[schemars(schema_with = "operation_object_schema")]
    operation: serde_json::Value,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

/// The `operation` parameter's schema: an object, deliberately open —
/// its real shape is the Operation union that describe_operations serves,
/// which JSON Schema cannot restate without a second copy drifting.
fn operation_object_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "object",
        "additionalProperties": true
    })
}

/// The operation as JSON, unwrapping one layer of accidental
/// stringification: a client that sends `"{\"type\":...}"` meant the
/// object inside it, and refusing the whole call over quoting teaches
/// nobody anything. Only a bare string is unwrapped — an object passes
/// through untouched — so this stays a compatibility ramp, not a second
/// wire format.
fn operation_value(operation: serde_json::Value) -> serde_json::Value {
    match operation {
        serde_json::Value::String(text) => {
            serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text))
        }
        other => other,
    }
}

/// Mirrors `framework_core::Suggestion` with a JSON schema for MCP clients.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct CompletionSuggestion {
    id: String,
    label: String,
    insert_text: String,
    kind: String,
    detail: String,
    score: i64,
}

impl From<framework_core::Suggestion> for CompletionSuggestion {
    fn from(suggestion: framework_core::Suggestion) -> Self {
        Self {
            id: suggestion.id,
            label: suggestion.label,
            insert_text: suggestion.insert_text,
            kind: format!("{:?}", suggestion.kind),
            detail: suggestion.detail,
            score: suggestion.score,
        }
    }
}

/// Mirrors `framework_core::CompletionResult` with a JSON schema for MCP clients.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct CompletionSnapshot {
    receiver_dtype: Option<String>,
    namespace: Option<String>,
    suggestions: Vec<CompletionSuggestion>,
    note: Option<String>,
}

impl From<framework_core::CompletionResult> for CompletionSnapshot {
    fn from(result: framework_core::CompletionResult) -> Self {
        Self {
            receiver_dtype: result.receiver_dtype,
            namespace: result.namespace,
            suggestions: result.suggestions.into_iter().map(Into::into).collect(),
            note: result.note,
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct CreateBlockArgs {
    name: String,
    /// Optional canvas x coordinate.
    x: Option<f64>,
    /// Optional canvas y coordinate.
    y: Option<f64>,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SetBlockSourceArgs {
    /// Block name or ID.
    block: String,
    /// The whole text of the block, replacing what was there. One expression
    /// per line; `name = expression` names a line, and a named line can be
    /// read from anywhere as `` `Block`.name ``. A line that does not parse
    /// is kept and reports its own complaint rather than failing the write.
    source: String,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct CreateFrameArgs {
    name: String,
    /// Rectangular string grid. The first row contains column names.
    grid: Vec<Vec<String>>,
    /// Optional canvas x coordinate.
    x: Option<f64>,
    /// Optional canvas y coordinate.
    y: Option<f64>,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SetValueArgs {
    /// Value-object name or stable object ID.
    value: String,
    raw: String,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct DeleteObjectArgs {
    /// Object name or stable object ID.
    object: String,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SetCellArgs {
    /// Frame name or stable frame ID.
    frame: String,
    /// Stable row ID or a 1-based row number, such as "2".
    row: String,
    /// Column name or stable column ID.
    column: String,
    raw: String,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct AddRowArgs {
    /// Frame name or stable frame ID.
    frame: String,
    /// Cell values keyed by column name or stable column ID. Omitted columns are blank.
    values: BTreeMap<String, String>,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct DeleteRowArgs {
    /// Frame name or stable frame ID.
    frame: String,
    /// Stable row ID or a 1-based row number.
    row: String,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct AddLiteralColumnArgs {
    /// Frame name or stable frame ID.
    frame: String,
    name: String,
    /// One of: string/text, categorical/enum, integer/int, number, currency, percentage/percent, boolean/bool, or date.
    data_type: String,
    /// Optional column name or ID after which the new column is inserted. Defaults to the end.
    after_column: Option<String>,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct DeleteColumnArgs {
    /// Frame name or stable frame ID.
    frame: String,
    /// Column name or stable column ID.
    column: String,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SetColumnTypeArgs {
    /// Frame name or stable frame ID.
    frame: String,
    /// Column name or stable column ID.
    column: String,
    /// One of: string/text, categorical/enum, integer/int, number, currency, percentage/percent, boolean/bool, or date.
    data_type: String,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SetColumnCategoriesArgs {
    /// Frame name or stable frame ID.
    frame: String,
    /// Column name or stable column ID.
    column: String,
    /// Ordered allowed values. Existing non-blank cells must remain represented.
    categories: Vec<String>,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct AddCalculatedColumnArgs {
    /// Frame name or stable frame ID.
    frame: String,
    name: String,
    /// Formula using exact semantic names, for example "`Quantity` * `Unit price` * `Safety Factor`".
    formula: String,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SetCellOverrideArgs {
    /// Frame name or stable frame ID.
    frame: String,
    /// Stable row ID or a 1-based row number.
    row: String,
    /// Column name or stable column ID.
    column: String,
    /// Formula for this cell. Use null to remove its override.
    formula: Option<String>,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct AddSummaryArgs {
    /// Frame name or stable frame ID.
    frame: String,
    /// Column name or stable column ID.
    column: String,
    /// One of: sum, mean, 25%, median, 75%, min, max, count, missing,
    /// distinct, or mode.
    operation: String,
    /// Reject the write if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct HistoryArgs {
    /// Reject the change if the document is no longer at this revision.
    expected_revision: Option<u64>,
}

impl FrameworkMcp {
    fn open(path: PathBuf) -> Result<Self, String> {
        let mut store = if path.exists() {
            Store::load(&path).map_err(|error| error.to_string())?
        } else {
            Store::new(Document::demo())
        };
        let journal = if is_framework_document_path(&path) {
            let journal = EventJournal::open(&path, &store.view().document.id)
                .map_err(|error| error.to_string())?;
            let merge = journal
                .merge_into(&mut store)
                .map_err(|error| error.to_string())?;
            if merge.applied > 0 {
                store.save(&path).map_err(|error| error.to_string())?;
            }
            Some(journal)
        } else {
            None
        };
        Ok(Self {
            session: Arc::new(Mutex::new(Session {
                store,
                path,
                journal,
                writer_id: Uuid::new_v4().to_string(),
                snapshot_dirty: false,
            })),
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, Session>, String> {
        ensure_mcp_enabled()?;
        let mut session = self
            .session
            .lock()
            .map_err(|_| "The FrameWork document session is unavailable".to_string())?;
        session.rescan()?;
        Ok(session)
    }

    fn mutate(
        &self,
        operation: Operation,
        expected_revision: Option<u64>,
        message: String,
        affected_object_id: Option<String>,
        affected_column_id: Option<String>,
        affected_row_id: Option<String>,
    ) -> Result<Json<MutationReceipt>, String> {
        let mut session = self.lock()?;
        check_revision(&session.store.view(), expected_revision)?;
        let view = session.apply(operation)?;
        Ok(Json(receipt(
            &view,
            message,
            affected_object_id,
            affected_column_id,
            affected_row_id,
        )))
    }
}

#[tool_router]
impl FrameworkMcp {
    /// Inspect the document and discover stable object, frame, and column IDs before editing.
    #[tool(
        name = "inspect_document",
        annotations(title = "Inspect FrameWork document", read_only_hint = true)
    )]
    fn inspect_document(&self) -> Result<Json<DocumentSummary>, String> {
        let session = self.lock()?;
        Ok(Json(document_summary(&session.store.view(), &session.path)))
    }

    /// Read a frame with stable row/column IDs, raw values, formulas, computed results, and errors.
    #[tool(
        name = "get_frame",
        annotations(title = "Read a FrameWork frame", read_only_hint = true)
    )]
    fn get_frame(
        &self,
        Parameters(args): Parameters<GetFrameArgs>,
    ) -> Result<Json<FrameSnapshot>, String> {
        let session = self.lock()?;
        let view = session.store.view();
        let frame_id = resolve_frame_id(&view, &args.frame)?;
        let limit = args.limit.unwrap_or(100).clamp(1, 1000);
        let frame = frame_by_id(&view, &frame_id)?;
        // Any frame that does not hold its rows literally answers through
        // the paged read: derived and generated frames, chained frames —
        // and plain file imports, whose omission here made a perfectly
        // good import look like zero rows. An agent seeing that "empty"
        // frame deleted it and hand-transcribed the CSV, which is exactly
        // the silent wrongness this snapshot exists to prevent.
        let page = (!frame.owns_its_rows() || !frame.steps.is_empty())
            .then(|| session.store.get_frame_page(&frame_id, 0, limit))
            .transpose()
            .map_err(|error| error.to_string())?;
        Ok(Json(frame_snapshot(
            &view,
            &frame_id,
            limit,
            page.as_ref(),
        )?))
    }

    /// Type-aware formula autocomplete: suggests columns, root functions, and
    /// dtype-filtered Polars methods for the cursor position in a formula.
    #[tool(
        name = "complete_formula",
        annotations(title = "Complete a FrameWork formula", read_only_hint = true)
    )]
    fn complete_formula(
        &self,
        Parameters(args): Parameters<CompleteFormulaArgs>,
    ) -> Result<Json<CompletionSnapshot>, String> {
        let session = self.lock()?;
        let view = session.store.view();
        let frame_id = resolve_frame_id(&view, &args.frame)?;
        Ok(Json(
            session
                .store
                .complete_formula(&frame_id, &args.formula, args.cursor_pos)
                .into(),
        ))
    }

    /// Search the formula function catalog by name, alias, category, or
    /// description. Aliases include the Excel and Polars names people arrive
    /// with, so "range" finds sequence() and "vlookup" finds the join
    /// vocabulary. Search here before concluding a capability is missing.
    #[tool(
        name = "search_functions",
        annotations(title = "Search FrameWork formula functions", read_only_hint = true)
    )]
    fn search_functions(
        &self,
        Parameters(args): Parameters<SearchFunctionsArgs>,
    ) -> Result<Json<FunctionSearchResult>, String> {
        // Producing the catalog needs no document, but this is still an MCP
        // request and obeys the machine-local access switch.
        drop(self.lock()?);
        let catalog = framework_core::formula_function_catalog();
        let query = args
            .query
            .as_deref()
            .map(str::trim)
            .filter(|query| !query.is_empty())
            .map(str::to_lowercase);
        // Name and alias hits come first: someone searching "range" wants
        // sequence() at the top, not every description that mentions ranges.
        let mut ranked: Vec<(u8, &framework_core::FormulaFunction)> = catalog
            .iter()
            .filter_map(|function| match &query {
                None => Some((0, function)),
                Some(query) => {
                    let words = || query.split_whitespace();
                    let name_hit = words().any(|word| {
                        function.name.to_lowercase().contains(word)
                            || function
                                .aliases
                                .iter()
                                .any(|alias| alias.to_lowercase().contains(word))
                    });
                    let text_hit = words().any(|word| {
                        function.category.to_lowercase().contains(word)
                            || function.description.to_lowercase().contains(word)
                            || function.signature.to_lowercase().contains(word)
                    });
                    match (name_hit, text_hit) {
                        (true, _) => Some((0, function)),
                        (false, true) => Some((1, function)),
                        (false, false) => None,
                    }
                }
            })
            .collect();
        ranked.sort_by_key(|(rank, _)| *rank);
        let total_matches = ranked.len();
        let limit = args
            .limit
            .unwrap_or(if query.is_some() { 25 } else { usize::MAX });
        let functions = ranked
            .into_iter()
            .take(limit)
            .map(|(_, function)| FunctionEntry {
                name: function.name.clone(),
                signature: function.signature.clone(),
                description: function.description.clone(),
                category: function.category.clone(),
                aliases: function.aliases.clone(),
                arguments: function
                    .arguments
                    .iter()
                    .map(|argument| FunctionArgumentEntry {
                        name: argument.name.clone(),
                        required: argument.required,
                        description: argument.description.clone(),
                    })
                    .collect(),
            })
            .collect();
        Ok(Json(FunctionSearchResult {
            functions,
            total_matches,
        }))
    }

    /// Describe every mutation accepted by the canonical Rust operation API.
    /// The answer is generated from the same enum that generates the desktop's
    /// TypeScript binding, including nested pipeline, style and artifact inputs.
    #[tool(
        name = "describe_operations",
        annotations(title = "Describe all FrameWork operations", read_only_hint = true)
    )]
    fn describe_operations(&self) -> Result<Json<OperationCatalog>, String> {
        // This is still an MCP request and obeys the machine-local access
        // switch even though producing the catalog itself needs no document.
        drop(self.lock()?);
        Ok(Json(OperationCatalog {
            format: "TypeScript discriminated union describing JSON".into(),
            instructions: "Pass one Operation object as operation to apply_operation. Use stable IDs from inspect_document and include expectedRevision from the latest read. Prefer the named task-level tools when one already matches the job.".into(),
            type_script: operation_catalog::operation_typescript(),
        }))
    }

    /// Apply any public operation supported by the desktop. This is the
    /// complete escape hatch for capabilities that do not yet have a friendly
    /// task-level MCP tool; validation, history, persistence and collaboration
    /// remain exactly the same as a desktop edit.
    #[tool(
        name = "apply_operation",
        annotations(title = "Apply any FrameWork operation", read_only_hint = false)
    )]
    fn apply_operation(
        &self,
        Parameters(args): Parameters<ApplyOperationArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let payload = operation_value(args.operation);
        let operation_kind = payload
            .get("type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("operation")
            .to_string();
        let operation: Operation = serde_json::from_value(payload)
            .map_err(|error| format!("Invalid FrameWork operation: {error}"))?;
        match operation {
            // These two spellings predate the Wrangle chain. Keep accepting
            // their serialized contract, but do not let the generated MCP
            // escape hatch recreate a hidden column-formula authoring path.
            Operation::AddComputedColumn {
                frame_id,
                name,
                formula,
                ..
            } => {
                return self.add_calculated_column(Parameters(AddCalculatedColumnArgs {
                    frame: frame_id,
                    name,
                    formula,
                    expected_revision: args.expected_revision,
                }));
            }
            Operation::SetColumnFormula {
                frame_id,
                column_id,
                formula,
            } => {
                let mut session = self.lock()?;
                let before = session.store.view();
                check_revision(&before, args.expected_revision)?;
                let frame = frame_by_id(&before, &frame_id)?;
                let column = frame
                    .columns
                    .iter()
                    .find(|column| column.id == column_id)
                    .ok_or_else(|| format!("Column '{column_id}' was not found"))?;
                let computed = before.computed_frames.get(&frame_id).ok_or_else(|| {
                    format!("Computed data for frame '{}' is unavailable", frame.name)
                })?;
                let mut steps = rendered_pipeline_inputs(frame, &computed.steps)?;
                steps.push(FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: column.id.clone(),
                        name: column.name.clone(),
                        formula,
                    }],
                });
                let view = session.apply(Operation::SetFramePipeline {
                    frame_id: frame_id.clone(),
                    steps,
                })?;
                return Ok(Json(receipt(
                    &view,
                    format!("Applied {operation_kind} through Wrangle"),
                    Some(frame_id),
                    Some(column_id),
                    None,
                )));
            }
            _ => {}
        }
        self.mutate(
            operation,
            args.expected_revision,
            format!("Applied {operation_kind}"),
            None,
            None,
            None,
        )
    }

    /// Create an empty formula block on the canvas and return its stable object ID.
    ///
    /// A block is where scalars live: constants and computed values are both
    /// written as lines of one, rather than as separate cards on the canvas.
    #[tool(
        name = "create_block",
        annotations(title = "Create a FrameWork formula block", read_only_hint = false)
    )]
    fn create_block(
        &self,
        Parameters(args): Parameters<CreateBlockArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let mut session = self.lock()?;
        check_revision(&session.store.view(), args.expected_revision)?;
        let before = object_ids(&session.store.view());
        let view = session.apply(Operation::AddBlock {
            name: args.name.clone(),
            x: args.x.unwrap_or(80.0),
            y: args.y.unwrap_or(80.0),
        })?;
        let object_id = new_object_id(&view, &before)?;
        Ok(Json(receipt(
            &view,
            format!("Created block '{}'", args.name),
            Some(object_id),
            None,
            None,
        )))
    }

    /// Replace the whole text of a formula block.
    #[tool(
        name = "set_block_source",
        annotations(title = "Write a FrameWork formula block", read_only_hint = false)
    )]
    fn set_block_source(
        &self,
        Parameters(args): Parameters<SetBlockSourceArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let view = self.lock()?.store.view();
        let block_id = resolve_block_id(&view, &args.block)?;
        self.mutate(
            Operation::SetBlockSource {
                block_id: block_id.clone(),
                source: args.source,
                editing: None,
            },
            args.expected_revision,
            format!("Wrote block '{}'", args.block),
            Some(block_id),
            None,
            None,
        )
    }

    /// Create a frame from a string grid whose first row contains the column names.
    #[tool(
        name = "create_frame",
        annotations(title = "Create a FrameWork frame", read_only_hint = false)
    )]
    fn create_frame(
        &self,
        Parameters(args): Parameters<CreateFrameArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        if args.grid.is_empty() {
            return Err("grid must contain at least a header row".into());
        }
        let mut session = self.lock()?;
        check_revision(&session.store.view(), args.expected_revision)?;
        let before = object_ids(&session.store.view());
        let view = session.apply(Operation::AddFrame {
            name: args.name.clone(),
            grid: args.grid,
            x: args.x.unwrap_or(360.0),
            y: args.y.unwrap_or(80.0),
        })?;
        let object_id = new_object_id(&view, &before)?;
        Ok(Json(receipt(
            &view,
            format!("Created frame '{}'", args.name),
            Some(object_id),
            None,
            None,
        )))
    }

    /// Create a frame whose rows are grown from a rule — `sequence(0, 16)`,
    /// a date range, or bounds naming a value so the frame follows it. Use
    /// this instead of writing a helper CSV of offsets or dates; Expand
    /// steps multiply against it for calendar and scenario cross products.
    #[tool(
        name = "create_generator_frame",
        annotations(title = "Create a generated FrameWork frame", read_only_hint = false)
    )]
    fn create_generator_frame(
        &self,
        Parameters(args): Parameters<CreateGeneratorFrameArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let mut session = self.lock()?;
        check_revision(&session.store.view(), args.expected_revision)?;
        let before = object_ids(&session.store.view());
        let view = session.apply(Operation::AddGeneratorFrame {
            name: args.name.clone(),
            formula: args.formula,
            column_name: args.column_name,
            x: args.x.unwrap_or(360.0),
            y: args.y.unwrap_or(80.0),
        })?;
        let object_id = new_object_id(&view, &before)?;
        Ok(Json(receipt(
            &view,
            format!("Created generated frame '{}'", args.name),
            Some(object_id),
            None,
            None,
        )))
    }

    /// Replace a generated frame's rule. The column's type follows the new
    /// rule, and every frame expanding against this one regrows.
    #[tool(
        name = "set_frame_generator",
        annotations(title = "Replace a generated frame's rule", read_only_hint = false)
    )]
    fn set_frame_generator(
        &self,
        Parameters(args): Parameters<SetFrameGeneratorArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let view = self.lock()?.store.view();
        let frame_id = resolve_frame_id(&view, &args.frame)?;
        self.mutate(
            Operation::SetFrameGenerator {
                frame_id: frame_id.clone(),
                formula: args.formula,
            },
            args.expected_revision,
            format!("Replaced the rule of '{}'", args.frame),
            Some(frame_id),
            None,
            None,
        )
    }

    /// Re-save a frame's chain as it stands so pivot columns and other
    /// data-baked outputs follow the data as it is now. Surviving outputs
    /// keep their column ids. Use after changing a value that drives a
    /// pivoted date range or category set.
    #[tool(
        name = "refresh_frame_pipeline",
        annotations(title = "Refresh a frame's baked outputs", read_only_hint = false)
    )]
    fn refresh_frame_pipeline(
        &self,
        Parameters(args): Parameters<RefreshFramePipelineArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let view = self.lock()?.store.view();
        let frame_id = resolve_frame_id(&view, &args.frame)?;
        self.mutate(
            Operation::RefreshFramePipeline {
                frame_id: frame_id.clone(),
            },
            args.expected_revision,
            format!("Refreshed the chain of '{}'", args.frame),
            Some(frame_id),
            None,
            None,
        )
    }

    /// Pair every row of one frame with every row of another — the
    /// table-shaped `for each` behind calendars and scenario grids. Makes
    /// a new frame derived from `frame`, expanded against `against`
    /// (typically a generator frame of dates). This is the step that turns
    /// entry lines × period dates into a timesheet skeleton.
    #[tool(
        name = "expand_frame",
        annotations(title = "Cross-multiply two frames", read_only_hint = false)
    )]
    fn expand_frame(
        &self,
        Parameters(args): Parameters<ExpandFrameArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let view = self.lock()?.store.view();
        let source_id = resolve_frame_id(&view, &args.frame)?;
        let against_id = resolve_frame_id(&view, &args.against)?;
        self.mutate(
            Operation::AddLinkedFrame {
                source_frame_id: source_id,
                name: args.name.clone(),
                x: args.x.unwrap_or(0.0),
                y: args.y.unwrap_or(0.0),
            },
            args.expected_revision,
            format!("Added '{}'", args.name),
            None,
            None,
            None,
        )?;
        let view = self.lock()?.store.view();
        let frame_id = resolve_frame_id(&view, &args.name)?;
        let existing = frame_by_id(&view, &frame_id)?;
        let mut steps: Vec<FrameStepInput> = view
            .computed_frames
            .get(&frame_id)
            .map(|computed| pass_through_inputs(&computed.steps, existing))
            .unwrap_or_default();
        steps.push(FrameStepInput::Expand {
            frame_id: against_id,
        });
        let expanded = self.mutate(
            Operation::SetFramePipeline { frame_id, steps },
            None,
            format!("Expanded '{}' against '{}'", args.frame, args.against),
            None,
            None,
            None,
        );
        if expanded.is_err() {
            // The linked frame without its expansion is half a gesture;
            // take it back out rather than leaving it to puzzle over.
            let view = self.lock()?.store.view();
            if let Ok(orphan) = resolve_frame_id(&view, &args.name) {
                let _ = self.mutate(
                    Operation::DeleteObject { object_id: orphan },
                    None,
                    "Removed the half-made expansion".into(),
                    None,
                    None,
                    None,
                );
            }
        }
        expanded
    }

    /// Show a long frame wide: one column per value of `names_column`,
    /// cells from `values_column`, everything else grouping the rows.
    /// Display only — the data stays long, and downstream frames see no
    /// schema change. Pair with an entry column for an editable grid.
    /// Pass off=true to fold back to rows.
    #[tool(
        name = "set_crosstab",
        annotations(title = "Show a frame as a crosstab", read_only_hint = false)
    )]
    fn set_crosstab(
        &self,
        Parameters(args): Parameters<SetCrosstabArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let view = self.lock()?.store.view();
        let frame_id = resolve_frame_id(&view, &args.frame)?;
        let crosstab = if args.off.unwrap_or(false) {
            None
        } else {
            let frame = frame_by_id(&view, &frame_id)?;
            let names = args
                .names_column
                .as_deref()
                .ok_or("set_crosstab needs namesColumn (or off: true)")?;
            let values = args
                .values_column
                .as_deref()
                .ok_or("set_crosstab needs valuesColumn (or off: true)")?;
            Some(framework_core::CrosstabDisplay {
                names_column_id: resolve_column_id(frame, names)?,
                values_column_id: resolve_column_id(frame, values)?,
            })
        };
        self.mutate(
            Operation::SetFrameDisplayCrosstab {
                frame_id: frame_id.clone(),
                crosstab,
            },
            args.expected_revision,
            "Set the crosstab view".into(),
            Some(frame_id),
            None,
            None,
        )
    }

    /// Declare (or clear) a unique key over a frame's columns. A unique
    /// key is what joins validate against and what entry columns address
    /// rows by; add_entry_column mints its own, so reach for this tool
    /// when a join needs the lookup side keyed.
    #[tool(
        name = "set_unique_key",
        annotations(title = "Set a frame's unique key", read_only_hint = false)
    )]
    fn set_unique_key(
        &self,
        Parameters(args): Parameters<SetUniqueKeyArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let view = self.lock()?.store.view();
        let frame_id = resolve_frame_id(&view, &args.frame)?;
        let frame = frame_by_id(&view, &frame_id)?;
        let column_ids = args
            .columns
            .iter()
            .map(|reference| resolve_column_id(frame, reference))
            .collect::<Result<Vec<_>, _>>()?;
        self.mutate(
            Operation::SetUniqueKey {
                frame_id: frame_id.clone(),
                column_ids,
                enabled: args.enabled.unwrap_or(true),
            },
            args.expected_revision,
            "Set the unique key".into(),
            Some(frame_id),
            None,
            None,
        )
    }

    /// Add a hand-entered column to a computed frame, keyed by row identity
    /// so entries survive the frame being regenerated. This is how a
    /// generated skeleton takes human input — hours against a timesheet
    /// line, a note against a scenario — without freezing a copy.
    #[tool(
        name = "add_entry_column",
        annotations(title = "Add a keyed entry column", read_only_hint = false)
    )]
    fn add_entry_column(
        &self,
        Parameters(args): Parameters<AddEntryColumnArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let view = self.lock()?.store.view();
        let frame_id = resolve_frame_id(&view, &args.frame)?;
        let frame = frame_by_id(&view, &frame_id)?;
        let data_type = parse_data_type(&args.data_type)?;
        let key_column_ids = args
            .key_columns
            .iter()
            .map(|reference| resolve_column_id(frame, reference))
            .collect::<Result<Vec<_>, _>>()?;
        self.mutate(
            Operation::AddEntryColumn {
                frame_id: frame_id.clone(),
                name: args.name.clone(),
                data_type,
                key_column_ids,
            },
            args.expected_revision,
            format!("Added entry column '{}'", args.name),
            Some(frame_id),
            None,
            None,
        )
    }

    /// Enter one value into an entry column, addressed by its row's key
    /// values rather than a row position — read the key raws from
    /// get_frame. Empty raw removes the entry.
    #[tool(
        name = "set_entry_value",
        annotations(title = "Enter a keyed value", read_only_hint = false)
    )]
    fn set_entry_value(
        &self,
        Parameters(args): Parameters<SetEntryValueArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let view = self.lock()?.store.view();
        let frame_id = resolve_frame_id(&view, &args.frame)?;
        let frame = frame_by_id(&view, &frame_id)?;
        let column_id = resolve_column_id(frame, &args.column)?;
        self.mutate(
            Operation::SetEntryValue {
                frame_id: frame_id.clone(),
                column_id: column_id.clone(),
                key: args.key,
                raw: args.raw,
            },
            args.expected_revision,
            "Entered a keyed value".into(),
            Some(frame_id),
            Some(column_id),
            None,
        )
    }

    /// Update a scalar value by name or ID.
    #[tool(
        name = "set_value",
        annotations(title = "Update a FrameWork value", read_only_hint = false)
    )]
    fn set_value(
        &self,
        Parameters(args): Parameters<SetValueArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let view = self.lock()?.store.view();
        // A named scalar lives either as a value object or as a named line
        // of a formula block, and the person updating "Timesheet date"
        // should not have to know which. The block spelling used to be
        // refused here, which meant the friendly tool could not touch the
        // very parameter the friendly create path produces.
        if let Ok(object_id) = resolve_value_id(&view, &args.value) {
            return self.mutate(
                Operation::SetValue {
                    object_id: object_id.clone(),
                    raw: args.raw,
                },
                args.expected_revision,
                format!("Updated value '{}'", args.value),
                Some(object_id),
                None,
                None,
            );
        }
        let (block_id, source) = block_line_rewrite(&view, &args.value, &args.raw)?;
        self.mutate(
            Operation::SetBlockSource {
                block_id: block_id.clone(),
                source,
                editing: None,
            },
            args.expected_revision,
            format!("Updated value '{}'", args.value),
            Some(block_id),
            None,
            None,
        )
    }

    /// Delete a canvas value, frame, or text object. Referenced values are protected.
    #[tool(
        name = "delete_object",
        annotations(title = "Delete a FrameWork object", read_only_hint = false)
    )]
    fn delete_object(
        &self,
        Parameters(args): Parameters<DeleteObjectArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let view = self.lock()?.store.view();
        let object_id = resolve_any_object_id(&view, &args.object)?;
        self.mutate(
            Operation::DeleteObject {
                object_id: object_id.clone(),
            },
            args.expected_revision,
            format!("Deleted object '{}'", args.object),
            Some(object_id),
            None,
            None,
        )
    }

    /// Update one literal frame cell using semantic frame/row/column references.
    #[tool(
        name = "set_cell",
        annotations(title = "Update a FrameWork cell", read_only_hint = false)
    )]
    fn set_cell(
        &self,
        Parameters(args): Parameters<SetCellArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let view = self.lock()?.store.view();
        let frame_id = resolve_frame_id(&view, &args.frame)?;
        let frame = frame_by_id(&view, &frame_id)?;
        let row_id = resolve_row_id(frame, &args.row)?;
        let column_id = resolve_column_id(frame, &args.column)?;
        self.mutate(
            Operation::SetCell {
                frame_id: frame_id.clone(),
                row_id: row_id.clone(),
                column_id: column_id.clone(),
                raw: args.raw,
            },
            args.expected_revision,
            format!("Updated row {} column '{}'", args.row, args.column),
            Some(frame_id),
            Some(column_id),
            Some(row_id),
        )
    }

    /// Append a literal row. Values are keyed by human column names or stable column IDs.
    #[tool(
        name = "add_row",
        annotations(title = "Add a FrameWork row", read_only_hint = false)
    )]
    fn add_row(
        &self,
        Parameters(args): Parameters<AddRowArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let mut session = self.lock()?;
        let before = session.store.view();
        check_revision(&before, args.expected_revision)?;
        let frame_id = resolve_frame_id(&before, &args.frame)?;
        let frame = frame_by_id(&before, &frame_id)?;
        let before_rows: HashSet<String> = frame.rows.iter().map(|row| row.id.clone()).collect();
        let mut values = BTreeMap::new();
        for (column, raw) in args.values {
            values.insert(resolve_column_id(frame, &column)?, raw);
        }
        let view = session.apply(Operation::AddRow {
            frame_id: frame_id.clone(),
            values,
        })?;
        let row_id = frame_by_id(&view, &frame_id)?
            .rows
            .iter()
            .find(|row| !before_rows.contains(&row.id))
            .map(|row| row.id.clone())
            .ok_or_else(|| "The row was created but its ID was not found".to_string())?;
        Ok(Json(receipt(
            &view,
            "Added row".into(),
            Some(frame_id),
            None,
            Some(row_id),
        )))
    }

    /// Delete a frame row by stable ID or 1-based row number.
    #[tool(
        name = "delete_row",
        annotations(title = "Delete a FrameWork row", read_only_hint = false)
    )]
    fn delete_row(
        &self,
        Parameters(args): Parameters<DeleteRowArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let view = self.lock()?.store.view();
        let frame_id = resolve_frame_id(&view, &args.frame)?;
        let row_id = resolve_row_id(frame_by_id(&view, &frame_id)?, &args.row)?;
        self.mutate(
            Operation::DeleteRow {
                frame_id: frame_id.clone(),
                row_id: row_id.clone(),
            },
            args.expected_revision,
            format!("Deleted row {}", args.row),
            Some(frame_id),
            None,
            Some(row_id),
        )
    }

    /// Insert a non-calculated column and return its stable column ID.
    #[tool(
        name = "add_literal_column",
        annotations(title = "Add a literal FrameWork column", read_only_hint = false)
    )]
    fn add_literal_column(
        &self,
        Parameters(args): Parameters<AddLiteralColumnArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let mut session = self.lock()?;
        let before = session.store.view();
        check_revision(&before, args.expected_revision)?;
        let frame_id = resolve_frame_id(&before, &args.frame)?;
        let frame = frame_by_id(&before, &frame_id)?;
        let before_columns: HashSet<String> = frame
            .columns
            .iter()
            .map(|column| column.id.clone())
            .collect();
        let after_column_id = args
            .after_column
            .as_deref()
            .map(|column| resolve_column_id(frame, column))
            .transpose()?;
        let data_type = parse_data_type(&args.data_type)?;
        let view = session.apply(Operation::AddColumn {
            frame_id: frame_id.clone(),
            name: args.name.clone(),
            data_type,
            after_column_id,
        })?;
        let column_id = frame_by_id(&view, &frame_id)?
            .columns
            .iter()
            .find(|column| !before_columns.contains(&column.id))
            .map(|column| column.id.clone())
            .ok_or_else(|| "The column was created but its ID was not found".to_string())?;
        Ok(Json(receipt(
            &view,
            format!("Added literal column '{}'", args.name),
            Some(frame_id),
            Some(column_id),
            None,
        )))
    }

    /// Delete an unreferenced frame column. Dependent formulas prevent deletion.
    #[tool(
        name = "delete_column",
        annotations(title = "Delete a FrameWork column", read_only_hint = false)
    )]
    fn delete_column(
        &self,
        Parameters(args): Parameters<DeleteColumnArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let view = self.lock()?.store.view();
        let frame_id = resolve_frame_id(&view, &args.frame)?;
        let column_id = resolve_column_id(frame_by_id(&view, &frame_id)?, &args.column)?;
        self.mutate(
            Operation::DeleteColumn {
                frame_id: frame_id.clone(),
                column_id: column_id.clone(),
            },
            args.expected_revision,
            format!("Deleted column '{}'", args.column),
            Some(frame_id),
            Some(column_id),
            None,
        )
    }

    /// Change a column's display/data type without rewriting its raw cell values.
    #[tool(
        name = "set_column_type",
        annotations(title = "Set a FrameWork column type", read_only_hint = false)
    )]
    fn set_column_type(
        &self,
        Parameters(args): Parameters<SetColumnTypeArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let view = self.lock()?.store.view();
        let frame_id = resolve_frame_id(&view, &args.frame)?;
        let column_id = resolve_column_id(frame_by_id(&view, &frame_id)?, &args.column)?;
        let data_type = parse_data_type(&args.data_type)?;
        self.mutate(
            Operation::SetColumnType {
                frame_id: frame_id.clone(),
                column_id: column_id.clone(),
                data_type,
            },
            args.expected_revision,
            format!("Changed column '{}' to {}", args.column, args.data_type),
            Some(frame_id),
            Some(column_id),
            None,
        )
    }

    /// Make a column categorical and define its ordered allowed values.
    #[tool(
        name = "set_column_categories",
        annotations(title = "Set FrameWork column categories", read_only_hint = false)
    )]
    fn set_column_categories(
        &self,
        Parameters(args): Parameters<SetColumnCategoriesArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let view = self.lock()?.store.view();
        let frame_id = resolve_frame_id(&view, &args.frame)?;
        let column_id = resolve_column_id(frame_by_id(&view, &frame_id)?, &args.column)?;
        self.mutate(
            Operation::SetColumnCategories {
                frame_id: frame_id.clone(),
                column_id: column_id.clone(),
                categories: args.categories,
            },
            args.expected_revision,
            format!("Changed categories for column '{}'", args.column),
            Some(frame_id),
            Some(column_id),
            None,
        )
    }

    /// Add a calculated column. Formula names resolve to stable IDs before storage.
    #[tool(
        name = "add_calculated_column",
        annotations(title = "Add a calculated column", read_only_hint = false)
    )]
    fn add_calculated_column(
        &self,
        Parameters(args): Parameters<AddCalculatedColumnArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let mut session = self.lock()?;
        let before_view = session.store.view();
        check_revision(&before_view, args.expected_revision)?;
        let frame_id = resolve_frame_id(&before_view, &args.frame)?;
        let frame = frame_by_id(&before_view, &frame_id)?;
        let before_columns: HashSet<String> = frame
            .columns
            .iter()
            .map(|column| column.id.clone())
            .collect();
        let computed = before_view
            .computed_frames
            .get(&frame_id)
            .ok_or_else(|| format!("Computed data for frame '{}' is unavailable", frame.name))?;
        let output_column_id = Uuid::new_v4().to_string();
        let mut steps = rendered_pipeline_inputs(frame, &computed.steps)?;
        steps.push(FrameStepInput::WithColumns {
            columns: vec![ExistingFormulaInput {
                output_column_id,
                name: args.name.clone(),
                formula: args.formula,
            }],
        });
        let view = session.apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps,
        })?;
        let column_id = frame_by_id(&view, &frame_id)?
            .columns
            .iter()
            .find(|column| !before_columns.contains(&column.id))
            .map(|column| column.id.clone())
            .ok_or_else(|| {
                "The calculated column was created but its ID was not found".to_string()
            })?;
        Ok(Json(receipt(
            &view,
            format!("Added calculated column '{}'", args.name),
            Some(frame_id),
            Some(column_id),
            None,
        )))
    }

    /// Add, replace, or remove a formula override for one cell.
    #[tool(
        name = "set_cell_override",
        annotations(title = "Set a cell formula override", read_only_hint = false)
    )]
    fn set_cell_override(
        &self,
        Parameters(args): Parameters<SetCellOverrideArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let view = self.lock()?.store.view();
        let frame_id = resolve_frame_id(&view, &args.frame)?;
        let frame = frame_by_id(&view, &frame_id)?;
        let row_id = resolve_row_id(frame, &args.row)?;
        let column_id = resolve_column_id(frame, &args.column)?;
        let action = if args.formula.is_some() {
            "Set"
        } else {
            "Removed"
        };
        self.mutate(
            Operation::SetCellOverride {
                frame_id: frame_id.clone(),
                row_id: row_id.clone(),
                column_id: column_id.clone(),
                formula: args.formula,
            },
            args.expected_revision,
            format!("{action} the cell formula override"),
            Some(frame_id),
            Some(column_id),
            Some(row_id),
        )
    }

    /// Add one saved aggregate to a frame column.
    #[tool(
        name = "add_summary",
        annotations(title = "Add a frame summary", read_only_hint = false)
    )]
    fn add_summary(
        &self,
        Parameters(args): Parameters<AddSummaryArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let view = self.lock()?.store.view();
        let frame_id = resolve_frame_id(&view, &args.frame)?;
        let frame = frame_by_id(&view, &frame_id)?;
        let column_id = resolve_column_id(frame, &args.column)?;
        let operation = match normalize_name(&args.operation).as_str() {
            "sum" | "total" => SummaryOperation::Sum,
            "mean" | "average" | "avg" => SummaryOperation::Mean,
            "25" | "25%" | "q1" | "quartile25" => SummaryOperation::Quartile25,
            "median" => SummaryOperation::Median,
            "75" | "75%" | "q3" | "quartile75" => SummaryOperation::Quartile75,
            "min" | "minimum" => SummaryOperation::Min,
            "max" | "maximum" => SummaryOperation::Max,
            "count" => SummaryOperation::Count,
            "missing" | "null" | "nulls" => SummaryOperation::Missing,
            "distinct" | "countdistinct" | "nunique" => SummaryOperation::CountDistinct,
            "mode" => SummaryOperation::Mode,
            _ => {
                return Err(
                    "operation must be sum, mean, 25%, median, 75%, min, max, count, missing, distinct, or mode".into(),
                );
            }
        };
        self.mutate(
            Operation::AddSummary {
                frame_id: frame_id.clone(),
                column_id: column_id.clone(),
                operation,
            },
            args.expected_revision,
            format!("Added {} summary", args.operation),
            Some(frame_id),
            Some(column_id),
            None,
        )
    }

    /// Undo the most recent accepted document operation.
    #[tool(
        name = "undo",
        annotations(title = "Undo a FrameWork change", read_only_hint = false)
    )]
    fn undo(
        &self,
        Parameters(args): Parameters<HistoryArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let mut session = self.lock()?;
        let before = session.store.view();
        check_revision(&before, args.expected_revision)?;
        if !before.can_undo {
            return Err("There is no change to undo".into());
        }
        let view = session.store.undo();
        session
            .store
            .save(&session.path)
            .map_err(|error| error.to_string())?;
        Ok(Json(receipt(
            &view,
            "Undid the most recent change".into(),
            None,
            None,
            None,
        )))
    }

    /// Redo the most recently undone document operation.
    #[tool(
        name = "redo",
        annotations(title = "Redo a FrameWork change", read_only_hint = false)
    )]
    fn redo(
        &self,
        Parameters(args): Parameters<HistoryArgs>,
    ) -> Result<Json<MutationReceipt>, String> {
        let mut session = self.lock()?;
        let before = session.store.view();
        check_revision(&before, args.expected_revision)?;
        if !before.can_redo {
            return Err("There is no change to redo".into());
        }
        let view = session.store.redo();
        session
            .store
            .save(&session.path)
            .map_err(|error| error.to_string())?;
        Ok(Json(receipt(
            &view,
            "Redid the most recent change".into(),
            None,
            None,
            None,
        )))
    }
}

#[tool_handler(
    name = "framework",
    version = "0.1.2",
    instructions = "FrameWork is a local structured-data canvas. Use inspect_document first, then get_frame when row-level context is needed. Prefer stable IDs returned by reads, include expectedRevision on writes, and read back changed frames after formula edits. Cells are addressed by frame, row, and column rather than screen coordinates. Mutating tools persist immediately and support undo/redo. Friendly task-level tools cover common work; call describe_operations then apply_operation for complete mutation functionality."
)]
impl ServerHandler for FrameworkMcp {}

fn document_summary(view: &DocumentView, path: &Path) -> DocumentSummary {
    let objects = view
        .document
        .objects
        .iter()
        .map(|object| match object {
            DataObject::Value(value) => ObjectSummary {
                id: value.id.clone(),
                name: value.name.clone(),
                kind: "value".into(),
                value: Some(value.raw.clone()),
                data_type: Some(data_type_name(value.data_type)),
                row_count: None,
                columns: Vec::new(),
            },
            DataObject::Result(result) => ObjectSummary {
                id: result.id.clone(),
                name: result.name.clone(),
                kind: "result".into(),
                // The formula and the answer as it stands, both: what a
                // result is and what it says are equally the point of one.
                value: Some(
                    view.computed_results
                        .get(&result.id)
                        .map(|computed| {
                            format!("= {} → {}", computed.formula, computed.cell.display)
                        })
                        .unwrap_or_default(),
                ),
                data_type: view
                    .computed_results
                    .get(&result.id)
                    .map(|computed| data_type_name(computed.data_type)),
                row_count: None,
                columns: Vec::new(),
            },
            DataObject::Block(block) => ObjectSummary {
                id: block.id.clone(),
                name: block.name.clone(),
                kind: "block".into(),
                // Every line as it was typed with its answer after it, in
                // order: a block is a worked calculation, and the working is
                // the content. Blank lines carry nothing and are left out.
                value: Some(
                    view.computed_blocks
                        .get(&block.id)
                        .map(|computed| {
                            computed
                                .lines
                                .iter()
                                .filter(|line| !line.blank)
                                .map(|line| match (&line.cell.error, line.comment) {
                                    (_, true) => line.text.clone(),
                                    (Some(error), _) => format!("{} → {error}", line.text),
                                    (None, _) => {
                                        format!("{} → {}", line.text, line.cell.display)
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("; ")
                        })
                        .unwrap_or_default(),
                ),
                data_type: None,
                row_count: Some(block.lines.len()),
                columns: Vec::new(),
            },
            DataObject::Series(series) => ObjectSummary {
                id: series.id.clone(),
                name: series.name.clone(),
                kind: "series".into(),
                // The values themselves, joined, rather than a count: a list
                // is small by nature and knowing what is in it is the whole
                // reason to ask about one.
                value: Some(series.values.join(", ")),
                data_type: Some(data_type_name(series.data_type)),
                row_count: Some(series.values.len()),
                columns: Vec::new(),
            },
            DataObject::Container(container) => ObjectSummary {
                id: container.id.clone(),
                name: container.name.clone(),
                kind: "container".into(),
                // The names it holds, so a reader can write
                // `Finance`.`Interest rate` without a second lookup.
                value: Some(
                    container
                        .member_ids
                        .iter()
                        .filter_map(|member_id| view.document.object(member_id).ok())
                        .map(|member| member.name())
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
                data_type: None,
                row_count: Some(container.member_ids.len()),
                columns: Vec::new(),
            },
            DataObject::Frame(frame) => ObjectSummary {
                id: frame.id.clone(),
                name: frame.name.clone(),
                kind: "frame".into(),
                value: None,
                data_type: None,
                // The literal rows are the count only when the frame's rows
                // *are* its data. A computed or file-backed frame stores
                // none here, and reporting its zero as a count told agents
                // an imported frame was empty; the honest answer is the
                // engine's, or nothing — get_frame counts what this cannot.
                row_count: if frame.owns_its_rows() && frame.steps.is_empty() {
                    Some(frame.rows.len())
                } else {
                    view.computed_frames
                        .get(&frame.id)
                        .and_then(|computed| computed.total_rows)
                },
                columns: column_summaries(view, frame),
            },
            DataObject::Text(text) => ObjectSummary {
                id: text.id.clone(),
                name: text.name.clone(),
                kind: "text".into(),
                value: Some(text.text.clone()),
                data_type: Some("text".into()),
                row_count: None,
                columns: Vec::new(),
            },
            DataObject::Plot(plot) => ObjectSummary {
                id: plot.id.clone(),
                name: plot.name.clone(),
                kind: "plot".into(),
                value: None,
                data_type: Some("vega-lite".into()),
                row_count: None,
                columns: Vec::new(),
            },
        })
        .collect();
    DocumentSummary {
        id: view.document.id.clone(),
        name: view.document.name.clone(),
        revision: view.document.revision,
        document_path: path.display().to_string(),
        can_undo: view.can_undo,
        can_redo: view.can_redo,
        objects,
        formula_reference: "Search formula functions with the search_functions tool — \
                            aliases cover Excel and Polars vocabulary."
            .into(),
    }
}

fn rendered_pipeline_inputs(
    frame: &FrameObject,
    rendered: &[RenderedFrameStep],
) -> Result<Vec<FrameStepInput>, String> {
    let output_name = |output: &RenderedDerivedExpression| {
        frame
            .columns
            .iter()
            .chain(frame.base_columns.iter())
            .find(|column| column.id == output.output_column_id)
            .map(|column| column.name.clone())
            .unwrap_or_else(|| output.output_column_id.clone())
    };
    let expression = |output: &RenderedDerivedExpression| ExistingFormulaInput {
        output_column_id: output.output_column_id.clone(),
        name: output_name(output),
        formula: output.formula.clone(),
    };
    let formula_token = |name: &str| format!("`{}`", name.replace('`', "``"));

    rendered
        .iter()
        .filter_map(|step| {
            Some(Ok(match step {
                RenderedFrameStep::Filter {
                    predicates,
                    match_all,
                } => FrameStepInput::Filter {
                    predicates: predicates.clone(),
                    match_all: *match_all,
                },
                RenderedFrameStep::WithColumns { columns } => FrameStepInput::WithColumns {
                    columns: columns.iter().map(&expression).collect(),
                },
                RenderedFrameStep::Select { column_ids } => FrameStepInput::Select {
                    column_ids: column_ids.clone(),
                },
                RenderedFrameStep::Summarize {
                    group_keys,
                    aggregates,
                    maintain_order,
                } => FrameStepInput::Summarize {
                    group_keys: group_keys.iter().map(&expression).collect(),
                    aggregates: aggregates.iter().map(&expression).collect(),
                    maintain_order: *maintain_order,
                },
                RenderedFrameStep::Sort { keys } => FrameStepInput::Sort {
                    keys: keys
                        .iter()
                        .map(|key| SortInput {
                            column_id: key.column_id.clone(),
                            descending: key.descending,
                        })
                        .collect(),
                },
                RenderedFrameStep::Union { frame_id, .. } => FrameStepInput::Union {
                    frame_id: frame_id.clone(),
                },
                RenderedFrameStep::Expand { frame_id, .. } => FrameStepInput::Expand {
                    frame_id: frame_id.clone(),
                },
                RenderedFrameStep::Pivot {
                    names_column_id,
                    values_column_id,
                    aggregate,
                    ..
                } => FrameStepInput::Pivot {
                    names_column_id: names_column_id.clone(),
                    values_column_id: values_column_id.clone(),
                    aggregate: *aggregate,
                },
                RenderedFrameStep::Unpivot {
                    columns,
                    name_column_id,
                    name_column_name,
                    value_column_id,
                    value_column_name,
                } => FrameStepInput::Unpivot {
                    columns: columns
                        .iter()
                        .map(|column| formula_token(&column.label))
                        .collect::<Vec<_>>()
                        .join(", "),
                    name_column_id: name_column_id.clone(),
                    name_column_name: name_column_name.clone(),
                    value_column_id: value_column_id.clone(),
                    value_column_name: value_column_name.clone(),
                },
                RenderedFrameStep::Comment { text } => {
                    FrameStepInput::Comment { text: text.clone() }
                }
                // A join is the fixed input to Wrangle and is reattached by
                // SetFramePipeline. It is not an editable pipeline input.
                RenderedFrameStep::Join { .. } => return None,
            }))
        })
        .collect()
}

fn frame_snapshot(
    view: &DocumentView,
    frame_id: &str,
    limit: usize,
    page: Option<&FramePage>,
) -> Result<FrameSnapshot, String> {
    let frame = frame_by_id(view, frame_id)?;
    let computed = view
        .computed_frames
        .get(frame_id)
        .ok_or_else(|| format!("Computed data for frame '{}' is unavailable", frame.name))?;
    let rows: Vec<RowSnapshot> = if let Some(page) = page {
        page.rows
            .iter()
            .enumerate()
            .map(|(index, values)| RowSnapshot {
                id: page.row_ids[index].clone(),
                index: page.offset + index + 1,
                cells: page
                    .columns
                    .iter()
                    .zip(values)
                    .map(|(column, display)| CellSnapshot {
                        column_id: column.id.clone(),
                        column_name: column.name.clone(),
                        raw: String::new(),
                        display: display.clone(),
                        numeric_value: matches!(
                            column.data_type,
                            DataType::Integer
                                | DataType::Number
                                | DataType::Currency
                                | DataType::Percentage
                        )
                        .then(|| display.parse().ok())
                        .flatten(),
                        typed_value: page_scalar_value(display, column.data_type),
                        formula: computed.formulas.get(&column.id).cloned(),
                        is_override: false,
                        error: None,
                    })
                    .collect(),
            })
            .collect()
    } else {
        frame
            .rows
            .iter()
            .take(limit)
            .enumerate()
            .map(|(index, row)| {
                let cells = frame
                    .columns
                    .iter()
                    .map(|column| {
                        let cell = row.cells.get(&column.id);
                        let raw = cell.map(|cell| cell.raw.clone()).unwrap_or_default();
                        let override_formula = computed
                            .override_formulas
                            .get(&row.id)
                            .and_then(|formulas| formulas.get(&column.id))
                            .cloned();
                        let column_formula = computed.formulas.get(&column.id).cloned();
                        let formula = override_formula.or(column_formula);
                        let computed_cell = computed
                            .rows
                            .get(&row.id)
                            .and_then(|cells| cells.get(&column.id));
                        let is_formula = formula.is_some();
                        CellSnapshot {
                            column_id: column.id.clone(),
                            column_name: column.name.clone(),
                            raw: raw.clone(),
                            display: if is_formula {
                                computed_cell
                                    .map(|cell| cell.display.clone())
                                    .unwrap_or_else(|| "—".into())
                            } else {
                                raw
                            },
                            numeric_value: computed_cell.and_then(|cell| cell.value),
                            typed_value: computed_cell
                                .map(|cell| api_scalar_value(&cell.typed_value))
                                .unwrap_or(ApiScalarValue::Null),
                            formula,
                            is_override: computed_cell.is_some_and(|cell| cell.is_override),
                            error: computed_cell.and_then(|cell| cell.error.clone()),
                        }
                    })
                    .collect();
                RowSnapshot {
                    id: row.id.clone(),
                    index: index + 1,
                    cells,
                }
            })
            .collect()
    };
    let summaries = frame
        .summaries
        .iter()
        .map(|summary| {
            let column_name = frame
                .columns
                .iter()
                .find(|column| column.id == summary.column_id)
                .map(|column| column.name.clone())
                .unwrap_or_else(|| "#REF".into());
            let computed_summary = computed.summaries.get(&summary.id);
            SummarySnapshot {
                id: summary.id.clone(),
                label: summary.label.clone(),
                column_id: summary.column_id.clone(),
                column_name,
                operation: summary_operation_name(summary.operation),
                display: computed_summary
                    .map(|cell| cell.display.clone())
                    .unwrap_or_else(|| "—".into()),
                numeric_value: computed_summary.and_then(|cell| cell.value),
                typed_value: computed_summary
                    .map(|cell| api_scalar_value(&cell.typed_value))
                    .unwrap_or(ApiScalarValue::Null),
                error: computed_summary.and_then(|cell| cell.error.clone()),
            }
        })
        .collect();
    Ok(FrameSnapshot {
        id: frame.id.clone(),
        name: frame.name.clone(),
        revision: view.document.revision,
        total_row_count: page.map_or(frame.rows.len(), |page| page.total_rows),
        returned_row_count: rows.len(),
        columns: column_summaries(view, frame),
        rows,
        summaries,
    })
}

fn page_scalar_value(display: &str, data_type: DataType) -> ApiScalarValue {
    if display.is_empty() || display == "—" {
        return ApiScalarValue::Null;
    }
    match data_type {
        DataType::Integer | DataType::Number | DataType::Currency | DataType::Percentage => display
            .parse()
            .map(ApiScalarValue::Number)
            .unwrap_or(ApiScalarValue::Null),
        DataType::Boolean => match display {
            "true" => ApiScalarValue::Boolean(true),
            "false" => ApiScalarValue::Boolean(false),
            _ => ApiScalarValue::Null,
        },
        DataType::Date => ApiScalarValue::Date(display.to_string()),
        DataType::String | DataType::Categorical => ApiScalarValue::String(display.to_string()),
    }
}

fn api_scalar_value(value: &ScalarValue) -> ApiScalarValue {
    match value {
        ScalarValue::Null => ApiScalarValue::Null,
        ScalarValue::Number(value) => ApiScalarValue::Number(*value),
        ScalarValue::String(value) => ApiScalarValue::String(value.clone()),
        ScalarValue::Boolean(value) => ApiScalarValue::Boolean(*value),
        ScalarValue::Date(value) => ApiScalarValue::Date(value.format("%Y-%m-%d").to_string()),
    }
}

fn column_summaries(view: &DocumentView, frame: &FrameObject) -> Vec<ColumnSummary> {
    let computed = view.computed_frames.get(&frame.id);
    frame
        .columns
        .iter()
        .map(|column| ColumnSummary {
            id: column.id.clone(),
            name: column.name.clone(),
            data_type: data_type_name(column.data_type),
            categories: column.categories.clone(),
            formula: computed.and_then(|frame| frame.formulas.get(&column.id).cloned()),
        })
        .collect()
}

fn receipt(
    view: &DocumentView,
    message: String,
    affected_object_id: Option<String>,
    affected_column_id: Option<String>,
    affected_row_id: Option<String>,
) -> MutationReceipt {
    MutationReceipt {
        revision: view.document.revision,
        message,
        affected_object_id,
        affected_column_id,
        affected_row_id,
        can_undo: view.can_undo,
        can_redo: view.can_redo,
    }
}

fn check_revision(view: &DocumentView, expected: Option<u64>) -> Result<(), String> {
    if let Some(expected) = expected
        && expected != view.document.revision
    {
        return Err(format!(
            "Revision conflict: expected {expected}, but the document is at revision {}. Inspect the document again before writing.",
            view.document.revision
        ));
    }
    Ok(())
}

fn object_ids(view: &DocumentView) -> HashSet<String> {
    view.document
        .objects
        .iter()
        .map(|object| object.id().to_string())
        .collect()
}

fn new_object_id(view: &DocumentView, before: &HashSet<String>) -> Result<String, String> {
    view.document
        .objects
        .iter()
        .find(|object| !before.contains(object.id()))
        .map(|object| object.id().to_string())
        .ok_or_else(|| "The object was created but its ID was not found".into())
}

fn resolve_frame_id(view: &DocumentView, reference: &str) -> Result<String, String> {
    resolve_object(
        view,
        reference,
        |object| matches!(object, DataObject::Frame(_)),
        "frame",
    )
}

/// A linked frame's pass-through prefix written back as save inputs, so a
/// tool appending one step re-sends the chain it found. Only the
/// projection shapes a linked frame starts with appear here; anything else
/// means the frame already has a hand-authored chain, which a convenience
/// tool should not silently rewrite — the caller sees the refusal as an
/// invalid step conversion.
fn pass_through_inputs(rendered: &[RenderedFrameStep], frame: &FrameObject) -> Vec<FrameStepInput> {
    let name_of = |output_column_id: &str| {
        frame
            .columns
            .iter()
            .find(|column| column.id == output_column_id)
            .map(|column| column.name.clone())
            .unwrap_or_else(|| output_column_id.to_string())
    };
    rendered
        .iter()
        .filter_map(|step| match step {
            RenderedFrameStep::WithColumns { columns } => Some(FrameStepInput::WithColumns {
                columns: columns
                    .iter()
                    .map(|column| ExistingFormulaInput {
                        output_column_id: column.output_column_id.clone(),
                        name: name_of(&column.output_column_id),
                        formula: column.formula.clone(),
                    })
                    .collect(),
            }),
            RenderedFrameStep::Select { column_ids } => Some(FrameStepInput::Select {
                column_ids: column_ids.clone(),
            }),
            _ => None,
        })
        .collect()
}

/// Finds the one named block line matching `name` and returns its block's
/// id with the whole source retyped so that line reads `name = raw`. The
/// block is edited through its own whole-text operation because that is
/// what a block *is* — but nobody updating one parameter should have to
/// re-send the lines they are not touching, so this does the splice.
fn block_line_rewrite(
    view: &DocumentView,
    name: &str,
    raw: &str,
) -> Result<(String, String), String> {
    let mut matches = Vec::new();
    for object in &view.document.objects {
        if let DataObject::Block(block) = object {
            for line in &block.lines {
                if line.named && line.name == name {
                    matches.push((block.id.clone(), line.id.clone()));
                }
            }
        }
    }
    match matches.as_slice() {
        [] => Err(format!(
            "Unknown value '{name}'. Values live on the canvas or as named lines of a \
             formula block — inspect_document lists both."
        )),
        [(block_id, line_id)] => {
            let computed = view
                .computed_blocks
                .get(block_id)
                .ok_or("The block's computed source is unavailable")?;
            let source = computed
                .lines
                .iter()
                .map(|line| {
                    if line.id == *line_id {
                        // A raw that would not read back as one value —
                        // text, mostly — travels quoted; everything the
                        // formula language reads bare stays bare.
                        let looks_like_date = raw.len() == 10
                            && raw.char_indices().all(|(at, c)| match at {
                                4 | 7 => c == '-',
                                _ => c.is_ascii_digit(),
                            });
                        let written = if raw.parse::<f64>().is_ok()
                            || raw.parse::<bool>().is_ok()
                            || looks_like_date
                        {
                            raw.to_string()
                        } else {
                            format!("\"{}\"", raw.replace('"', "\\\""))
                        };
                        format!("{name} = {written}")
                    } else {
                        line.text.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            Ok((block_id.clone(), source))
        }
        _ => Err(format!(
            "'{name}' names lines in more than one block. Use set_block_source on the \
             block you mean."
        )),
    }
}

fn resolve_value_id(view: &DocumentView, reference: &str) -> Result<String, String> {
    resolve_object(
        view,
        reference,
        |object| matches!(object, DataObject::Value(_)),
        "value",
    )
}

fn resolve_block_id(view: &DocumentView, reference: &str) -> Result<String, String> {
    resolve_object(
        view,
        reference,
        |object| matches!(object, DataObject::Block(_)),
        "block",
    )
}

fn resolve_any_object_id(view: &DocumentView, reference: &str) -> Result<String, String> {
    resolve_object(view, reference, |_| true, "object")
}

fn resolve_object(
    view: &DocumentView,
    reference: &str,
    accepts: impl Fn(&DataObject) -> bool,
    kind: &str,
) -> Result<String, String> {
    if let Some(object) = view
        .document
        .objects
        .iter()
        .find(|object| object.id() == reference && accepts(object))
    {
        return Ok(object.id().to_string());
    }
    let normalized = normalize_name(reference);
    let matches: Vec<&DataObject> = view
        .document
        .objects
        .iter()
        .filter(|object| accepts(object) && normalize_name(object.name()) == normalized)
        .collect();
    match matches.as_slice() {
        [object] => Ok(object.id().to_string()),
        [] => Err(format!("Unknown {kind} '{reference}'")),
        _ => Err(format!(
            "Ambiguous {kind} name '{reference}'; use a stable ID from inspect_document"
        )),
    }
}

fn frame_by_id<'a>(view: &'a DocumentView, frame_id: &str) -> Result<&'a FrameObject, String> {
    view.document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.id == frame_id => Some(frame),
            _ => None,
        })
        .ok_or_else(|| format!("Unknown frame ID '{frame_id}'"))
}

fn resolve_column_id(frame: &FrameObject, reference: &str) -> Result<String, String> {
    if let Some(column) = frame.columns.iter().find(|column| column.id == reference) {
        return Ok(column.id.clone());
    }
    let normalized = normalize_name(reference);
    let matches: Vec<_> = frame
        .columns
        .iter()
        .filter(|column| normalize_name(&column.name) == normalized)
        .collect();
    match matches.as_slice() {
        [column] => Ok(column.id.clone()),
        [] => Err(format!(
            "Unknown column '{reference}' in frame '{}'",
            frame.name
        )),
        _ => Err(format!(
            "Ambiguous column name '{reference}' in frame '{}'; use a stable ID",
            frame.name
        )),
    }
}

fn resolve_row_id(frame: &FrameObject, reference: &str) -> Result<String, String> {
    if let Some(row) = frame.rows.iter().find(|row| row.id == reference) {
        return Ok(row.id.clone());
    }
    if let Ok(index) = reference.parse::<usize>()
        && index > 0
        && let Some(row) = frame.rows.get(index - 1)
    {
        return Ok(row.id.clone());
    }
    Err(format!(
        "Unknown row '{reference}' in frame '{}'; use a 1-based row number or stable row ID",
        frame.name
    ))
}

fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn data_type_name(data_type: DataType) -> String {
    match data_type {
        DataType::String => "string",
        DataType::Categorical => "categorical",
        DataType::Integer => "integer",
        DataType::Number => "number",
        DataType::Currency => "currency",
        DataType::Percentage => "percentage",
        DataType::Boolean => "boolean",
        DataType::Date => "date",
    }
    .into()
}

fn parse_data_type(value: &str) -> Result<DataType, String> {
    match normalize_name(value).as_str() {
        "string" | "text" => Ok(DataType::String),
        "categorical" | "category" | "enum" => Ok(DataType::Categorical),
        "integer" | "int" => Ok(DataType::Integer),
        "number" | "numeric" => Ok(DataType::Number),
        "currency" | "money" => Ok(DataType::Currency),
        "percentage" | "percent" => Ok(DataType::Percentage),
        "boolean" | "bool" => Ok(DataType::Boolean),
        "date" => Ok(DataType::Date),
        _ => Err("dataType must be string/text, categorical/enum, integer/int, number, currency, percentage/percent, boolean/bool, or date".into()),
    }
}

fn summary_operation_name(operation: SummaryOperation) -> String {
    match operation {
        SummaryOperation::Sum => "sum",
        SummaryOperation::Mean => "mean",
        SummaryOperation::Quartile25 => "quartile25",
        SummaryOperation::Median => "median",
        SummaryOperation::Quartile75 => "quartile75",
        SummaryOperation::Min => "min",
        SummaryOperation::Max => "max",
        SummaryOperation::Count => "count",
        SummaryOperation::Missing => "missing",
        SummaryOperation::CountDistinct => "countDistinct",
        SummaryOperation::Mode => "mode",
    }
    .into()
}

fn document_path() -> Result<PathBuf, String> {
    let mut args = env::args().skip(1);
    let mut path = env::var_os("FRAMEWORK_DOCUMENT").map(PathBuf::from);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--document" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--document requires a path".to_string())?;
                path = Some(PathBuf::from(value));
            }
            "--help" | "-h" => {
                eprintln!(
                    "framework-mcp [--document PATH]\n\nEnvironment: FRAMEWORK_DOCUMENT=PATH\nDefault: {DEFAULT_DOCUMENT}"
                );
                std::process::exit(0);
            }
            other => return Err(format!("Unknown argument '{other}'")),
        }
    }
    Ok(path.unwrap_or_else(|| PathBuf::from(DEFAULT_DOCUMENT)))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = document_path().map_err(std::io::Error::other)?;
    let server = FrameworkMcp::open(path).map_err(std::io::Error::other)?;
    server
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn mcp_access_is_opt_in_for_an_application_data_directory() {
        let directory = unique_temp_path("framework-mcp-access");
        assert!(!mcp_enabled_at(&directory));
        let marker = directory
            .join(FRAMEWORK_APP_IDENTIFIER)
            .join(MCP_ENABLED_NAME);
        std::fs::create_dir_all(marker.parent().unwrap()).unwrap();
        std::fs::write(&marker, b"enabled\n").unwrap();
        assert!(mcp_enabled_at(&directory));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn function_search_finds_sequence_by_its_excel_vocabulary() {
        let (server, _path) = test_server();
        // "range" is the word an agent or Excel person actually arrives
        // with; the timesheet smoke test hand-wrote a 16-row CSV because
        // this exact search had no tool to land on.
        let result = server
            .search_functions(Parameters(SearchFunctionsArgs {
                query: Some("range".into()),
                limit: None,
            }))
            .unwrap()
            .0;
        assert!(
            result
                .functions
                .iter()
                .any(|function| function.name == "sequence"),
            "searching 'range' must surface sequence()"
        );
        // Name hits rank ahead of description mentions.
        let sequence_index = result
            .functions
            .iter()
            .position(|function| function.name == "sequence")
            .unwrap();
        assert!(sequence_index < 5, "sequence buried at {sequence_index}");

        let everything = server
            .search_functions(Parameters(SearchFunctionsArgs {
                query: None,
                limit: None,
            }))
            .unwrap()
            .0;
        assert_eq!(everything.total_matches, everything.functions.len());
        assert!(everything.total_matches > 50);
    }

    #[test]
    fn generator_frames_grow_expand_and_follow_their_value() {
        let (server, _path) = test_server();
        // The catalog names the new operations, so a generic agent can find
        // them without the task-level tools.
        let catalog = server.describe_operations().unwrap().0;
        assert!(
            catalog
                .type_script
                .contains(r#""type": "addGeneratorFrame""#)
        );
        assert!(
            catalog
                .type_script
                .contains(r#""type": "setFrameGenerator""#)
        );
        assert!(catalog.type_script.contains(r#""type": "addEntryColumn""#));
        assert!(catalog.type_script.contains(r#""type": "setEntryValue""#));
        assert!(
            catalog
                .type_script
                .contains(r#""type": "refreshFramePipeline""#)
        );

        server
            .create_generator_frame(Parameters(CreateGeneratorFrameArgs {
                name: "Days".into(),
                formula: "sequence(0, 4)".into(),
                column_name: Some("Day".into()),
                x: None,
                y: None,
                expected_revision: None,
            }))
            .unwrap();
        let snapshot = server
            .get_frame(Parameters(GetFrameArgs {
                frame: "Days".into(),
                limit: None,
            }))
            .unwrap()
            .0;
        assert_eq!(snapshot.rows.len(), 4);

        server
            .set_frame_generator(Parameters(SetFrameGeneratorArgs {
                frame: "Days".into(),
                formula: "sequence(0, 7)".into(),
                expected_revision: None,
            }))
            .unwrap();
        let snapshot = server
            .get_frame(Parameters(GetFrameArgs {
                frame: "Days".into(),
                limit: None,
            }))
            .unwrap()
            .0;
        assert_eq!(snapshot.rows.len(), 7);
    }

    #[test]
    fn the_timesheet_flow_runs_on_named_tools_alone() {
        // Both smoke-test rounds reached for the raw operation escape
        // hatch for the cross join and the wide view. This walks the whole
        // app — parameter, generator, expansion, keyed entry, crosstab,
        // period flip — through named tools only, which is the bar the
        // verdicts asked for.
        let (server, _path) = test_server();
        server
            .create_block(Parameters(CreateBlockArgs {
                name: "Params".into(),
                x: None,
                y: None,
                expected_revision: None,
            }))
            .unwrap();
        server
            .set_block_source(Parameters(SetBlockSourceArgs {
                block: "Params".into(),
                source: "Anchor = 2026-09-15".into(),
                expected_revision: None,
            }))
            .unwrap();
        server
            .create_generator_frame(Parameters(CreateGeneratorFrameArgs {
                name: "Period".into(),
                formula: "sequence(`Params`.`Anchor`.dt.month_start(), `Params`.`Anchor` + 1)"
                    .into(),
                column_name: Some("Date".into()),
                x: None,
                y: None,
                expected_revision: None,
            }))
            .unwrap();
        server
            .create_frame(Parameters(CreateFrameArgs {
                name: "Lines".into(),
                grid: vec![
                    vec!["Line".into()],
                    vec!["Admin".into()],
                    vec!["Marketing".into()],
                ],
                x: None,
                y: None,
                expected_revision: None,
            }))
            .unwrap();
        server
            .expand_frame(Parameters(ExpandFrameArgs {
                frame: "Lines".into(),
                against: "Period".into(),
                name: "Sheet".into(),
                x: None,
                y: None,
                expected_revision: None,
            }))
            .unwrap();
        let sheet = server
            .get_frame(Parameters(GetFrameArgs {
                frame: "Sheet".into(),
                limit: Some(1000),
            }))
            .unwrap()
            .0;
        assert_eq!(sheet.total_row_count, 2 * 15, "two lines × Sept 1–15");

        server
            .add_entry_column(Parameters(AddEntryColumnArgs {
                frame: "Sheet".into(),
                name: "Hours".into(),
                data_type: "number".into(),
                key_columns: vec!["Line".into(), "Date".into()],
                expected_revision: None,
            }))
            .unwrap();
        server
            .set_entry_value(Parameters(SetEntryValueArgs {
                frame: "Sheet".into(),
                column: "Hours".into(),
                key: vec!["Admin".into(), "2026-09-03".into()],
                raw: "8".into(),
                expected_revision: None,
            }))
            .unwrap();
        server
            .set_crosstab(Parameters(SetCrosstabArgs {
                frame: "Sheet".into(),
                names_column: Some("Date".into()),
                values_column: Some("Hours".into()),
                off: None,
                expected_revision: None,
            }))
            .unwrap();

        // The period flip: set_value reaches the block-hosted parameter,
        // the sheet regrows, and the entered hours come back with it.
        server
            .set_value(Parameters(SetValueArgs {
                value: "Anchor".into(),
                raw: "2026-10-31".into(),
                expected_revision: None,
            }))
            .unwrap();
        let october = server
            .get_frame(Parameters(GetFrameArgs {
                frame: "Sheet".into(),
                limit: Some(1000),
            }))
            .unwrap()
            .0;
        assert_eq!(
            october.total_row_count,
            2 * 31,
            "two lines × all of October"
        );
        server
            .set_value(Parameters(SetValueArgs {
                value: "Anchor".into(),
                raw: "2026-09-15".into(),
                expected_revision: None,
            }))
            .unwrap();
        let back = server
            .get_frame(Parameters(GetFrameArgs {
                frame: "Sheet".into(),
                limit: Some(1000),
            }))
            .unwrap()
            .0;
        assert_eq!(back.total_row_count, 30);
        let hours_back = back
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .filter(|cell| cell.column_name == "Hours")
            .filter(|cell| !cell.display.is_empty())
            .count();
        assert_eq!(hours_back, 1, "the entered value survived the round trip");
    }

    #[test]
    fn an_imported_frame_shows_its_rows_through_get_frame() {
        // Run 2 of the timesheet smoke test imported a CSV successfully and
        // then read it back as zero rows: file-backed rows answer through
        // the paged read, and the snapshot only paged frames with a chain.
        // The agent concluded the import was broken and hand-transcribed
        // the file. The snapshot must never make a working import look
        // empty.
        let (server, path) = test_server();
        let csv = path.with_extension("csv");
        std::fs::write(&csv, "Line,Section\n1,General\n2,Services\n3,Field\n").unwrap();
        server
            .apply_operation(Parameters(ApplyOperationArgs {
                operation: serde_json::json!({
                    "type": "importFrameFromFile",
                    "name": "Imported",
                    "path": csv.display().to_string(),
                    "x": 0.0,
                    "y": 0.0,
                }),
                expected_revision: None,
            }))
            .unwrap();
        let snapshot = server
            .get_frame(Parameters(GetFrameArgs {
                frame: "Imported".into(),
                limit: None,
            }))
            .unwrap()
            .0;
        assert_eq!(snapshot.total_row_count, 3);
        assert_eq!(snapshot.returned_row_count, 3);
        let section = snapshot.rows[0]
            .cells
            .iter()
            .find(|cell| cell.column_name == "Section")
            .unwrap();
        assert_eq!(section.display, "General");
        std::fs::remove_file(csv).ok();
    }

    #[test]
    fn apply_operation_declares_an_object_and_survives_stringification() {
        // The schema half: a client reads the parameter schema to decide
        // how to encode the payload. Without an explicit `type: object`,
        // real clients stringified the operation and every call died —
        // the smoke test that found this lost its entire escape hatch.
        let schema = serde_json::to_value(schemars::schema_for!(ApplyOperationArgs)).unwrap();
        assert_eq!(
            schema["properties"]["operation"]["type"],
            serde_json::json!("object"),
            "the operation parameter must declare itself an object"
        );

        // The tolerance half: a stringified payload still means the object
        // inside it.
        let (server, _path) = test_server();
        let receipt = server
            .apply_operation(Parameters(ApplyOperationArgs {
                operation: serde_json::Value::String(
                    r#"{"type":"renameDocument","name":"Sent as a string"}"#.into(),
                ),
                expected_revision: Some(0),
            }))
            .unwrap()
            .0;
        assert_eq!(receipt.revision, 1);
        assert_eq!(
            server.inspect_document().unwrap().0.name,
            "Sent as a string"
        );
    }

    #[test]
    fn generic_operation_surface_tracks_and_applies_the_canonical_enum() {
        let (server, path) = test_server();
        let catalog = server.describe_operations().unwrap().0;
        assert!(catalog.type_script.contains(r#""type": "renameDocument""#));
        assert!(
            catalog
                .type_script
                .contains(r#""type": "setFramePipeline""#)
        );

        let receipt = server
            .apply_operation(Parameters(ApplyOperationArgs {
                operation: serde_json::json!({
                    "type": "renameDocument",
                    "name": "Named through the operation API"
                }),
                expected_revision: Some(0),
            }))
            .unwrap()
            .0;
        assert_eq!(receipt.revision, 1);
        assert_eq!(
            server.inspect_document().unwrap().0.name,
            "Named through the operation API"
        );
        if path.exists() {
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn generic_operation_rejects_unknown_variants_without_writing() {
        let (server, path) = test_server();
        let result = server.apply_operation(Parameters(ApplyOperationArgs {
            operation: serde_json::json!({ "type": "inventedOperation" }),
            expected_revision: Some(0),
        }));
        let error = match result {
            Ok(_) => panic!("an unknown operation must not be accepted"),
            Err(error) => error,
        };
        assert!(error.contains("Invalid FrameWork operation"));
        assert_eq!(server.inspect_document().unwrap().0.revision, 0);
        if path.exists() {
            std::fs::remove_file(path).unwrap();
        }
    }
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Tests run in parallel threads and the clock is coarser than a nanosecond,
    /// so the timestamp alone can repeat; the counter keeps each path distinct.
    fn unique_temp_path(name: &str) -> PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = COUNTER.fetch_add(1, Ordering::Relaxed);
        let process = std::process::id();
        env::temp_dir().join(format!("{name}-{process}-{timestamp}-{sequence}"))
    }

    fn test_server() -> (FrameworkMcp, PathBuf) {
        let path = unique_temp_path("framework-mcp").with_extension("json");
        (FrameworkMcp::open(path.clone()).unwrap(), path)
    }

    #[test]
    fn exposes_semantic_document_and_frame_snapshots() {
        let (server, _) = test_server();
        let summary = server.inspect_document().unwrap().0;
        assert_eq!(summary.objects.len(), 7);
        // The catalog no longer rides along — it drowned the answer. The
        // pointer to search_functions is the whole formula story here.
        assert!(summary.formula_reference.contains("search_functions"));
        let frame = summary
            .objects
            .iter()
            .find(|object| object.kind == "frame" && object.name == "Orders")
            .unwrap();
        let snapshot = server
            .get_frame(Parameters(GetFrameArgs {
                frame: frame.id.clone(),
                limit: Some(2),
            }))
            .unwrap()
            .0;
        assert_eq!(snapshot.returned_row_count, 2);
        assert_eq!(snapshot.total_row_count, 3);
        assert!(
            snapshot
                .columns
                .iter()
                .any(|column| column.formula.is_some())
        );
    }

    #[test]
    fn writes_persist_and_revision_guards_reject_stale_clients() {
        let (server, path) = test_server();
        let initial = server.inspect_document().unwrap().0;
        let created = server
            .create_block(Parameters(CreateBlockArgs {
                name: "Assumptions".into(),
                x: None,
                y: None,
                expected_revision: Some(initial.revision),
            }))
            .unwrap()
            .0;
        assert_eq!(created.revision, initial.revision + 1);
        assert!(created.affected_object_id.is_some());
        assert!(path.exists());

        let stale = server.create_block(Parameters(CreateBlockArgs {
            name: "Stale".into(),
            x: None,
            y: None,
            expected_revision: Some(initial.revision),
        }));
        assert!(matches!(stale, Err(error) if error.contains("Revision conflict")));

        let undone = server
            .undo(Parameters(HistoryArgs {
                expected_revision: Some(created.revision),
            }))
            .unwrap()
            .0;
        assert_eq!(undone.revision, created.revision + 1);
        let stale_after_undo = server.create_block(Parameters(CreateBlockArgs {
            name: "Still stale".into(),
            x: None,
            y: None,
            expected_revision: Some(initial.revision),
        }));
        assert!(matches!(stale_after_undo, Err(error) if error.contains("Revision conflict")));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn fw_servers_exchange_mutations_through_the_event_journal() {
        let directory = unique_temp_path("framework-mcp-events");
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("Shared.fw");
        let initial_store = Store::new(Document::demo());
        let document_id = initial_store.view().document.id.clone();
        initial_store.save(&path).unwrap();
        let first = FrameworkMcp::open(path.clone()).unwrap();
        let second = FrameworkMcp::open(path.clone()).unwrap();

        first
            .create_block(Parameters(CreateBlockArgs {
                name: "From first writer".into(),
                x: None,
                y: None,
                expected_revision: Some(0),
            }))
            .unwrap();
        let second_view = second.inspect_document().unwrap().0;
        assert!(
            second_view
                .objects
                .iter()
                .any(|object| object.name == "From first writer")
        );

        let collaboration =
            framework_core::CollaborationPaths::for_document(&path, &document_id).unwrap();
        assert!(
            std::fs::read_dir(collaboration.events)
                .unwrap()
                .any(|entry| entry.unwrap().path().is_dir())
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn calculated_columns_accept_exact_names_and_return_stable_ids() {
        let (server, path) = test_server();
        let block = server
            .create_block(Parameters(CreateBlockArgs {
                name: "Overrides".into(),
                x: None,
                y: None,
                expected_revision: Some(0),
            }))
            .unwrap()
            .0;
        let created = server
            .set_block_source(Parameters(SetBlockSourceArgs {
                block: "Overrides".into(),
                source: "Safety Factor = 1.7".into(),
                expected_revision: Some(block.revision),
            }))
            .unwrap()
            .0;
        let result = server
            .add_calculated_column(Parameters(AddCalculatedColumnArgs {
                frame: "Orders".into(),
                name: "Adjusted".into(),
                formula: "`Total` * `Overrides`.`Safety Factor`".into(),
                expected_revision: Some(created.revision),
            }))
            .unwrap()
            .0;
        assert!(result.affected_column_id.is_some());
        let frame = server
            .get_frame(Parameters(GetFrameArgs {
                frame: "Orders".into(),
                limit: None,
            }))
            .unwrap()
            .0;
        assert!(
            frame
                .columns
                .iter()
                .any(|column| column.name == "Adjusted" && column.formula.is_none())
        );
        let inspected = server.lock().unwrap().store.view();
        let orders =
            frame_by_id(&inspected, result.affected_object_id.as_deref().unwrap()).unwrap();
        assert!(orders.steps.iter().any(|step| matches!(
            step,
            framework_core::FrameStep::WithColumns { columns }
                if columns.iter().any(|column| {
                    Some(&column.output_column_id) == result.affected_column_id.as_ref()
                })
        )));
        drop(inspected);
        let chained = server
            .add_calculated_column(Parameters(AddCalculatedColumnArgs {
                frame: "Orders".into(),
                name: "Adjusted twice".into(),
                formula: "`Adjusted` * 2".into(),
                expected_revision: Some(result.revision),
            }))
            .unwrap()
            .0;
        let inspected = server.lock().unwrap().store.view();
        let orders =
            frame_by_id(&inspected, chained.affected_object_id.as_deref().unwrap()).unwrap();
        assert_eq!(
            orders
                .steps
                .iter()
                .filter(|step| matches!(step, framework_core::FrameStep::WithColumns { .. }))
                .count(),
            2
        );
        drop(inspected);
        let generic = server
            .apply_operation(Parameters(ApplyOperationArgs {
                operation: serde_json::json!({
                    "type": "addComputedColumn",
                    "frameId": chained.affected_object_id.as_deref().unwrap(),
                    "name": "Generic adjusted",
                    "formula": "`Adjusted twice` + 1"
                }),
                expected_revision: Some(chained.revision),
            }))
            .unwrap()
            .0;
        let inspected = server.lock().unwrap().store.view();
        let orders =
            frame_by_id(&inspected, generic.affected_object_id.as_deref().unwrap()).unwrap();
        assert_eq!(
            orders
                .steps
                .iter()
                .filter(|step| matches!(step, framework_core::FrameStep::WithColumns { .. }))
                .count(),
            3,
            "the generated operation escape hatch must also write through Wrangle"
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn literal_frame_structure_tools_use_semantic_names() {
        let (server, path) = test_server();
        let column = server
            .add_literal_column(Parameters(AddLiteralColumnArgs {
                frame: "Orders".into(),
                name: "Notes".into(),
                data_type: "text".into(),
                after_column: Some("Quantity".into()),
                expected_revision: Some(0),
            }))
            .unwrap()
            .0;
        let typed = server
            .set_column_type(Parameters(SetColumnTypeArgs {
                frame: "Orders".into(),
                column: "Notes".into(),
                data_type: "number".into(),
                expected_revision: Some(column.revision),
            }))
            .unwrap()
            .0;
        let row = server
            .add_row(Parameters(AddRowArgs {
                frame: "Orders".into(),
                values: BTreeMap::from([("Notes".into(), "42".into())]),
                expected_revision: Some(typed.revision),
            }))
            .unwrap()
            .0;
        assert!(row.affected_row_id.is_some());

        let frame = server
            .get_frame(Parameters(GetFrameArgs {
                frame: "Orders".into(),
                limit: None,
            }))
            .unwrap()
            .0;
        assert_eq!(frame.total_row_count, 4);
        assert!(
            frame
                .columns
                .iter()
                .any(|column| column.name == "Notes" && column.data_type == "number")
        );
        assert!(
            frame
                .rows
                .last()
                .unwrap()
                .cells
                .iter()
                .any(|cell| { cell.column_name == "Notes" && cell.raw == "42" })
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn deletion_tools_resolve_semantic_names_and_can_be_undone() {
        let (server, path) = test_server();
        let added = server
            .add_literal_column(Parameters(AddLiteralColumnArgs {
                frame: "Orders".into(),
                name: "Notes".into(),
                data_type: "text".into(),
                after_column: None,
                expected_revision: Some(0),
            }))
            .unwrap()
            .0;
        let deleted_column = server
            .delete_column(Parameters(DeleteColumnArgs {
                frame: "Orders".into(),
                column: "Notes".into(),
                expected_revision: Some(added.revision),
            }))
            .unwrap()
            .0;
        assert!(
            server
                .get_frame(Parameters(GetFrameArgs {
                    frame: "Orders".into(),
                    limit: None,
                }))
                .unwrap()
                .0
                .columns
                .iter()
                .all(|column| column.name != "Notes")
        );

        server
            .undo(Parameters(HistoryArgs {
                expected_revision: Some(deleted_column.revision),
            }))
            .unwrap();
        assert!(
            server
                .get_frame(Parameters(GetFrameArgs {
                    frame: "Orders".into(),
                    limit: None,
                }))
                .unwrap()
                .0
                .columns
                .iter()
                .any(|column| column.name == "Notes")
        );

        let protected = server.delete_column(Parameters(DeleteColumnArgs {
            frame: "Orders".into(),
            column: "Quantity".into(),
            expected_revision: None,
        }));
        // Named, so an agent reading this can act on it rather than guess.
        let Err(refusal) = protected else {
            panic!("a column another formula reads cannot be deleted");
        };
        assert!(refusal.contains("‘Quantity’"), "{refusal}");
        assert!(refusal.contains("‘Orders’"), "{refusal}");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn date_and_null_values_are_exposed_as_typed_api_values() {
        let (server, path) = test_server();
        let created = server
            .create_frame(Parameters(CreateFrameArgs {
                name: "Schedule".into(),
                grid: vec![
                    vec!["Start".into(), "Fallback".into()],
                    vec!["2026-08-11".into(), "2026-08-12".into()],
                    vec!["".into(), "2026-09-01".into()],
                ],
                x: None,
                y: None,
                expected_revision: Some(0),
            }))
            .unwrap()
            .0;
        server
            .add_calculated_column(Parameters(AddCalculatedColumnArgs {
                frame: "Schedule".into(),
                name: "Effective".into(),
                formula: "coalesce(`Start`, `Fallback`)".into(),
                expected_revision: Some(created.revision),
            }))
            .unwrap();

        let frame = server
            .get_frame(Parameters(GetFrameArgs {
                frame: "Schedule".into(),
                limit: None,
            }))
            .unwrap()
            .0;
        assert_eq!(frame.columns[0].data_type, "date");
        assert_eq!(
            frame.rows[0].cells[0].typed_value,
            ApiScalarValue::Date("2026-08-11".into())
        );
        assert_eq!(frame.rows[1].cells[0].typed_value, ApiScalarValue::Null);
        assert_eq!(
            frame.rows[1].cells[2].typed_value,
            ApiScalarValue::Date("2026-09-01".into())
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn refuses_to_replace_a_malformed_existing_document() {
        let path = unique_temp_path("framework-mcp-invalid").with_extension("json");
        std::fs::write(&path, "not valid json").unwrap();
        let result = FrameworkMcp::open(path.clone());
        assert!(matches!(result, Err(error) if error.contains("Could not load document")));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "not valid json");
        std::fs::remove_file(path).unwrap();
    }
}
