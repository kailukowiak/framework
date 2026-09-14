//! Resolving `Operation`s in this family into fully determined
//! `ReplicatedOperation`s: IDs minted, formula names bound to column IDs, and
//! every precondition checked before anything is applied.
//!
//! Whole-object lifecycle: adding, importing, renaming, and deleting the
//! values, frames, plots, and text a document holds.

use crate::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use ts_rs::TS;

/// What a refresh or a repoint did to a frame's schema, in the words the
/// person reading it uses: column display names, not ids.
///
/// Deliberately not stored. History records the columns a refresh produced,
/// which is the durable fact; *how they differ from the ones before* is a
/// sentence about one moment, useful when the refresh lands and misleading
/// forever after — a document reopened next week would otherwise still be
/// announcing that Region arrived.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SchemaDiff {
    /// Fields the source now supplies that it did not before.
    pub added: Vec<String>,
    /// Columns dropped because their field went away and nothing read them.
    pub removed: Vec<String>,
    /// Columns whose field went away but which something still reads, with
    /// what reads them. These keep their id and show nulls rather than
    /// disappearing out from under a formula.
    pub kept_missing: Vec<(String, String)>,
    /// Columns the source still supplies under a different type.
    pub type_changed: Vec<(String, DataType, DataType)>,
}

impl SchemaDiff {
    /// Whether the refresh changed nothing about the shape of the data —
    /// the ordinary case, and the one that should say nothing at all.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.removed.is_empty()
            && self.kept_missing.is_empty()
            && self.type_changed.is_empty()
    }
}

/// A name field accepts the spelling people see in formulas, but quoting is
/// syntax rather than part of the name. This is kept at the operation boundary
/// so UI, MCP, and future authoring surfaces cannot accidentally turn
/// `` `my variable` `` into a name whose rendered token ends in three ticks.
fn variable_name_from_input(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('`') && trimmed.ends_with('`') {
        trimmed[1..trimmed.len() - 1].replace("``", "`")
    } else {
        trimmed.to_string()
    }
}

impl Document {
    /// A compact formula surface: small enough for a single assumption, but
    /// still a real live expression rather than a raw field with a second set
    /// of parsing rules. Lists are allowed because this is the compact cousin
    /// of Scratchwork, whose line may answer with either one value or many.
    pub(crate) fn prepare_add_variable(
        &self,
        name: String,
        formula: String,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        let expression = self.parse_formula_rule(&formula)?;
        expression.validate_list_placement(self, true)?;
        let object_id = id();
        Ok(ReplicatedOperation::AddObject {
            object: DataObject::Result(ResultObject {
                id: object_id.clone(),
                name: variable_name_from_input(&name),
                formula: Formula { expression },
                variable: true,
            }),
            view: CanvasView {
                id: id(),
                object_id,
                x,
                y,
                width: 320.0,
                height: 46.0,
                collapsed: false,
                tab_object_ids: Vec::new(),
            },
            container_id: None,
        })
    }

    /// Where the old card-shaped value, result, or list is allowed to live:
    /// not on the bare canvas. `AddVariable` above is the compact exception.
    ///
    /// A single number sitting on the canvas is a card that says `4.25%` and
    /// nothing else, and forty of them is the density problem the formula
    /// block exists to answer. A block line does the same job in one row of
    /// the screen, with a name, an editable formula, and its answer beside
    /// it — so a constant is written the way a formula is written, and the
    /// two stop being different kinds of thing.
    ///
    /// A container is the exception, because there a value is part of an
    /// arrangement somebody laid out rather than a card that drifted loose.
    /// That is the dashboard case, and it is the only one left.
    fn refuse_on_the_canvas(container_id: &Option<Id>, what: &str) -> Result<(), CoreError> {
        match container_id {
            Some(_) => Ok(()),
            None => Err(CoreError::InvalidOperation(format!(
                "{what} belongs on a line of a formula block, where it can be named and \
                 read from anywhere, or inside a container. The canvas itself holds compact \
                 variables, frames, blocks, and containers."
            ))),
        }
    }

    pub(crate) fn prepare_add_value(
        &self,
        name: String,
        raw: String,
        x: f64,
        y: f64,
        container_id: Option<Id>,
    ) -> Result<ReplicatedOperation, CoreError> {
        Self::refuse_on_the_canvas(&container_id, "A value")?;
        Ok({
            let object_id = id();
            ReplicatedOperation::AddObject {
                object: DataObject::Value(ValueObject {
                    id: object_id.clone(),
                    name,
                    data_type: infer_data_type(&raw),
                    raw,
                }),
                view: CanvasView {
                    id: id(),
                    object_id,
                    x,
                    y,
                    width: 220.0,
                    height: 116.0,
                    collapsed: false,
                    tab_object_ids: Vec::new(),
                },
                container_id,
            }
        })
    }

    pub(crate) fn prepare_add_container(
        &self,
        name: String,
        x: f64,
        y: f64,
        container_id: Option<Id>,
    ) -> Result<ReplicatedOperation, CoreError> {
        let object_id = id();
        Ok(ReplicatedOperation::AddObject {
            object: DataObject::Container(ContainerObject {
                id: object_id.clone(),
                name,
                member_ids: Vec::new(),
            }),
            view: CanvasView {
                id: id(),
                object_id,
                x,
                y,
                width: 260.0,
                height: 240.0,
                collapsed: false,
                tab_object_ids: Vec::new(),
            },
            container_id,
        })
    }

    /// Moving an object into a container, or out of one.
    ///
    /// Both containers involved are named in the result — the one taking it
    /// and the one losing it — so applying this is a straight assignment and
    /// no replica has to work out where the object used to be.
    pub(crate) fn prepare_move_into_container(
        &self,
        object_id: Id,
        container_id: Option<Id>,
    ) -> Result<ReplicatedOperation, CoreError> {
        let object = self.object(&object_id)?;
        if !matches!(
            object,
            DataObject::Value(_)
                | DataObject::Result(_)
                | DataObject::Series(_)
                | DataObject::Container(_)
        ) {
            return Err(CoreError::InvalidOperation(
                "Only a value, a result, a list, or another container can go in a container".into(),
            ));
        }
        // Taking an old value, dashboard result, or stored list out of a
        // container puts it on the canvas, where those full cards do not
        // belong. A compact variable is deliberately the exception: the
        // canvas is its home, whether it has briefly been grouped or not.
        if !matches!(
            object,
            DataObject::Container(_) | DataObject::Result(ResultObject { variable: true, .. })
        ) {
            Self::refuse_on_the_canvas(&container_id, "A value, a result, or a list")?;
        }
        if let Some(container_id) = &container_id {
            if !matches!(self.object(container_id)?, DataObject::Container(_)) {
                return Err(CoreError::InvalidOperation(
                    "That is not a container".into(),
                ));
            }
            // A container inside itself, at any depth, is a shape with no
            // bottom: nothing could draw it and no name could resolve
            // through it.
            if *container_id == object_id || self.container_holds(&object_id, container_id) {
                return Err(CoreError::InvalidOperation(
                    "A container cannot be put inside itself".into(),
                ));
            }
        }
        let mut members: Vec<(Id, Vec<Id>)> = Vec::new();
        if let Some(previous) = self.container_of(&object_id) {
            if Some(&previous.id) == container_id.as_ref() {
                // Already where it is being sent. Recording the same list
                // back is a no-op rather than an error: dropping something
                // where it already is is not a mistake.
                return Ok(ReplicatedOperation::SetContainerMembers {
                    members: Vec::new(),
                });
            }
            members.push((
                previous.id.clone(),
                previous
                    .member_ids
                    .iter()
                    .filter(|member| **member != object_id)
                    .cloned()
                    .collect(),
            ));
        }
        if let Some(container_id) = container_id {
            let DataObject::Container(container) = self.object(&container_id)? else {
                unreachable!("checked above");
            };
            let mut next = container.member_ids.clone();
            next.push(object_id);
            members.push((container_id, next));
        }
        Ok(ReplicatedOperation::SetContainerMembers { members })
    }

    pub(crate) fn prepare_add_result(
        &self,
        name: String,
        formula: String,
        x: f64,
        y: f64,
        container_id: Option<Id>,
    ) -> Result<ReplicatedOperation, CoreError> {
        Self::refuse_on_the_canvas(&container_id, "A result")?;
        let expression = self.parse_formula_scalar(&formula)?;
        expression.validate_list_placement(self, false)?;
        let object_id = id();
        Ok(ReplicatedOperation::AddObject {
            object: DataObject::Result(ResultObject {
                id: object_id.clone(),
                name,
                formula: Formula { expression },
                variable: false,
            }),
            view: CanvasView {
                id: id(),
                object_id,
                x,
                y,
                width: 220.0,
                height: 132.0,
                collapsed: false,
                tab_object_ids: Vec::new(),
            },
            container_id,
        })
    }

    /// A text card: an empty prose surface at the given spot, waiting to be
    /// typed into, the way a block starts empty.
    pub(crate) fn prepare_add_text(
        &self,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        let object_id = id();
        Ok(ReplicatedOperation::AddObject {
            object: DataObject::Text(TextObject {
                id: object_id.clone(),
                name: "Text".into(),
                text: String::new(),
                segments: Vec::new(),
            }),
            view: CanvasView {
                id: id(),
                object_id,
                x,
                y,
                width: 480.0,
                height: 280.0,
                collapsed: false,
                tab_object_ids: Vec::new(),
            },
            container_id: None,
        })
    }

    /// The card retyped: markdown split at its `{{…}}` holes, each hole
    /// parsed as a scalar formula against the document as it stands.
    ///
    /// Nothing here refuses. A hole that does not parse — or parses to a
    /// list, which has no single answer to print — is stored broken with
    /// its complaint, shown where it sits, and fixed by typing: the same
    /// tolerance a formula block extends to a line it could not read.
    pub(crate) fn prepare_set_text_source(
        &self,
        object_id: Id,
        source: String,
    ) -> Result<ReplicatedOperation, CoreError> {
        self.text_object(&object_id)?;
        Ok(ReplicatedOperation::SetTextSegments {
            object_id,
            segments: self.parse_text_segments(&source),
        })
    }

    fn parse_text_segments(&self, source: &str) -> Vec<TextSegment> {
        let mut segments = Vec::new();
        let mut literal = String::new();
        let mut rest = source;
        while let Some(start) = next_markdown_formula_hole(rest) {
            // An unclosed hole is still prose — half-typed braces should
            // read as what they are until the closing pair arrives.
            let Some(length) = rest[start + 2..].find("}}") else {
                break;
            };
            literal.push_str(&rest[..start]);
            let inner = &rest[start + 2..start + 2 + length];
            rest = &rest[start + 2 + length + 2..];
            if !literal.is_empty() {
                segments.push(TextSegment::Literal {
                    text: std::mem::take(&mut literal),
                });
            }
            let parsed = self.parse_formula_scalar(inner).and_then(|expression| {
                expression.validate_list_placement(self, false)?;
                Ok(expression)
            });
            segments.push(match parsed {
                Ok(expression) => TextSegment::Formula {
                    formula: Formula { expression },
                },
                Err(error) => TextSegment::Broken {
                    source: inner.to_string(),
                    error: error.to_string(),
                },
            });
        }
        literal.push_str(rest);
        if !literal.is_empty() {
            segments.push(TextSegment::Literal { text: literal });
        }
        segments
    }

    pub(crate) fn prepare_set_result_formula(
        &self,
        object_id: Id,
        formula: String,
    ) -> Result<ReplicatedOperation, CoreError> {
        let DataObject::Result(result) = self.object(&object_id)? else {
            return Err(CoreError::ObjectNotFound);
        };
        let expression = if result.variable {
            self.parse_formula_rule(&formula)?
        } else {
            self.parse_formula_scalar(&formula)?
        };
        expression.validate_list_placement(self, result.variable)?;
        // The one shape a formula that only reads by id can still tie: a
        // result reaching itself through other results. Refused here, which
        // is what lets compilation recurse without watching its own feet.
        if self.formula_reaches_object(&expression, &object_id) {
            return Err(CoreError::Formula(format!(
                "This formula reads ‘{}’, so it would be defined in terms of itself.",
                result.name
            )));
        }
        Ok(ReplicatedOperation::SetResultFormula {
            object_id,
            formula: Formula { expression },
        })
    }

    pub(crate) fn prepare_add_series(
        &self,
        name: String,
        values: String,
        x: f64,
        y: f64,
        container_id: Option<Id>,
    ) -> Result<ReplicatedOperation, CoreError> {
        Self::refuse_on_the_canvas(&container_id, "A list")?;
        let values = parse_list_text(&values);
        if values.is_empty() {
            return Err(CoreError::InvalidOperation(
                "A list needs at least one value".into(),
            ));
        }
        let object_id = id();
        Ok(ReplicatedOperation::AddObject {
            object: DataObject::Series(SeriesObject {
                id: object_id.clone(),
                name,
                data_type: infer_list_type(&values),
                values,
            }),
            view: CanvasView {
                id: id(),
                object_id,
                x,
                y,
                width: 240.0,
                height: 220.0,
                collapsed: false,
                tab_object_ids: Vec::new(),
            },
            container_id,
        })
    }

    /// One column of a file, as a list.
    ///
    /// The same reader every import uses, so a list arrives from a CSV or a
    /// parquet by the path already trusted to read them — and the column's
    /// own type comes with it, rather than being guessed back out of the
    /// text it was printed to.
    pub(crate) fn prepare_import_series_from_file(
        &self,
        name: String,
        path: String,
        column: Option<String>,
        x: f64,
        y: f64,
        container_id: Option<Id>,
    ) -> Result<ReplicatedOperation, CoreError> {
        Self::refuse_on_the_canvas(&container_id, "A list")?;
        let frame = read_import_frame(Path::new(&path))?;
        let source = match &column {
            Some(column) => frame.column(column.as_str()).map_err(|_| {
                CoreError::Import(format!("‘{column}’ is not a column of that file"))
            })?,
            None => frame
                .columns()
                .first()
                .ok_or_else(|| CoreError::Import("That file has no columns".into()))?,
        };
        let series = source.as_materialized_series();
        let data_type = framework_type_from_polars(series.dtype()).unwrap_or(DataType::String);
        let values = (0..series.len())
            .map(|index| {
                polars_value_at(series, index)
                    .map(scalar_value_to_raw)
                    .map_err(CoreError::Import)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if values.is_empty() {
            return Err(CoreError::Import("That column has no values".into()));
        }
        let object_id = id();
        Ok(ReplicatedOperation::AddObject {
            object: DataObject::Series(SeriesObject {
                id: object_id.clone(),
                name,
                data_type,
                values,
            }),
            view: CanvasView {
                id: id(),
                object_id,
                x,
                y,
                width: 240.0,
                height: 220.0,
                collapsed: false,
                tab_object_ids: Vec::new(),
            },
            container_id,
        })
    }

    pub(crate) fn prepare_set_series(
        &self,
        object_id: Id,
        values: String,
    ) -> Result<ReplicatedOperation, CoreError> {
        let values = parse_list_text(&values);
        if values.is_empty() {
            return Err(CoreError::InvalidOperation(
                "A list needs at least one value".into(),
            ));
        }
        Ok(ReplicatedOperation::SetSeries {
            object_id,
            data_type: infer_list_type(&values),
            values,
        })
    }

    pub(crate) fn prepare_add_frame(
        &self,
        name: String,
        grid: Vec<Vec<String>>,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            let name = self.unique_frame_name(&name, None);
            let (frame, view) = Self::build_frame(name, grid, x, y);
            ReplicatedOperation::AddObject {
                object: DataObject::Frame(frame),
                view,
                container_id: None,
            }
        })
    }

    /// The clipboard version of `prepare_add_frame`: the text is read the way
    /// a paste into an empty frame is, so a pasted column of dates becomes a
    /// date column here for the same reason.
    pub(crate) fn prepare_add_frame_from_pasted_text(
        &self,
        name: String,
        text: String,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        let name = self.unique_frame_name(&name, None);
        let (frame, view) = Self::build_frame_from_pasted_text(name, &text, x, y)?;
        Ok(ReplicatedOperation::AddObject {
            object: DataObject::Frame(frame),
            view,
            container_id: None,
        })
    }

    /// Resolves a generated frame: the rule parsed in scalar scope, run once
    /// to learn what it makes, and refused if it cannot run at all — a
    /// generator that has never produced rows is a typo, not a frame.
    pub(crate) fn prepare_add_generator_frame(
        &self,
        name: String,
        formula: String,
        column_name: Option<String>,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        let name = self.unique_frame_name(&name, None);
        let expression = self.parse_formula_rule(&formula)?;
        self.validate_generator_rule(&expression)?;
        let (data_type, _) = self
            .evaluate_rule_series(&expression)
            .map_err(CoreError::Formula)?;
        let column_name = column_name
            .filter(|column_name| !column_name.trim().is_empty())
            .unwrap_or_else(|| name.clone());
        let frame_id = id();
        let frame = FrameObject {
            id: frame_id.clone(),
            name,
            columns: vec![Column {
                id: column_id(&column_name),
                name: column_name,
                source_name: None,
                data_type,
                scale: None,
                categories: Vec::new(),
                format: None,
                formula: None,
            }],
            generator: Some(FrameGenerator {
                formula: Formula { expression },
            }),
            ..FrameObject::default()
        };
        let view = CanvasView {
            id: id(),
            object_id: frame_id,
            x,
            y,
            // One column of small values: the narrow end of the frame-card
            // range, resized like any card if the rule grows.
            width: 260.0,
            height: 300.0,
            collapsed: false,
            tab_object_ids: Vec::new(),
        };
        Ok(ReplicatedOperation::AddObject {
            object: DataObject::Frame(frame),
            view,
            container_id: None,
        })
    }

    /// Resolves a rule replacement: same parsing and same trial run as
    /// creation, with the column's type following the new rule so a
    /// day-offset generator rewritten as a date range *becomes* dates.
    pub(crate) fn prepare_set_frame_generator(
        &self,
        frame_id: Id,
        formula: String,
    ) -> Result<ReplicatedOperation, CoreError> {
        let frame = self.frame(&frame_id)?;
        if frame.generator.is_none() {
            return Err(CoreError::InvalidOperation(
                "Only a generated frame has a rule to replace".into(),
            ));
        }
        let expression = self.parse_formula_rule(&formula)?;
        self.validate_generator_rule(&expression)?;
        let (data_type, _) = self
            .evaluate_rule_series(&expression)
            .map_err(CoreError::Formula)?;
        let mut columns = frame.columns.clone();
        let first = columns.first_mut().ok_or_else(|| {
            CoreError::InvalidOperation("A generated frame needs a column to fill".into())
        })?;
        first.data_type = data_type;
        Ok(ReplicatedOperation::SetFrameGenerator {
            frame_id,
            generator: FrameGenerator {
                formula: Formula { expression },
            },
            columns,
        })
    }

    pub(crate) fn prepare_import_frame_from_file(
        &self,
        name: String,
        path: String,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            let name = self.unique_frame_name(&name, None);
            let (frame, view) = Self::build_imported_frame(name, Path::new(&path), x, y)?;
            ReplicatedOperation::AddObject {
                object: DataObject::Frame(frame),
                view,
                container_id: None,
            }
        })
    }

    pub(crate) fn prepare_import_frame_from_artifact(
        &self,
        name: String,
        artifact: DataArtifact,
        connector: Option<ConnectorRecipe>,
        file_origin: Option<String>,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            let name = self.unique_frame_name(&name, None);
            let (mut frame, view) = Self::build_artifact_frame(name, artifact, connector, x, y)?;
            // Resolved here rather than by the caller because the binding it
            // records is between the file's fields and *this frame's* columns,
            // and those column ids were minted one line ago.
            if let Some(path) = file_origin {
                frame.file_origin = Some(Self::delimited_file_origin(&path, &frame.columns)?);
            }
            ReplicatedOperation::AddObject {
                object: DataObject::Frame(frame),
                view,
                container_id: None,
            }
        })
    }

    /// A plot either gets a window of its own or joins the card that already
    /// shows the frame it draws.
    ///
    /// The second is the same edit as branching a tab, because it is the
    /// same thing: another rendering of data the card already holds. Which
    /// one you get is the only difference the caller decides.
    pub(crate) fn prepare_add_plot(
        &self,
        name: String,
        source_frame_id: Id,
        spec: serde_json::Value,
        x: f64,
        y: f64,
        view_id: Option<Id>,
    ) -> Result<ReplicatedOperation, CoreError> {
        self.frame(&source_frame_id)?;
        let object_id = id();
        let plot = DataObject::Plot(PlotObject {
            id: object_id.clone(),
            name,
            source_frame_id,
            spec,
        });
        let Some(view_id) = view_id else {
            return Ok(ReplicatedOperation::AddObject {
                object: plot,
                view: CanvasView {
                    id: id(),
                    object_id,
                    x,
                    y,
                    width: 640.0,
                    height: 430.0,
                    collapsed: false,
                    tab_object_ids: Vec::new(),
                },
                container_id: None,
            });
        };
        self.view(&view_id)?;
        Ok(ReplicatedOperation::AddTab {
            view_id,
            object: plot,
        })
    }

    pub(crate) fn prepare_rename_object(
        &self,
        object_id: Id,
        name: String,
    ) -> Result<ReplicatedOperation, CoreError> {
        let name = match self.object(&object_id)? {
            DataObject::Frame(_) => self.unique_frame_name(&name, Some(&object_id)),
            DataObject::Result(result) if result.variable => variable_name_from_input(&name),
            _ => name,
        };
        let blocks = self.blocks_renamed_by(&object_id, &name)?;
        Ok(ReplicatedOperation::RenameObject {
            object_id,
            name,
            blocks,
        })
    }

    /// The block lines a rename has to rewrite, already rewritten.
    ///
    /// Which lines those are is settled by rendering each one twice — once
    /// against this document and once against a copy carrying the new name —
    /// and keeping the ones that came out differently. That asks the renderer
    /// the question directly, rather than re-deriving from the AST which
    /// names a line happens to use; the renderer is the thing that decides,
    /// so it is the thing worth asking. The cost is one document clone per
    /// rename, which holds schemas rather than rows.
    ///
    /// A rewritten line comes back canonically spelled, spacing and all. That
    /// is already true of a line rewritten by renaming a sibling — see
    /// `prepare_set_block_source` — and it is confined to lines that actually
    /// named the renamed thing.
    fn blocks_renamed_by(
        &self,
        object_id: &Id,
        name: &str,
    ) -> Result<Vec<(Id, Vec<BlockLine>)>, CoreError> {
        // Renaming something to what it is already called changes no text,
        // and a rename of something that is not here is somebody else's
        // error to report.
        if self
            .object(object_id)
            .is_ok_and(|object| object.name() == name)
        {
            return Ok(Vec::new());
        }
        let mut renamed = self.clone();
        renamed.apply_rename_object(object_id.clone(), name.to_string())?;
        let empty = FrameObject::default();
        Ok(self
            .objects
            .iter()
            .filter_map(|object| match object {
                DataObject::Block(block) => Some(block),
                _ => None,
            })
            .filter_map(|block| {
                let after = renamed.block(&block.id).ok()?;
                let mut lines = block.lines.clone();
                let mut touched = false;
                for line in &mut lines {
                    let Some(expression) = line.expression() else {
                        continue;
                    };
                    let was = expression.render_in_scope(&empty, self, Some(block), 0);
                    let now = expression.render_in_scope(&empty, &renamed, Some(after), 0);
                    if was == now {
                        continue;
                    }
                    line.source = now;
                    touched = true;
                }
                touched.then(|| (block.id.clone(), lines))
            })
            .collect())
    }

    pub(crate) fn prepare_delete_object(
        &self,
        object_id: Id,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok(ReplicatedOperation::DeleteObject { object_id })
    }

    pub(crate) fn prepare_set_value(
        &self,
        object_id: Id,
        raw: String,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok(ReplicatedOperation::SetValue { object_id, raw })
    }

    pub(crate) fn prepare_set_plot_spec(
        &self,
        plot_id: Id,
        spec: serde_json::Value,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok(ReplicatedOperation::SetPlotSpec { plot_id, spec })
    }

    pub(crate) fn prepare_rename_document(
        &self,
        name: String,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            let name = name.trim().to_string();
            if name.is_empty() {
                return Err(CoreError::InvalidOperation(
                    "Document name cannot be empty".into(),
                ));
            }
            ReplicatedOperation::RenameDocument { name }
        })
    }
}

/// The prose card's formula holes are Markdown extensions, so Markdown code
/// wins when the two syntaxes meet. A tutorial must be able to show someone
/// the literal spelling `` `{{formula}}` `` or put it in a fenced example
/// without executing the example against the workbook it is explaining.
///
/// This is deliberately only the opening-hole search. Once a real hole begins,
/// its closing braces belong to the formula extension; formula text has its own
/// string and backtick parser and does not need a second Markdown interpretation.
fn next_markdown_formula_hole(source: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut index = 0;
    let mut fenced = false;
    let mut inline = false;
    while index < bytes.len() {
        if bytes[index] == b'`' {
            let start = index;
            while index < bytes.len() && bytes[index] == b'`' {
                index += 1;
            }
            let run = index - start;
            if run >= 3 && !inline {
                fenced = !fenced;
            } else if run == 1 && !fenced {
                inline = !inline;
            }
            continue;
        }
        if !fenced && !inline && bytes[index] == b'{' && bytes.get(index + 1) == Some(&b'{') {
            return Some(index);
        }
        index += 1;
    }
    None
}
