//! `FrameObject`'s own behaviour: dependency layers, formula rendering, and
//! turning a frame into the Polars frame the plan starts from.

use crate::*;
use polars::prelude as pl;
use polars::prelude::{IntoLazy, NamedFrom};
use std::collections::{HashMap, HashSet};
use std::path::Path;

impl FrameObject {
    /// The columns the frame's own data provides -- what the base scan
    /// reads and what a chain's first step sees.
    ///
    /// Without a chain this is just `columns`: the same list describes the
    /// data and the display, which is why documents written before chains
    /// existed need no migration.
    pub fn input_columns(&self) -> &[Column] {
        if self.base_columns.is_empty() {
            &self.columns
        } else {
            &self.base_columns
        }
    }

    /// Whether the values on screen are computed rather than stored.
    ///
    /// Editing a cell, adding a column, or deleting one all mean editing an
    /// *input*, and a computed frame's columns are outputs -- so the same
    /// guard that has always protected derived frames protects a frame with
    /// a chain of its own.
    pub(crate) fn is_computed(&self) -> bool {
        self.derivation.is_some()
            || self.generator.is_some()
            || self.steps.iter().any(|step| {
                !matches!(
                    step,
                    FrameStep::Filter { .. } | FrameStep::Sort { .. } | FrameStep::Comment { .. }
                )
            })
    }

    /// A chain rendered back to the text that was written, for the editor.
    ///
    /// `input_columns` is what the chain starts from. Each step's formulas
    /// were written against the schema at its *own* position, so rendering
    /// them against that starting schema alone would print `#REF` for every
    /// reference to a column an earlier step added. Column ids are unique,
    /// so one scope holding everything the chain can produce resolves each
    /// reference exactly as the step's own position would.
    pub(crate) fn render_steps(
        &self,
        document: &Document,
        input_columns: &[Column],
        steps: &[FrameStep],
    ) -> Vec<RenderedFrameStep> {
        if steps.is_empty() {
            return Vec::new();
        }
        let mut scope = FrameObject {
            comment: None,
            id: self.id.clone(),
            name: self.name.clone(),
            columns: input_columns.to_vec(),
            rows: Vec::new(),
            steps: Vec::new(),
            display: FrameDisplay::default(),
            base_columns: Vec::new(),
            source_file: None,
            artifact: None,
            connector: None,
            derivation: None,
            generator: None,
            entry_columns: Vec::new(),
            materialization: None,
            unique_keys: Vec::new(),
            summaries: Vec::new(),
        };
        for step in steps {
            let outputs = match step {
                FrameStep::WithColumns { columns } => columns.iter().collect::<Vec<_>>(),
                FrameStep::Summarize {
                    group_keys,
                    aggregates,
                    ..
                } => group_keys.iter().chain(aggregates).collect(),
                _ => Vec::new(),
            };
            for output in outputs {
                if scope
                    .columns
                    .iter()
                    .any(|column| column.id == output.output_column_id)
                {
                    continue;
                }
                let declared = self
                    .columns
                    .iter()
                    .find(|column| column.id == output.output_column_id)
                    .cloned();
                // A column the chain produced and a later step dropped has
                // no declared name left to find. When it was a plain rename
                // of something already in scope — which is exactly what a
                // linked frame's pass-through projection is — the name it
                // stood for is still right there, so it is inherited rather
                // than lost. Without this, a summarize over a linked frame
                // renders every reference below it as a raw column id.
                let inherited = || match &output.expression {
                    Expr::Column { column_id } => scope
                        .columns
                        .iter()
                        .find(|column| column.id == *column_id)
                        .map(|column| Column {
                            id: output.output_column_id.clone(),
                            ..column.clone()
                        }),
                    _ => None,
                };
                // Anything genuinely computed and then dropped keeps the id
                // as its name: it renders something referenceable, and the
                // step that dropped it means nothing below can name it.
                scope
                    .columns
                    .push(declared.or_else(inherited).unwrap_or_else(|| Column {
                        id: output.output_column_id.clone(),
                        name: output.output_column_id.clone(),
                        source_name: None,
                        data_type: DataType::String,
                        categories: Vec::new(),
                        format: None,
                        formula: None,
                    }));
            }
            // A pivot or unpivot produces columns with no formula behind
            // them, so they never appear in `outputs` above — but a later
            // step's formula can still read them, and has to render as a
            // name rather than an id. A pivot output's own value is its
            // natural name; the unpivot pair falls back to the id, since a
            // dropped one has no name left anywhere.
            let named_outputs: Vec<(&Id, String)> = match step {
                FrameStep::Expand { frame_id, outputs } => {
                    let expanded = document.frame(frame_id).ok();
                    outputs
                        .iter()
                        .map(|output| {
                            let name = expanded
                                .and_then(|frame| {
                                    frame
                                        .columns
                                        .iter()
                                        .find(|column| column.id == output.source_column_id)
                                })
                                .map(|column| column.name.clone())
                                .unwrap_or_else(|| output.source_column_id.clone());
                            (&output.output_column_id, name)
                        })
                        .collect()
                }
                FrameStep::Pivot { outputs, .. } => outputs
                    .iter()
                    .map(|output| (&output.output_column_id, output.value.clone()))
                    .collect(),
                FrameStep::Unpivot {
                    name_column_id,
                    value_column_id,
                    ..
                } => vec![
                    (name_column_id, name_column_id.clone()),
                    (value_column_id, value_column_id.clone()),
                ],
                _ => Vec::new(),
            };
            for (output_column_id, fallback_name) in named_outputs {
                if scope
                    .columns
                    .iter()
                    .any(|column| column.id == *output_column_id)
                {
                    continue;
                }
                let declared = self
                    .columns
                    .iter()
                    .find(|column| column.id == *output_column_id)
                    .cloned();
                scope.columns.push(declared.unwrap_or_else(|| Column {
                    id: output_column_id.clone(),
                    name: fallback_name,
                    source_name: None,
                    data_type: DataType::String,
                    categories: Vec::new(),
                    format: None,
                    formula: None,
                }));
            }
        }
        let render_all = |items: &[DerivedExpression]| {
            items
                .iter()
                .map(|item| RenderedDerivedExpression {
                    output_column_id: item.output_column_id.clone(),
                    formula: item.expression.render(&scope, document, 0),
                })
                .collect::<Vec<_>>()
        };
        steps
            .iter()
            .map(|step| match step {
                FrameStep::Filter {
                    predicates,
                    match_all,
                } => RenderedFrameStep::Filter {
                    predicates: predicates
                        .iter()
                        .map(|expression| expression.render(&scope, document, 0))
                        .collect(),
                    match_all: *match_all,
                },
                FrameStep::WithColumns { columns } => RenderedFrameStep::WithColumns {
                    columns: render_all(columns),
                },
                FrameStep::Select { column_ids } => RenderedFrameStep::Select {
                    column_ids: column_ids.clone(),
                },
                FrameStep::Summarize {
                    group_keys,
                    aggregates,
                    maintain_order,
                } => RenderedFrameStep::Summarize {
                    group_keys: render_all(group_keys),
                    aggregates: render_all(aggregates),
                    maintain_order: *maintain_order,
                },
                FrameStep::Join { join } => RenderedFrameStep::Join { join: join.clone() },
                FrameStep::Sort { keys } => RenderedFrameStep::Sort { keys: keys.clone() },
                FrameStep::Union { frame_id, mapping } => RenderedFrameStep::Union {
                    frame_id: frame_id.clone(),
                    mapping: mapping.clone(),
                },
                FrameStep::Expand { frame_id, outputs } => RenderedFrameStep::Expand {
                    frame_id: frame_id.clone(),
                    outputs: outputs.clone(),
                },
                FrameStep::Pivot {
                    names_column_id,
                    values_column_id,
                    aggregate,
                    outputs,
                } => RenderedFrameStep::Pivot {
                    names_column_id: names_column_id.clone(),
                    values_column_id: values_column_id.clone(),
                    aggregate: *aggregate,
                    outputs: outputs.clone(),
                },
                FrameStep::Unpivot {
                    columns,
                    name_column_id,
                    value_column_id,
                } => {
                    let name_of = |column_id: &Id| {
                        scope
                            .columns
                            .iter()
                            .find(|column| &column.id == column_id)
                            .map(|column| column.name.clone())
                            .unwrap_or_else(|| column_id.clone())
                    };
                    RenderedFrameStep::Unpivot {
                        columns: columns.clone(),
                        name_column_name: name_of(name_column_id),
                        name_column_id: name_column_id.clone(),
                        value_column_name: name_of(value_column_id),
                        value_column_id: value_column_id.clone(),
                    }
                }
                FrameStep::Comment { text } => RenderedFrameStep::Comment { text: text.clone() },
            })
            .collect()
    }

    /// Every expression this frame holds, wherever it is kept.
    ///
    /// There are five places, and the reason to gather them in one is that
    /// anything asking "what does this frame read" has to ask all five or
    /// silently miss a dependency. A filter in the wrangle chain is as much
    /// a reference as a calculated column is.
    pub(crate) fn expressions(&self) -> impl Iterator<Item = &Expr> {
        let columns = self
            .columns
            .iter()
            .filter_map(|column| column.formula.as_ref())
            .map(|formula| &formula.expression);
        let overrides = self
            .rows
            .iter()
            .flat_map(|row| row.cells.values())
            .filter_map(|cell| cell.override_formula.as_ref())
            .map(|formula| &formula.expression);
        let own_steps = self.steps.iter().flat_map(step_expressions);
        let display = self.display.steps.iter().flat_map(step_expressions);
        let derived = self
            .derivation
            .iter()
            .flat_map(|derivation| derivation.steps.iter().flat_map(step_expressions));
        columns
            .chain(overrides)
            .chain(own_steps)
            .chain(display)
            .chain(derived)
    }

    /// Every other frame this one reads through a two-input step — a
    /// join's lookup, a union's stacked frame — wherever the step lives.
    /// A source frame's own chain can hold a union, so both step lists
    /// are asked; the derivation's answer covers the legacy flat join.
    pub(crate) fn lookup_frame_ids(&self) -> Vec<Id> {
        let mut lookup_ids: Vec<Id> = self
            .steps
            .iter()
            .filter_map(|step| step.lookup_frame_id().cloned())
            .collect();
        if let Some(derivation) = &self.derivation {
            lookup_ids.extend(derivation.lookup_frame_ids());
        }
        lookup_ids
    }

    /// Whether this frame's wrangle steps — its own chain or its
    /// derivation's — read `column_id` of `frame_id` through a two-input
    /// step. The formula-reference walks cannot see these edges: a union's
    /// mapping holds bare column ids, not expressions, so the guards that
    /// keep a column from being deleted out from under a reader have to
    /// ask here as well.
    pub(crate) fn wrangle_reads_foreign_column(&self, frame_id: &str, column_id: &str) -> bool {
        self.steps.iter().any(|step| match step {
            FrameStep::Union {
                frame_id: stacked_frame_id,
                mapping,
            } => {
                stacked_frame_id == frame_id
                    && mapping
                        .iter()
                        .any(|column| column.source_column_id.as_deref() == Some(column_id))
            }
            FrameStep::Expand {
                frame_id: expanded_frame_id,
                outputs,
            } => {
                expanded_frame_id == frame_id
                    && outputs
                        .iter()
                        .any(|output| output.source_column_id == column_id)
            }
            _ => false,
        }) || self
            .derivation
            .as_ref()
            .is_some_and(|derivation| derivation.references_input_column(frame_id, column_id))
    }

    /// Every other frame this one reads through a formula reference.
    ///
    /// A lineage edge that no derivation records, so every walk that decides
    /// staleness, liveness, or cache validity has to pick it up here.
    pub(crate) fn foreign_frames(&self) -> Vec<&str> {
        let mut output = Vec::new();
        for expression in self.expressions() {
            expression.foreign_frames(&mut output);
        }
        output.sort_unstable();
        output.dedup();
        output
    }

    pub(crate) fn references_object(&self, object_id: &str) -> bool {
        self.columns.iter().any(|column| {
            column
                .formula
                .as_ref()
                .is_some_and(|formula| formula.expression.references_object(object_id))
        }) || self.rows.iter().any(|row| {
            row.cells.values().any(|cell| {
                cell.override_formula
                    .as_ref()
                    .is_some_and(|formula| formula.expression.references_object(object_id))
            })
        })
    }

    pub(crate) fn references_column_from_other_formulas(&self, column_id: &str) -> bool {
        self.columns.iter().any(|column| {
            column.id != column_id
                && column
                    .formula
                    .as_ref()
                    .is_some_and(|formula| formula.expression.references_column(column_id))
        }) || self.rows.iter().any(|row| {
            row.cells.iter().any(|(cell_column_id, cell)| {
                cell_column_id != column_id
                    && cell
                        .override_formula
                        .as_ref()
                        .is_some_and(|formula| formula.expression.references_column(column_id))
            })
        })
    }

    pub(crate) fn compute(&self, document: &Document) -> ComputedFrame {
        let formulas = self
            .columns
            .iter()
            .filter_map(|column| {
                column.formula.as_ref().map(|formula| {
                    (
                        column.id.clone(),
                        formula.expression.render(self, document, 0),
                    )
                })
            })
            .collect();

        // A chained frame's stored rows are its *input*; what it shows is
        // the chain's output, which is read through pages like any other
        // artifact-backed frame. Computing per-row cells here would evaluate
        // the input rows against the output columns and produce nonsense.
        // One materialization feeds both the per-row cells and the
        // conditional-formatting masks: the rules are expressions over the
        // same rows, and running them is one more `with_columns` on the
        // frame that has just been built.
        let polars_frame = (!self.rows.is_empty() && self.steps.is_empty())
            .then(|| self.materialize_polars_frame(document));
        let polars_columns = match &polars_frame {
            Some(Ok(frame)) => self.polars_columns_from(document, frame),
            Some(Err(error)) => Err(error.clone()),
            None => Ok(HashMap::new()),
        };
        let style_rules = match &polars_frame {
            Some(Ok(frame)) => self.evaluate_style_rules(document, frame),
            _ => StyleRuleMatches::default(),
        };
        // Keyed by row id, not by position: a rule's answer has to survive
        // the frame being sorted or filtered the same way a direct format
        // does, and only the id is stable through both.
        let style_matches = self
            .rows
            .iter()
            .zip(style_rules.rows)
            .filter(|(_, matched)| !matched.is_empty())
            .map(|(row, matched)| (row.id.clone(), matched))
            .collect();
        let rows: HashMap<Id, HashMap<Id, ComputedCell>> = self
            .rows
            .iter()
            .enumerate()
            .map(|(row_index, row)| {
                let cells = self
                    .columns
                    .iter()
                    .map(|column| {
                        let cell = row.cells.get(&column.id);
                        let uses_formula = cell.is_some_and(|cell| cell.override_formula.is_some())
                            || column.formula.is_some();
                        let result = if uses_formula {
                            polars_columns
                                .as_ref()
                                .map_err(Clone::clone)
                                .and_then(|columns| {
                                    columns
                                        .get(&column.id)
                                        .and_then(|values| values.get(row_index))
                                        .cloned()
                                        .unwrap_or_else(|| {
                                            Err("Polars result is missing a row".into())
                                        })
                                })
                        } else {
                            parse_scalar_value(
                                cell.map(|cell| cell.raw.as_str()).unwrap_or_default(),
                                column.data_type,
                            )
                        };
                        (
                            column.id.clone(),
                            computed_cell(
                                result,
                                column.data_type,
                                cell.is_some_and(|cell| cell.override_formula.is_some()),
                            ),
                        )
                    })
                    .collect();
                (row.id.clone(), cells)
            })
            .collect();

        let override_formulas = self
            .rows
            .iter()
            .filter_map(|row| {
                let formulas: HashMap<Id, String> = row
                    .cells
                    .iter()
                    .filter_map(|(column_id, cell)| {
                        cell.override_formula.as_ref().map(|formula| {
                            (
                                column_id.clone(),
                                formula.expression.render(self, document, 0),
                            )
                        })
                    })
                    .collect();
                (!formulas.is_empty()).then(|| (row.id.clone(), formulas))
            })
            .collect();

        let summaries = literal_summary_cells(self, &rows);

        // One chain, whichever kind of frame carries it: a derived frame's
        // starts at the frame it reads, a source frame's at its own data.
        let (steps, pass_through_steps) = match &self.derivation {
            Some(derivation) => {
                let chain = derivation.steps();
                if derivation.join.is_some() {
                    // The join is configured in its own compact summary and
                    // is the fixed input to Wrangle. Only the steps after it
                    // are editable there, starting from the join's retained
                    // output schema.
                    let editable = chain
                        .strip_prefix(&[FrameStep::Join {
                            join: derivation.join.clone().expect("checked above"),
                        }])
                        .unwrap_or(&chain);
                    let input = if self.base_columns.is_empty() {
                        &self.columns
                    } else {
                        &self.base_columns
                    };
                    (
                        self.render_steps(document, input, editable),
                        pass_through_prefix(editable),
                    )
                } else {
                    let rendered = document
                        .frame(&derivation.source_frame_id)
                        .map(|source| self.render_steps(document, &source.columns, &chain))
                        .unwrap_or_default();
                    (rendered, pass_through_prefix(&chain))
                }
            }
            // A source frame's chain is only ever what someone wrote in it.
            None => (
                self.render_steps(document, self.input_columns(), &self.steps),
                0,
            ),
        };
        // The display layer runs after the chain, so its formulas resolve
        // against the declared columns rather than the chain's input.
        let display_steps = self.render_steps(document, &self.columns, &self.display.steps);

        // Only the two facts a chain cannot state about itself: what it
        // reads, and whether it is a join. The chain itself has already
        // been rendered into `steps` just above, and rendering it twice is
        // how the two copies would drift.
        let derivation = self.derivation.as_ref().and_then(|derivation| {
            document.frame(&derivation.source_frame_id).ok()?;
            Some(RenderedFrameDerivation {
                source_frame_id: derivation.source_frame_id.clone(),
                join: derivation.join.clone(),
            })
        });

        let paged = document.frame_depends_on_artifact(&self.id, &mut HashSet::new());
        // Only counts that are already known. A snapshot and an import both
        // record their own size, and an in-memory frame is its rows; a
        // derived frame read through pages would cost a full pass, so it
        // reports nothing and the caller takes the count from the page it
        // was going to read anyway.
        let total_rows = self
            .materialization
            .as_ref()
            .map(|materialization| materialization.artifact.row_count)
            .or_else(|| self.artifact.as_ref().map(|artifact| artifact.row_count))
            .or_else(|| (!paged).then_some(self.rows.len()));
        let materialization =
            self.materialization
                .as_ref()
                .map(|materialization| ComputedMaterialization {
                    row_count: materialization.artifact.row_count,
                    // Stale means the lineage moved on since the snapshot was
                    // written. It is reported, never acted on: the snapshot
                    // keeps serving its rows until someone refreshes it.
                    stale: document.frame_fingerprint_string(&self.id)
                        != materialization.fingerprint,
                });
        let upstream_stale = document.upstream_snapshot_is_stale(&self.id);
        let live = document.frame_is_live(&self.id);
        let editing = FrameEditing::for_frame(
            self,
            document.frame_cells_are_editable(&self.id),
            live,
            paged,
        );
        let source_name = self.source_name();
        let generator_rule = self
            .generator
            .as_ref()
            .map(|generator| document.render_formula_scalar(&generator.formula.expression));

        ComputedFrame {
            fingerprint: format!("{:016x}", document.frame_fingerprint(&self.id)),
            formulas,
            override_formulas,
            rows,
            summaries,
            derivation,
            steps,
            pass_through_steps,
            display_steps,
            total_rows,
            paged,
            materialization,
            generator_rule,
            editing,
            live,
            source_name,
            upstream_stale,
            style_matches,
            style_rule_errors: style_rules.errors,
            style_rule_formulas: self
                .display
                .style_rules
                .iter()
                .map(|rule| {
                    (
                        rule.id.clone(),
                        rule.formula.expression.render(self, document, 0),
                    )
                })
                .collect(),
        }
    }

    pub(crate) fn base_polars_lazy(&self) -> Result<pl::LazyFrame, String> {
        if let Some(artifact) = &self.artifact {
            let mut scan = pl::LazyFrame::scan_parquet(
                pl::PlRefPath::new(&artifact.path),
                pl::ScanArgsParquet::default(),
            )
            .map_err(|error| error.to_string())?;
            let schema = scan.collect_schema().map_err(|error| error.to_string())?;
            for column in self
                .input_columns()
                .iter()
                .filter(|column| column.formula.is_none())
            {
                let source_name = column.source_name.as_deref().unwrap_or(&column.name);
                if schema.get(source_name).is_none() {
                    return Err(format!(
                        "Source field ‘{source_name}’ for ‘{}’ [{}] is missing",
                        column.name, column.id
                    ));
                }
            }
            return Ok(scan.select(
                self.input_columns()
                    .iter()
                    .filter(|column| column.formula.is_none())
                    .map(|column| {
                        let source = pl::col(column.source_name.as_deref().unwrap_or(&column.name));
                        // A parquet scan preserves the physical width its
                        // importer chose (a CSV count is often UInt32), but
                        // FrameWork's declared Integer is an i64. Normalize
                        // at the artifact boundary so a later Stack has the
                        // same concrete type as an integer typed directly
                        // into a document; leaving it as UInt32 asks Polars
                        // to discover a supertype only after two plans meet.
                        let source = match column.data_type {
                            DataType::Integer => source.cast(pl::DataType::Int64),
                            DataType::Number | DataType::Currency | DataType::Percentage => {
                                source.cast(pl::DataType::Float64)
                            }
                            _ => source,
                        };
                        source.alias(&column.id)
                    })
                    .collect::<Vec<_>>(),
            ));
        }
        if let Some(path) = self.source_file.as_deref() {
            let path_buf = Path::new(path);
            let frame = read_import_frame_full(path_buf).map_err(|error| error.to_string())?;
            let mut columns = Vec::new();
            for column in self
                .input_columns()
                .iter()
                .filter(|column| column.formula.is_none())
            {
                let source_name = column.source_name.as_deref().unwrap_or(&column.name);
                let series = frame
                    .column(source_name)
                    .or_else(|_| frame.column(&column.id))
                    .map_err(|error| error.to_string())?;
                let mut series = series.as_materialized_series().clone();
                series.rename(column.id.clone().into());
                columns.push(series.into());
            }
            return pl::DataFrame::new(frame.height(), columns)
                .map(IntoLazy::lazy)
                .map_err(|error| error.to_string());
        }

        let mut columns = Vec::new();
        for column in self
            .input_columns()
            .iter()
            .filter(|column| column.formula.is_none())
        {
            let name = column.id.clone().into();
            let series = match column.data_type {
                DataType::String | DataType::Categorical => {
                    let text = pl::Series::new(
                        name,
                        self.rows
                            .iter()
                            .map(|row| {
                                let raw = row
                                    .cells
                                    .get(&column.id)
                                    .map(|cell| cell.raw.as_str())
                                    .unwrap_or_default();
                                (!raw.trim().is_empty()).then(|| raw.to_string())
                            })
                            .collect::<Vec<_>>(),
                    );
                    // A declared list is a type, not a note in the margin: it
                    // is what makes the column sort and compare in the order
                    // it was written down. A column called categorical with
                    // nothing declared yet is still just text.
                    if column.data_type == DataType::Categorical && !column.categories.is_empty() {
                        text.cast(&category_dtype(&column.categories)?)
                            .map_err(|error| error.to_string())?
                    } else {
                        text
                    }
                }
                DataType::Integer => pl::Series::new(
                    name,
                    self.rows
                        .iter()
                        .map(|row| {
                            row.cells
                                .get(&column.id)
                                .and_then(|cell| parse_integer(&cell.raw))
                        })
                        .collect::<Vec<_>>(),
                ),
                DataType::Number | DataType::Currency | DataType::Percentage => pl::Series::new(
                    name,
                    self.rows
                        .iter()
                        .map(|row| {
                            row.cells
                                .get(&column.id)
                                .and_then(|cell| parse_number(&cell.raw))
                        })
                        .collect::<Vec<_>>(),
                ),
                DataType::Boolean => pl::Series::new(
                    name,
                    self.rows
                        .iter()
                        .map(|row| {
                            row.cells
                                .get(&column.id)
                                .and_then(|cell| parse_boolean(&cell.raw))
                        })
                        .collect::<Vec<_>>(),
                ),
                DataType::Date => pl::Series::new(
                    name,
                    self.rows
                        .iter()
                        .map(|row| {
                            row.cells
                                .get(&column.id)
                                .and_then(|cell| parse_date(&cell.raw))
                        })
                        .collect::<Vec<_>>(),
                ),
            };
            columns.push(series.into());
        }
        pl::DataFrame::new(self.rows.len(), columns)
            .map(IntoLazy::lazy)
            .map_err(|error| error.to_string())
    }

    /// The rows a generated frame's rule evaluates to, as a base plan.
    ///
    /// The rule is compiled exactly like a scratchpad line and run against a
    /// one-row probe, so whatever a line can hold — a sequence, a written
    /// list, one value — becomes rows here. A list answer is opened out; a
    /// single value is one row. Evaluated eagerly rather than left lazy
    /// because a generator is small by construction (`sequence` refuses to
    /// build more than it can hold) and the collected series is what decides
    /// the row count everything downstream needs.
    fn generator_polars_lazy(&self, document: &Document) -> Result<pl::LazyFrame, String> {
        let generator = self
            .generator
            .as_ref()
            .expect("only called for generated frames");
        let column = self
            .columns
            .first()
            .ok_or("A generated frame needs a column to fill")?;
        let (_, mut series) = document.evaluate_rule_series(&generator.formula.expression)?;
        series.rename(column.id.clone().into());
        // The declared column type is the frame's contract; hold the
        // generated values to it the way an artifact scan is held to its
        // declared widths.
        let series = match column.data_type {
            DataType::Integer => series
                .cast(&pl::DataType::Int64)
                .map_err(|error| error.to_string())?,
            DataType::Number | DataType::Currency | DataType::Percentage => series
                .cast(&pl::DataType::Float64)
                .map_err(|error| error.to_string())?,
            _ => series,
        };
        pl::DataFrame::new(series.len(), vec![series.into()])
            .map(IntoLazy::lazy)
            .map_err(|error| error.to_string())
    }

    pub(crate) fn materialize_polars_lazy(
        &self,
        document: &Document,
    ) -> Result<pl::LazyFrame, String> {
        let mut plan = if self.generator.is_some() {
            self.generator_polars_lazy(document)?
        } else {
            self.base_polars_lazy()?
        };
        for layer in self.calculated_column_layers()? {
            let expressions = layer
                .iter()
                .map(|column| {
                    column
                        .formula
                        .as_ref()
                        .expect("calculated-column layer contains formulas")
                        .expression
                        .to_polars(document)
                        .map(|expression| expression.alias(column.id.clone()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            plan = plan.with_columns(expressions);
        }
        Ok(plan)
    }

    pub(crate) fn materialize_polars_frame(
        &self,
        document: &Document,
    ) -> Result<pl::DataFrame, String> {
        self.materialize_polars_lazy(document)?
            .collect()
            .map_err(|error| format!("Polars error in calculated columns: {error}"))
    }

    pub(crate) fn calculated_column_layers(&self) -> Result<Vec<Vec<&Column>>, String> {
        let mut remaining = self
            .input_columns()
            .iter()
            .filter(|column| column.formula.is_some())
            .collect::<Vec<_>>();
        let mut completed = HashSet::new();
        let mut layers = Vec::new();
        while !remaining.is_empty() {
            let before = remaining.len();
            let mut next = Vec::new();
            let mut layer = Vec::new();
            for column in remaining {
                let formula = column.formula.as_ref().expect("filtered formula column");
                let mut dependencies = Vec::new();
                formula.expression.column_dependencies(&mut dependencies);
                let blocked = dependencies.iter().any(|dependency| {
                    self.input_columns()
                        .iter()
                        .find(|candidate| candidate.id == **dependency)
                        .is_some_and(|candidate| {
                            candidate.formula.is_some() && !completed.contains(&candidate.id)
                        })
                });
                if blocked {
                    next.push(column);
                    continue;
                }
                layer.push(column);
            }
            if next.len() == before {
                return Err("Circular calculated-column dependency".into());
            }
            completed.extend(layer.iter().map(|column| column.id.clone()));
            layers.push(layer);
            remaining = next;
        }
        Ok(layers)
    }

    pub(crate) fn evaluate_polars_series(
        &self,
        document: &Document,
        frame: &pl::DataFrame,
        expression: &Expr,
    ) -> Result<pl::Series, String> {
        let evaluated = frame
            .clone()
            .lazy()
            // A frame formula is a column, even when it contains an
            // aggregate that broadcasts back over every row. `select` is a
            // projection and may keep an aggregate branch at height one
            // beside an elementwise branch at frame height; on a chunked,
            // multi-column CSV Polars then panics while assembling the
            // projection instead of returning an error. `with_columns` is
            // the operation this formula will actually run as, and owns the
            // broadcast semantics we need here as well as in calculated
            // columns and conditional-formatting masks.
            .with_columns([expression.to_polars(document)?.alias("__framework_result")])
            .collect()
            .map_err(|error| in_plain_words(error.to_string()))?;
        Ok(evaluated
            .column("__framework_result")
            .map_err(|error| error.to_string())?
            .as_materialized_series()
            .clone())
    }

    /// The per-row cells, read off a frame that has already been built.
    ///
    /// Takes the materialized frame rather than building one because the
    /// caller computing a whole view wants that same frame for the
    /// conditional-formatting masks, and materializing it twice is the whole
    /// calculated-column chain run twice.
    pub(crate) fn polars_columns_from(
        &self,
        document: &Document,
        frame: &pl::DataFrame,
    ) -> Result<HashMap<Id, Vec<Result<ScalarValue, String>>>, String> {
        let mut output: HashMap<Id, Vec<Result<ScalarValue, String>>> = HashMap::new();
        for column in &self.columns {
            let series = frame
                .column(&column.id)
                .map_err(|error| error.to_string())?
                .as_materialized_series();
            output.insert(
                column.id.clone(),
                (0..self.rows.len())
                    .map(|index| polars_value_at(series, index))
                    .collect(),
            );
        }

        for (row_index, row) in self.rows.iter().enumerate() {
            for (column_id, cell) in &row.cells {
                let Some(override_formula) = &cell.override_formula else {
                    continue;
                };
                let result = self
                    .evaluate_polars_series(document, frame, &override_formula.expression)
                    .and_then(|series| polars_value_at(&series, row_index));
                if let Some(values) = output.get_mut(column_id) {
                    values[row_index] = result;
                }
            }
        }
        Ok(output)
    }

    /// What a column of this expression holds, asked of Polars and then of
    /// the document.
    ///
    /// Polars answers Float64 to both `debit` and `debit.sum()`, because how
    /// a number is written is not a fact Polars carries — it is one this
    /// document keeps. Summing a column of money gives money, and a column
    /// that came out of a group-by reading `$98.00` and `$2,057.90` should
    /// not be the one place in the document that forgets it.
    pub(crate) fn infer_polars_expression_type(
        &self,
        document: &Document,
        expression: &Expr,
    ) -> Result<DataType, String> {
        let frame = document.materialize_frame_frame(&self.id, Layer::Data, &mut HashSet::new())?;
        let series = self.evaluate_polars_series(document, &frame, expression)?;
        let found = framework_type_from_polars(series.dtype())?;
        Ok(written_type(found, expression.declared_type(document)))
    }
}

/// The expressions one step holds. `Select`, `Join`, and `Sort` name
/// columns by id rather than by formula, so they hold none.
pub(crate) fn step_expressions(step: &FrameStep) -> Box<dyn Iterator<Item = &Expr> + '_> {
    match step {
        FrameStep::Filter { predicates, .. } => Box::new(predicates.iter()),
        FrameStep::WithColumns { columns } => {
            Box::new(columns.iter().map(|column| &column.expression))
        }
        FrameStep::Summarize {
            group_keys,
            aggregates,
            ..
        } => Box::new(
            group_keys
                .iter()
                .chain(aggregates)
                .map(|derived| &derived.expression),
        ),
        FrameStep::Select { .. }
        | FrameStep::Join { .. }
        | FrameStep::Sort { .. }
        | FrameStep::Union { .. }
        | FrameStep::Expand { .. }
        | FrameStep::Pivot { .. }
        | FrameStep::Unpivot { .. }
        | FrameStep::Comment { .. } => Box::new(std::iter::empty()),
    }
}

/// How many leading steps exist only so a derived frame owns its column
/// ids: a projection of bare column references, and the select that adopts
/// them as the frame's schema.
///
/// This is the shape `AddLinkedFrame` and `BranchFrame` produce, and the
/// shape `FrameDerivation::steps` synthesizes from the legacy `projections`
/// field. It carries no transformation — every output is one input column,
/// unchanged — so presenting it as something the user wrote is noise. It is
/// still load-bearing: it is what records this frame's dependency on each
/// source column, which is how deleting one out from under it is refused.
fn pass_through_prefix(steps: &[FrameStep]) -> usize {
    let [
        FrameStep::WithColumns { columns },
        FrameStep::Select { column_ids },
        ..,
    ] = steps
    else {
        return 0;
    };
    let renames_only = columns
        .iter()
        .all(|column| matches!(column.expression, Expr::Column { .. }));
    let adopts_them = column_ids.len() == columns.len()
        && column_ids
            .iter()
            .zip(columns)
            .all(|(selected, column)| *selected == column.output_column_id);
    if renames_only && adopts_them { 2 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::test_support::*;
    #[allow(unused_imports)]
    use crate::*;
    #[allow(unused_imports)]
    use std::{fs, path::PathBuf};
    #[allow(unused_imports)]
    use uuid::Uuid;

    #[test]
    pub(crate) fn calculated_columns_are_batched_by_dependency_layer() {
        let mut store = demo_store();
        let frame_id = store
            .document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) => Some(frame.id.clone()),
                _ => None,
            })
            .unwrap();

        let formulas = [
            ("Quantity plus one", "`Quantity` + 1"),
            ("Price plus one", "`Unit price` + 1"),
            ("Combined", "`Quantity plus one` + `Price plus one`"),
        ];
        for (name, formula) in formulas {
            let expression = store
                .document
                .prepare_formula_for_frame(&frame_id, formula)
                .unwrap();
            store
                .document
                .frame_mut(&frame_id)
                .unwrap()
                .columns
                .push(Column {
                    id: column_id(name),
                    name: name.into(),
                    source_name: None,
                    data_type: DataType::Number,
                    categories: Vec::new(),
                    format: None,
                    formula: Some(Formula { expression }),
                });
        }

        let frame = store.document.frame(&frame_id).unwrap();
        let layers = frame.calculated_column_layers().unwrap();
        assert_eq!(
            layers.iter().map(|layer| layer.len()).collect::<Vec<_>>(),
            vec![3, 1]
        );
        assert_eq!(layers[1][0].name, "Combined");

        let view = store.view();
        let combined_id = frame
            .columns
            .iter()
            .find(|column| column.name == "Combined")
            .unwrap()
            .id
            .clone();
        let values = frame
            .rows
            .iter()
            .map(|row| view.computed_frames[&frame_id].rows[&row.id][&combined_id].value)
            .collect::<Vec<_>>();
        assert_eq!(values, vec![Some(19.0), Some(14.5), Some(32.0)]);
    }
}
