//! Resolving `Operation`s in this family into fully determined
//! `ReplicatedOperation`s: IDs minted, formula names bound to column IDs, and
//! every precondition checked before anything is applied.
//!
//! Derived frames — transformation chains, joins, unique keys, and the
//! parquet snapshots a frame can be materialized into.

use crate::*;
use polars::prelude as pl;
use std::collections::{HashMap, HashSet};

impl Document {
    // The parameter list mirrors the operation's own fields; collapsing it
    // into a struct would just rename the variant.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_add_derived_frame(
        &self,
        source_frame_id: Id,
        name: String,
        group_keys: Vec<NamedFormulaInput>,
        aggregates: Vec<NamedFormulaInput>,
        maintain_order: bool,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            let name = self.unique_frame_name(&name, None);
            if aggregates.is_empty() {
                return Err(CoreError::InvalidOperation(
                    "A derived aggregate frame needs at least one aggregate".into(),
                ));
            }
            let source = self.frame(&source_frame_id)?;
            let mut columns = Vec::new();
            let mut parsed_group_keys = Vec::new();
            let mut parsed_aggregates = Vec::new();
            for input in group_keys {
                let expression =
                    self.prepare_formula_for_frame(&source_frame_id, &input.formula)?;
                let data_type = source
                    .infer_polars_expression_type(self, &expression)
                    .map_err(CoreError::Formula)?;
                let output_column_id = column_id(&input.name);
                columns.push(Column {
                    id: output_column_id.clone(),
                    name: input.name,
                    source_name: None,
                    data_type,
                    categories: Vec::new(),
                    format: None,
                    formula: None,
                });
                parsed_group_keys.push(DerivedExpression {
                    output_column_id,
                    expression,
                });
            }
            for input in aggregates {
                let expression =
                    self.prepare_formula_for_frame(&source_frame_id, &input.formula)?;
                let data_type = source
                    .infer_polars_expression_type(self, &expression)
                    .map_err(CoreError::Formula)?;
                let output_column_id = column_id(&input.name);
                columns.push(Column {
                    id: output_column_id.clone(),
                    name: input.name,
                    source_name: None,
                    data_type,
                    categories: Vec::new(),
                    format: None,
                    formula: None,
                });
                parsed_aggregates.push(DerivedExpression {
                    output_column_id,
                    expression,
                });
            }
            let object_id = id();
            ReplicatedOperation::AddObject {
                object: DataObject::Frame(FrameObject {
                    comment: None,
                    id: object_id.clone(),
                    name,
                    columns,
                    rows: Vec::new(),
                    steps: Vec::new(),
                    display: FrameDisplay::default(),
                    base_columns: Vec::new(),
                    source_file: None,
                    artifact: None,
                    connector: None,
                    generator: None,
                    entry_columns: Vec::new(),
                    materialization: None,
                    derivation: Some(FrameDerivation {
                        source_frame_id,
                        join: None,
                        steps: vec![FrameStep::Summarize {
                            group_keys: parsed_group_keys,
                            aggregates: parsed_aggregates,
                            maintain_order,
                        }],
                    }),
                    unique_keys: Vec::new(),
                    summaries: Vec::new(),
                }),
                view: CanvasView {
                    id: id(),
                    object_id,
                    x,
                    y,
                    width: 520.0,
                    height: 280.0,
                    collapsed: false,
                    tab_object_ids: Vec::new(),
                },
                container_id: None,
            }
        })
    }

    pub(crate) fn prepare_add_linked_frame(
        &self,
        source_frame_id: Id,
        name: String,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            let name = self.unique_frame_name(&name, None);
            let source = self.frame(&source_frame_id)?;
            let mut columns = Vec::with_capacity(source.columns.len());
            let mut projections = Vec::with_capacity(source.columns.len());
            for source_column in &source.columns {
                let output_column_id = column_id(&source_column.name);
                columns.push(Column {
                    id: output_column_id.clone(),
                    name: source_column.name.clone(),
                    source_name: None,
                    data_type: source_column.data_type,
                    categories: source_column.categories.clone(),
                    format: source_column.format.clone(),
                    formula: None,
                });
                projections.push(DerivedExpression {
                    output_column_id,
                    expression: Expr::Column {
                        column_id: source_column.id.clone(),
                    },
                });
            }
            let column_ids = projections
                .iter()
                .map(|projection| projection.output_column_id.clone())
                .collect();
            let object_id = id();
            ReplicatedOperation::AddObject {
                object: DataObject::Frame(FrameObject {
                    comment: None,
                    id: object_id.clone(),
                    name,
                    columns,
                    rows: Vec::new(),
                    steps: Vec::new(),
                    display: FrameDisplay::default(),
                    base_columns: Vec::new(),
                    source_file: None,
                    artifact: None,
                    connector: None,
                    generator: None,
                    entry_columns: Vec::new(),
                    materialization: None,
                    derivation: Some(FrameDerivation {
                        source_frame_id,
                        join: None,
                        // The identity projection and the select that
                        // adopts it: no transformation, but it is what
                        // gives this frame column ids of its own to
                        // publish. `pass_through_prefix` in engine/frame.rs
                        // recognizes exactly this pair and hides it from
                        // the editor.
                        steps: vec![
                            FrameStep::WithColumns {
                                columns: projections,
                            },
                            FrameStep::Select { column_ids },
                        ],
                    }),
                    unique_keys: Vec::new(),
                    summaries: Vec::new(),
                }),
                view: CanvasView {
                    id: id(),
                    object_id,
                    x,
                    y,
                    width: 520.0,
                    height: 280.0,
                    collapsed: false,
                    tab_object_ids: Vec::new(),
                },
                container_id: None,
            }
        })
    }

    pub(crate) fn prepare_set_frame_materialization(
        &self,
        frame_id: Id,
        artifact: DataArtifact,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            let frame = self.frame(&frame_id)?;
            if frame.derivation.is_none() {
                return Err(CoreError::InvalidOperation(
                    "Only a derived frame can be cached to a snapshot".into(),
                ));
            }
            // Recorded against the lineage as it stands right now, so a
            // later change upstream shows the snapshot as stale.
            let fingerprint = self.frame_fingerprint_string(&frame_id);
            ReplicatedOperation::SetFrameMaterialization {
                frame_id,
                materialization: Some(Materialization {
                    artifact,
                    fingerprint,
                }),
            }
        })
    }

    pub(crate) fn prepare_clear_frame_materialization(
        &self,
        frame_id: Id,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            self.frame(&frame_id)?;
            ReplicatedOperation::SetFrameMaterialization {
                frame_id,
                materialization: None,
            }
        })
    }

    pub(crate) fn prepare_set_unique_key(
        &self,
        frame_id: Id,
        column_ids: Vec<Id>,
        enabled: bool,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            if column_ids.is_empty() {
                return Err(CoreError::InvalidOperation(
                    "A unique key needs at least one column".into(),
                ));
            }
            let frame = self.frame(&frame_id)?;
            if column_ids
                .iter()
                .any(|column_id| !frame.columns.iter().any(|column| column.id == *column_id))
            {
                return Err(CoreError::ColumnNotFound);
            }
            let mut unique_keys = frame.unique_keys.clone();
            unique_keys.retain(|key| key.column_ids != column_ids);
            if enabled {
                unique_keys.push(UniqueKeyConstraint {
                    id: id(),
                    column_ids,
                });
            }
            ReplicatedOperation::SetUniqueKeys {
                frame_id,
                unique_keys,
            }
        })
    }

    // The parameter list mirrors the operation's own fields; collapsing it
    // into a struct would just rename the variant.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_add_join_frame(
        &self,
        primary_frame_id: Id,
        lookup_frame_id: Id,
        primary_key_column_ids: Vec<Id>,
        lookup_key_column_ids: Vec<Id>,
        join_type: FrameJoinType,
        output_inputs: Vec<JoinColumnInput>,
        name: String,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            let name = self.unique_frame_name(&name, None);
            if primary_frame_id == lookup_frame_id {
                return Err(CoreError::InvalidOperation(
                    "Choose two different frames to create a join".into(),
                ));
            }
            if primary_key_column_ids.len() != 1 || lookup_key_column_ids.len() != 1 {
                return Err(CoreError::InvalidOperation(
                    "This version supports one column on each side of a join".into(),
                ));
            }
            if output_inputs.is_empty() {
                return Err(CoreError::InvalidOperation(
                    "Choose at least one output column".into(),
                ));
            }
            let primary = self.frame(&primary_frame_id)?;
            let lookup = self.frame(&lookup_frame_id)?;
            // Anti and semi joins skip the unique-lookup-key requirement;
            // see validate_join_derivations for the rationale.
            if join_type.keeps_lookup_columns()
                && !lookup
                    .unique_keys
                    .iter()
                    .any(|key| key.column_ids == lookup_key_column_ids)
            {
                return Err(CoreError::InvalidOperation(
                    "The lookup column must be marked as a unique key".into(),
                ));
            }
            if !join_type.keeps_lookup_columns() {
                let lookup_outputs = output_inputs
                    .iter()
                    .filter(|input| input.source_frame_id == lookup_frame_id)
                    .map(|input| input.name.as_str())
                    .collect::<Vec<_>>();
                if !lookup_outputs.is_empty() {
                    return Err(CoreError::InvalidOperation(format!(
                        "A {} join keeps only {} columns; remove {}",
                        join_type.label(),
                        primary.name,
                        lookup_outputs.join(", ")
                    )));
                }
            }
            let primary_key = primary
                .columns
                .iter()
                .find(|column| column.id == primary_key_column_ids[0])
                .ok_or(CoreError::ColumnNotFound)?;
            let lookup_key = lookup
                .columns
                .iter()
                .find(|column| column.id == lookup_key_column_ids[0])
                .ok_or(CoreError::ColumnNotFound)?;
            if !join_types_compatible(primary_key.data_type, lookup_key.data_type) {
                return Err(CoreError::InvalidOperation(
                    "Join columns must have compatible types".into(),
                ));
            }

            // Key columns whose two sides allow different sets of values.
            // The join reads those as text — see `match_key_types` — so an
            // output taken from one of them is text, and must not go on
            // offering a dropdown and an order it no longer has.
            let read_as_text: HashSet<&Id> = primary_key_column_ids
                .iter()
                .zip(lookup_key_column_ids.iter())
                .filter_map(|(primary_id, lookup_id)| {
                    let primary_column = primary
                        .columns
                        .iter()
                        .find(|column| &column.id == primary_id)?;
                    let lookup_column = lookup
                        .columns
                        .iter()
                        .find(|column| &column.id == lookup_id)?;
                    let categorical = primary_column.data_type == DataType::Categorical
                        || lookup_column.data_type == DataType::Categorical;
                    (categorical && primary_column.categories != lookup_column.categories)
                        .then_some([primary_id, lookup_id])
                })
                .flatten()
                .collect();

            let mut columns = Vec::with_capacity(output_inputs.len());
            let mut outputs = Vec::with_capacity(output_inputs.len());
            for input in output_inputs {
                let source = if input.source_frame_id == primary_frame_id {
                    primary
                } else if input.source_frame_id == lookup_frame_id {
                    lookup
                } else {
                    return Err(CoreError::InvalidOperation(
                        "Join outputs must come from one of the input frames".into(),
                    ));
                };
                let source_column = source
                    .columns
                    .iter()
                    .find(|column| column.id == input.source_column_id)
                    .ok_or(CoreError::ColumnNotFound)?;
                let output_column_id = column_id(&input.name);
                let text = read_as_text.contains(&input.source_column_id);
                columns.push(Column {
                    id: output_column_id.clone(),
                    name: input.name,
                    source_name: None,
                    data_type: if text {
                        DataType::String
                    } else {
                        source_column.data_type
                    },
                    categories: if text {
                        Vec::new()
                    } else {
                        source_column.categories.clone()
                    },
                    format: source_column.format.clone(),
                    formula: None,
                });
                outputs.push(JoinOutput {
                    output_column_id,
                    source_frame_id: input.source_frame_id,
                    source_column_id: input.source_column_id,
                });
            }
            let object_id = id();
            ReplicatedOperation::AddObject {
                object: DataObject::Frame(FrameObject {
                    comment: None,
                    id: object_id.clone(),
                    name,
                    // The join is the immutable input to any Wrangle steps
                    // authored on the result. Keep that schema separately
                    // from `columns`, which is free to become the output of
                    // a later Select, Summarize, or calculated column.
                    base_columns: columns.clone(),
                    columns,
                    rows: Vec::new(),
                    steps: Vec::new(),
                    display: FrameDisplay::default(),
                    source_file: None,
                    artifact: None,
                    connector: None,
                    generator: None,
                    entry_columns: Vec::new(),
                    materialization: None,
                    derivation: Some(FrameDerivation {
                        source_frame_id: primary_frame_id,
                        join: Some(FrameJoin {
                            lookup_frame_id,
                            primary_key_column_ids,
                            lookup_key_column_ids,
                            join_type,
                            outputs,
                        }),
                        steps: Vec::new(),
                    }),
                    unique_keys: Vec::new(),
                    summaries: Vec::new(),
                }),
                view: CanvasView {
                    id: id(),
                    object_id,
                    x,
                    y,
                    width: 620.0,
                    height: 300.0,
                    collapsed: false,
                    tab_object_ids: Vec::new(),
                },
                container_id: None,
            }
        })
    }

    /// Walks a chain, parsing each step against what the steps before it
    /// leave behind, and records the schema at every position.
    ///
    /// Saving a chain and previewing one are the same walk. Only the ending
    /// differs: saving needs the parsed steps and refuses a chain it cannot
    /// parse, while the editor wants the schemas it did manage to work out
    /// and the index of the step that stopped it. So the walk goes as far
    /// as it can and hands back where it stopped, rather than throwing away
    /// what it learned on the first bad formula.
    /// `stop_after` ends the walk once that step has been applied, which is
    /// how a sample asks for the data as it stands partway down a chain.
    pub(crate) fn walk_pipeline(
        &self,
        frame_id: &str,
        steps: Vec<FrameStepInput>,
        stop_after: Option<usize>,
    ) -> Result<(PipelineWalk, Option<(usize, CoreError)>), CoreError> {
        let frame = self.frame(frame_id)?;
        let fixed_join = frame
            .derivation
            .as_ref()
            .and_then(|derivation| derivation.join.clone());
        let source_frame_id = frame
            .derivation
            .as_ref()
            .map(|derivation| derivation.source_frame_id.clone());
        let retained_format = |column_id: &str| {
            frame
                .columns
                .iter()
                .find(|column| column.id == column_id)
                .and_then(|column| column.format.clone())
        };

        // Where the chain starts, and what its first step sees. A derived
        // frame reads the frame it derives from; a source frame reads its
        // own data, whose schema is `input_columns` -- `columns` may already
        // be a previous chain's output.
        let (scope_frame_id, scope_name, input_columns, mut plan) = match &source_frame_id {
            Some(source_frame_id) => {
                let source = self.frame(source_frame_id)?;
                let mut source_plan = self
                    .materialize_frame_lazy(source_frame_id, Layer::Data, &mut HashSet::new())
                    .map_err(CoreError::Import)?;
                if let Some(join) = &fixed_join {
                    source_plan = self
                        .apply_step(
                            source_plan,
                            &FrameStep::Join { join: join.clone() },
                            &mut HashSet::new(),
                        )
                        .map_err(|error| CoreError::Transform(in_plain_words(error)))?;
                    (
                        frame_id.to_string(),
                        frame.name.clone(),
                        if frame.base_columns.is_empty() {
                            // Old documents did not retain the join-result
                            // schema. A flat join's declared columns are the
                            // same schema, so it remains a useful fallback.
                            frame.columns.clone()
                        } else {
                            frame.base_columns.clone()
                        },
                        source_plan,
                    )
                } else {
                    (
                        source_frame_id.clone(),
                        source.name.clone(),
                        source.columns.clone(),
                        source_plan,
                    )
                }
            }
            None => (
                frame_id.to_string(),
                frame.name.clone(),
                frame.input_columns().to_vec(),
                frame
                    .materialize_polars_lazy(self)
                    .map_err(CoreError::Import)?,
            ),
        };

        let steps = self.reconcile_pass_through_inputs(frame, &input_columns, steps);

        // The steps as they stand before this edit, kept so a re-saved
        // pivot can hand the same value the same output column identity.
        let existing_steps: Vec<FrameStep> = frame
            .derivation
            .as_ref()
            .map(|derivation| derivation.steps().into_owned())
            .unwrap_or_else(|| frame.steps.clone());

        // Display names for every column the chain can produce. Polars knows
        // ids and types; only the editor knows what a column is called.
        let mut names: HashMap<Id, String> = input_columns
            .iter()
            .map(|column| (column.id.clone(), column.name.clone()))
            .collect();
        let mut visible = input_columns.clone();
        // A replacement chain must be judged by the ordering it will have,
        // not by a sort in the old chain it is replacing. Source lineage and
        // the frame's separate display layer survive the replacement; each
        // Sort encountered below then makes ordering available to later
        // formula steps in this draft.
        let mut ordering_is_declared = source_frame_id
            .as_deref()
            .is_some_and(|source_id| self.plan_sorts(source_id))
            || frame
                .display
                .steps
                .iter()
                .any(|step| matches!(step, FrameStep::Sort { .. }));
        let mut walk = PipelineWalk {
            source_frame_id,
            input_columns,
            steps: Vec::new(),
            schemas: Vec::new(),
            plan: plan.clone(),
        };

        for (index, input) in steps.into_iter().enumerate() {
            let step = match self.parse_pipeline_step(
                input,
                &scope_frame_id,
                &scope_name,
                &visible,
                &mut names,
                &plan,
                frame_id,
                &existing_steps,
                ordering_is_declared,
            ) {
                Ok(step) => step,
                Err(error) => return Ok((walk, Some((index, error)))),
            };
            if matches!(step, FrameStep::Sort { .. }) {
                ordering_is_declared = true;
            }
            // Seeded with the frame being edited: a union or join in this
            // draft may read a frame whose own lineage leads back here, and
            // that cycle exists only once this chain is saved — so the
            // persisted graph cannot show it, and the walk has to refuse it
            // before it becomes a document no plan can run.
            let mut visiting = HashSet::new();
            visiting.insert(frame_id.to_string());
            plan = match self.apply_step(plan, &step, &mut visiting) {
                Ok(plan) => plan,
                Err(error) => {
                    let error = CoreError::Transform(in_plain_words(error));
                    return Ok((walk, Some((index, error))));
                }
            };
            // The plan is the authority on what exists and what type it is;
            // asking it costs no scan.
            let schema = match plan.collect_schema() {
                Ok(schema) => schema,
                Err(error) => {
                    let error = CoreError::Transform(in_plain_words(error.to_string()));
                    return Ok((walk, Some((index, error))));
                }
            };
            // How a number is written is this document's fact, not Polars':
            // it stores Float64 for a price and Float64 for the sum of
            // prices alike. So the step's own expressions are asked, against
            // the schema going in, and a column of money summed by a group
            // comes out the other side still money.
            let mut written: HashMap<Id, DataType> = step_outputs(&step)
                .filter_map(|(output_column_id, expression)| {
                    Some((
                        output_column_id.clone(),
                        expression.declared_type_among(self, &visible)?,
                    ))
                })
                .collect();
            // A pivot's and an unpivot's outputs have no formula to ask, but
            // the same fact holds: a grid of summed prices is still money.
            // A pivot cell is written the way the values column was; an
            // unpivot's value column keeps its notation only when every
            // melted column agrees on one, because a column holding prices
            // in some rows and percentages in others is written as neither.
            match &step {
                FrameStep::Pivot {
                    values_column_id,
                    aggregate,
                    outputs,
                    ..
                } if !matches!(aggregate, PivotAggregate::Count) => {
                    if let Some(values_column) =
                        visible.iter().find(|column| &column.id == values_column_id)
                    {
                        for output in outputs {
                            written
                                .insert(output.output_column_id.clone(), values_column.data_type);
                        }
                    }
                }
                FrameStep::Unpivot {
                    columns,
                    value_column_id,
                    ..
                } => {
                    let mut melted_types = columns.iter().filter_map(|melted| {
                        visible
                            .iter()
                            .find(|column| column.id == melted.column_id)
                            .map(|column| column.data_type)
                    });
                    if let Some(first) = melted_types.next()
                        && melted_types.all(|data_type| data_type == first)
                    {
                        written.insert(value_column_id.clone(), first);
                    }
                }
                _ => {}
            }
            let carried: HashMap<&str, DataType> = visible
                .iter()
                .map(|column| (column.id.as_str(), column.data_type))
                .collect();
            let carried_sources: HashMap<Id, Option<String>> = visible
                .iter()
                .map(|column| (column.id.clone(), column.source_name.clone()))
                .collect();
            visible = schema
                .iter()
                .map(|(column_id, dtype)| {
                    let column_id = column_id.to_string();
                    let found = framework_type_from_polars(dtype).map_err(CoreError::Import)?;
                    Ok(Column {
                        name: names
                            .get(&column_id)
                            .cloned()
                            .unwrap_or_else(|| column_id.clone()),
                        source_name: carried_sources.get(&column_id).cloned().flatten(),
                        // A column this step did not touch keeps what it was
                        // written as on the way in; one it produced is asked
                        // of its formula.
                        data_type: written_type(
                            found,
                            written
                                .get(&column_id)
                                .or_else(|| carried.get(column_id.as_str()))
                                .copied(),
                        ),
                        categories: declared_categories(dtype),
                        format: retained_format(&column_id),
                        formula: None,
                        id: column_id,
                    })
                })
                .collect::<Result<Vec<_>, CoreError>>()?;
            walk.steps.push(step);
            walk.schemas.push(visible.clone());
            walk.plan = plan.clone();
            if stop_after == Some(index) {
                break;
            }
        }
        Ok((walk, None))
    }

    /// One step, parsed against `visible` — the columns the steps before it
    /// leave behind, which is what lets a step read a column an earlier one
    /// added.
    ///
    /// `plan` is the chain as far as the previous step, and exists here for
    /// exactly one reader: a pivot has to look at the data to learn what
    /// columns it makes, and this is the one moment that look happens —
    /// everything downstream reads what got baked. `edited_frame_id` and
    /// `existing_steps` describe the frame being edited, so a union can
    /// refuse to stack a frame onto itself and a re-saved pivot can keep
    /// its output ids.
    #[allow(clippy::too_many_arguments)]
    fn parse_pipeline_step(
        &self,
        input: FrameStepInput,
        scope_frame_id: &str,
        scope_name: &str,
        visible: &[Column],
        names: &mut HashMap<Id, String>,
        plan: &pl::LazyFrame,
        edited_frame_id: &str,
        existing_steps: &[FrameStep],
        ordering_is_declared: bool,
    ) -> Result<FrameStep, CoreError> {
        let scope = FrameObject {
            comment: None,
            id: scope_frame_id.to_string(),
            name: scope_name.to_string(),
            columns: visible.to_vec(),
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
        let parse_all = |items: &[ExistingFormulaInput]| {
            items
                .iter()
                .map(|item| {
                    let expression = Parser::new(&item.formula, &scope, self)?.parse()?;
                    self.validate_expression_ordering(&expression, ordering_is_declared)?;
                    expression.recurrence_parts().map_err(CoreError::Formula)?;
                    Ok(DerivedExpression {
                        output_column_id: item.output_column_id.clone(),
                        expression,
                    })
                })
                .collect::<Result<Vec<_>, CoreError>>()
        };
        let known = |column_id: &Id| visible.iter().any(|column| &column.id == column_id);

        Ok(match input {
            FrameStepInput::Filter {
                predicates,
                match_all,
            } => FrameStep::Filter {
                predicates: predicates
                    .iter()
                    .map(|formula| {
                        let expression = Parser::new(formula, &scope, self)?.parse()?;
                        self.validate_expression_ordering(&expression, ordering_is_declared)?;
                        Ok(expression)
                    })
                    .collect::<Result<Vec<_>, CoreError>>()?,
                match_all,
            },
            FrameStepInput::WithColumns { columns } => {
                for column in &columns {
                    names.insert(column.output_column_id.clone(), column.name.clone());
                }
                FrameStep::WithColumns {
                    columns: parse_all(&columns)?,
                }
            }
            FrameStepInput::Select { column_ids } => {
                if let Some(missing) = column_ids.iter().find(|id| !known(id)) {
                    return Err(CoreError::InvalidOperation(format!(
                        "Step selects ‘{}’, which nothing before it produces",
                        names
                            .get(missing)
                            .cloned()
                            .unwrap_or_else(|| missing.clone())
                    )));
                }
                FrameStep::Select { column_ids }
            }
            FrameStepInput::Summarize {
                group_keys,
                aggregates,
                maintain_order,
            } => {
                if aggregates.is_empty() {
                    return Err(CoreError::InvalidOperation(
                        "A summarize step needs at least one aggregate".into(),
                    ));
                }
                for column in group_keys.iter().chain(&aggregates) {
                    names.insert(column.output_column_id.clone(), column.name.clone());
                }
                FrameStep::Summarize {
                    group_keys: parse_all(&group_keys)?,
                    aggregates: parse_all(&aggregates)?,
                    maintain_order,
                }
            }
            FrameStepInput::Sort { keys } => {
                if let Some(missing) = keys.iter().find(|key| !known(&key.column_id)) {
                    return Err(CoreError::InvalidOperation(format!(
                        "Step sorts by ‘{}’, which nothing before it produces",
                        names
                            .get(&missing.column_id)
                            .cloned()
                            .unwrap_or_else(|| missing.column_id.clone())
                    )));
                }
                FrameStep::Sort {
                    keys: keys
                        .into_iter()
                        .map(|key| DerivedSort {
                            column_id: key.column_id,
                            descending: key.descending,
                        })
                        .collect(),
                }
            }
            FrameStepInput::Union { frame_id } => {
                self.prepare_union_step(frame_id, edited_frame_id, visible)?
            }
            FrameStepInput::Expand { frame_id } => {
                self.prepare_expand_step(frame_id, edited_frame_id, existing_steps, names)?
            }
            FrameStepInput::Pivot {
                names_column_id,
                values_column_id,
                aggregate,
            } => self.prepare_pivot_step(
                names_column_id,
                values_column_id,
                aggregate,
                visible,
                plan,
                existing_steps,
                names,
            )?,
            FrameStepInput::Unpivot {
                columns,
                name_column_id,
                name_column_name,
                value_column_id,
                value_column_name,
            } => prepare_unpivot_step(
                &columns,
                name_column_id,
                name_column_name,
                value_column_id,
                value_column_name,
                &scope,
                names,
            )?,
            FrameStepInput::Comment { text } => FrameStep::Comment { text },
        })
    }

    // Matched by name, once, right here. From this point on the mapping is
    // ids: what "the Amount column" means is decided when the person writes
    // the step, and a later rename on either side does not quietly re-route
    // rows.
    fn prepare_union_step(
        &self,
        frame_id: Id,
        edited_frame_id: &str,
        visible: &[Column],
    ) -> Result<FrameStep, CoreError> {
        if frame_id == edited_frame_id {
            return Err(CoreError::InvalidOperation(
                "A frame cannot stack itself under itself".into(),
            ));
        }
        let stacked = self.frame(&frame_id)?;
        let mapping: Vec<UnionColumn> = visible
            .iter()
            .map(|column| UnionColumn {
                column_id: column.id.clone(),
                source_column_id: stacked
                    .columns
                    .iter()
                    .find(|other| other.name == column.name)
                    .map(|other| other.id.clone()),
            })
            .collect();
        if mapping
            .iter()
            .all(|column| column.source_column_id.is_none())
        {
            return Err(CoreError::InvalidOperation(format!(
                "No column of {} shares a name with one here, so there is \
                 nothing to line its rows up under. Rename the columns that \
                 should stack.",
                stacked.name
            )));
        }
        Ok(FrameStep::Union { frame_id, mapping })
    }

    fn prepare_expand_step(
        &self,
        frame_id: Id,
        edited_frame_id: &str,
        existing_steps: &[FrameStep],
        names: &mut HashMap<Id, String>,
    ) -> Result<FrameStep, CoreError> {
        if frame_id == edited_frame_id {
            return Err(CoreError::InvalidOperation(
                "A frame cannot expand itself".into(),
            ));
        }
        let expanded = self.frame(&frame_id)?;
        if expanded.columns.is_empty() {
            return Err(CoreError::InvalidOperation(format!(
                "{} has no columns to expand with",
                expanded.name
            )));
        }
        // A value the previous save already made a column for keeps its id,
        // so formats and formulas written against the column survive the
        // re-save; only genuinely new columns mint ids.
        let mut kept: HashMap<Id, Id> = existing_steps
            .iter()
            .filter_map(|step| match step {
                FrameStep::Expand {
                    frame_id: kept_frame_id,
                    outputs,
                } if *kept_frame_id == frame_id => Some(outputs.iter().map(|output| {
                    (
                        output.source_column_id.clone(),
                        output.output_column_id.clone(),
                    )
                })),
                _ => None,
            })
            .flatten()
            .collect();
        let outputs = expanded
            .columns
            .iter()
            .map(|column| {
                let output_column_id = kept
                    .remove(&column.id)
                    .unwrap_or_else(|| column_id(&column.name));
                names.insert(output_column_id.clone(), column.name.clone());
                ExpandOutput {
                    output_column_id,
                    source_column_id: column.id.clone(),
                }
            })
            .collect();
        Ok(FrameStep::Expand { frame_id, outputs })
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_pivot_step(
        &self,
        names_column_id: Id,
        values_column_id: Id,
        aggregate: PivotAggregate,
        visible: &[Column],
        plan: &pl::LazyFrame,
        existing_steps: &[FrameStep],
        names: &mut HashMap<Id, String>,
    ) -> Result<FrameStep, CoreError> {
        let names_column = visible
            .iter()
            .find(|column| column.id == names_column_id)
            .ok_or_else(|| {
                CoreError::InvalidOperation(
                    "Step pivots on a column nothing before it produces".into(),
                )
            })?;
        let values_column = visible
            .iter()
            .find(|column| column.id == values_column_id)
            .ok_or_else(|| {
                CoreError::InvalidOperation(
                    "Step pivots a column nothing before it produces".into(),
                )
            })?;
        if names_column_id == values_column_id {
            return Err(CoreError::InvalidOperation(
                "A pivot needs two different columns: one to name the new \
                 columns, one to fill them"
                    .into(),
            ));
        }
        // Text, categories, and dates. A column of floats can name columns
        // in principle, but its values arrive here as renderings, and `1.5`
        // rendered, compared, and re-rendered is exactly the kind of round
        // trip that drifts. A date is different: `2026-09-01` has exactly
        // one spelling in this document, so it survives the round trip —
        // and a column per day of a period is the commonest wide layout
        // there is. The person who wants numbers can add a text column
        // first.
        if !matches!(
            names_column.data_type,
            DataType::String | DataType::Categorical | DataType::Date
        ) {
            return Err(CoreError::InvalidOperation(format!(
                "‘{}’ holds {}s. New columns need names, so pivot on a text \
                 column, a date column, or a list of allowed values.",
                names_column.name,
                data_type_name(names_column.data_type)
            )));
        }
        if matches!(aggregate, PivotAggregate::Sum | PivotAggregate::Mean)
            && !matches!(
                values_column.data_type,
                DataType::Integer | DataType::Number | DataType::Currency | DataType::Percentage
            )
        {
            return Err(CoreError::InvalidOperation(format!(
                "‘{}’ holds {}s, which cannot be summed or averaged. Count \
                 them, take the first, or pick a number column.",
                values_column.name,
                data_type_name(values_column.data_type)
            )));
        }
        // The one look at the data. Distinct values of one column, sorted —
        // a declared list of allowed values sorts in its declared order,
        // which is the order its person chose — and capped, because a pivot
        // that makes four thousand columns has answered the wrong question.
        const WIDEST_PIVOT: usize = 100;
        let distinct = plan
            .clone()
            .select([pl::col(names_column_id.as_str())])
            .filter(pl::col(names_column_id.as_str()).is_not_null())
            .unique(None, pl::UniqueKeepStrategy::Any)
            .sort_by_exprs(
                vec![pl::col(names_column_id.as_str())],
                pl::SortMultipleOptions::default(),
            )
            .limit((WIDEST_PIVOT + 1) as pl::IdxSize)
            .collect()
            .map_err(|error| CoreError::Transform(in_plain_words(error.to_string())))?;
        if distinct.height() > WIDEST_PIVOT {
            return Err(CoreError::InvalidOperation(format!(
                "‘{}’ holds more than {WIDEST_PIVOT} different values, and a \
                 frame that wide stops being readable. Filter or group it \
                 first.",
                names_column.name
            )));
        }
        if distinct.height() == 0 {
            return Err(CoreError::InvalidOperation(format!(
                "‘{}’ holds no values yet, so there are no columns to make.",
                names_column.name
            )));
        }
        let values_as_text = distinct
            .column(names_column_id.as_str())
            .map_err(|error| CoreError::Transform(error.to_string()))?
            .as_materialized_series()
            .cast(&pl::DataType::String)
            .map_err(|error| CoreError::Transform(error.to_string()))?;
        let strings = values_as_text
            .str()
            .map_err(|error| CoreError::Transform(error.to_string()))?;
        // A value the previous save already made a column for keeps its id,
        // so formats and formulas written against the column survive the
        // re-save; only genuinely new values mint ids.
        let mut kept: HashMap<String, Id> = existing_steps
            .iter()
            .filter_map(|step| match step {
                FrameStep::Pivot {
                    names_column_id: kept_names,
                    values_column_id: kept_values,
                    outputs,
                    ..
                } if *kept_names == names_column_id && *kept_values == values_column_id => Some(
                    outputs
                        .iter()
                        .map(|output| (output.value.clone(), output.output_column_id.clone())),
                ),
                _ => None,
            })
            .flatten()
            .collect();
        let outputs: Vec<PivotOutput> = strings
            .iter()
            .flatten()
            .map(|value| {
                let output_column_id = kept.remove(value).unwrap_or_else(|| column_id(value));
                names.insert(output_column_id.clone(), value.to_string());
                PivotOutput {
                    output_column_id,
                    value: value.to_string(),
                }
            })
            .collect();
        Ok(FrameStep::Pivot {
            names_column_id,
            values_column_id,
            aggregate,
            outputs,
        })
    }

    /// Re-saves a frame's chain exactly as it stands, so every step whose
    /// outputs are discovered from data — a pivot's columns, an expansion's
    /// outputs, a union's mapping — re-discovers them against the data as it
    /// is *now*. Output ids for values and columns that survive are kept by
    /// the same rules an ordinary re-save keeps them, so formulas written
    /// against the old columns keep meaning what they meant.
    ///
    /// This exists because a pivot bakes its columns when the step is
    /// written (a frame whose schema drifts under its readers is not a
    /// frame), which leaves one honest way to follow a parameter change:
    /// write the step again. This operation is that re-write, without
    /// asking anyone to re-author a chain they already have.
    pub(crate) fn prepare_refresh_frame_pipeline(
        &self,
        frame_id: Id,
    ) -> Result<ReplicatedOperation, CoreError> {
        let frame = self.frame(&frame_id)?;
        let (input_columns, chain): (&[Column], Vec<FrameStep>) = match &frame.derivation {
            Some(derivation) => {
                let steps = derivation.steps().into_owned();
                if derivation.join.is_some() {
                    let editable = steps
                        .strip_prefix(&[FrameStep::Join {
                            join: derivation.join.clone().expect("checked above"),
                        }])
                        .unwrap_or(&steps)
                        .to_vec();
                    let input = if frame.base_columns.is_empty() {
                        &frame.columns
                    } else {
                        &frame.base_columns
                    };
                    (input, editable)
                } else {
                    let source = self.frame(&derivation.source_frame_id)?;
                    (&source.columns, steps)
                }
            }
            None => (frame.input_columns(), frame.steps.clone()),
        };
        if chain.is_empty() {
            return Err(CoreError::InvalidOperation(
                "This frame has no transformation chain to refresh".into(),
            ));
        }
        let rendered = frame.render_steps(self, input_columns, &chain);
        let inputs = rendered
            .iter()
            .map(|step| rendered_step_input(step, frame))
            .collect::<Result<Vec<_>, _>>()?;
        self.prepare_set_frame_pipeline(frame_id, inputs)
    }

    pub(crate) fn prepare_set_frame_pipeline(
        &self,
        frame_id: Id,
        steps: Vec<FrameStepInput>,
    ) -> Result<ReplicatedOperation, CoreError> {
        let fixed_join = self
            .frame(&frame_id)?
            .derivation
            .as_ref()
            .and_then(|derivation| derivation.join.clone());
        let (walk, failure) = self.walk_pipeline(&frame_id, steps, None)?;
        if let Some((_, error)) = failure {
            return Err(error);
        }
        let mut visible = walk
            .schemas
            .last()
            .cloned()
            .unwrap_or_else(|| walk.input_columns.clone());
        // Names are formula addresses, not decoration. Two visible columns
        // with the same one make a backtick reference ambiguous, so retain
        // the requested spelling for the first and number later collisions
        // deterministically. The editor normally prevents this before the
        // request arrives; doing it here keeps every client and older draft
        // subject to the same invariant.
        let mut used_names = HashSet::new();
        for column in &mut visible {
            let base = column.name.clone();
            if used_names.insert(base.clone()) {
                continue;
            }
            let blank_number = base
                .strip_prefix("Column ")
                .and_then(|suffix| suffix.parse::<usize>().ok());
            let (root, separator, mut suffix) = match blank_number {
                Some(number) => ("Column".to_string(), " ", number + 1),
                None => column
                    .name
                    .rsplit_once('_')
                    .and_then(|(root, suffix)| {
                        suffix
                            .parse::<usize>()
                            .ok()
                            .map(|suffix| (root.to_string(), "_", suffix + 1))
                    })
                    .unwrap_or_else(|| (base.clone(), "_", 2)),
            };
            loop {
                let candidate = format!("{root}{separator}{suffix}");
                if used_names.insert(candidate.clone()) {
                    column.name = candidate;
                    break;
                }
                suffix += 1;
            }
        }
        // An entry column is not produced by any step, so the walked schema
        // cannot know it. It survives a chain edit as long as the columns
        // that key it survive; the apply prunes any whose keys are gone.
        {
            let frame = self.frame(&frame_id)?;
            for entry_column in &frame.entry_columns {
                let keys_survive = entry_column
                    .key_column_ids
                    .iter()
                    .all(|key| visible.iter().any(|column| column.id == *key));
                let already = visible
                    .iter()
                    .any(|column| column.id == entry_column.column_id);
                if keys_survive
                    && !already
                    && let Some(column) = frame
                        .columns
                        .iter()
                        .find(|column| column.id == entry_column.column_id)
                {
                    visible.push(column.clone());
                }
            }
        }
        let PipelineWalk {
            source_frame_id,
            input_columns,
            steps: parsed_steps,
            ..
        } = walk;
        if parsed_steps.is_empty() && input_columns.is_empty() {
            return Err(CoreError::InvalidOperation(
                "A frame needs at least one column".into(),
            ));
        }
        if !parsed_steps.is_empty() && visible.is_empty() {
            return Err(CoreError::InvalidOperation(
                "A frame needs at least one column".into(),
            ));
        }
        let prepared = match source_frame_id {
            Some(source_frame_id) => ReplicatedOperation::SetFrameDerivation {
                frame_id: frame_id.clone(),
                name: self.frame(&frame_id)?.name.clone(),
                columns: visible,
                derivation: FrameDerivation {
                    source_frame_id,
                    join: fixed_join.clone(),
                    steps: match fixed_join {
                        Some(join) if !parsed_steps.is_empty() => {
                            std::iter::once(FrameStep::Join { join })
                                .chain(parsed_steps)
                                .collect()
                        }
                        _ => parsed_steps,
                    },
                },
            },
            // Clearing the chain returns the frame to showing its own data,
            // so the split between input and output schema goes away with it
            // rather than lingering as dead state.
            None if parsed_steps.is_empty() => ReplicatedOperation::SetFrameSteps {
                frame_id,
                columns: input_columns,
                base_columns: Vec::new(),
                steps: Vec::new(),
            },
            None => ReplicatedOperation::SetFrameSteps {
                frame_id,
                columns: visible,
                base_columns: input_columns,
                steps: parsed_steps,
            },
        };
        self.smoke_prepared_chain(&prepared)?;
        Ok(prepared)
    }

    /// Runs one row of the chain being saved, so a plan that cannot execute
    /// is refused *here*, with its real error, instead of being written into
    /// the document and failing on every read after.
    ///
    /// The schema walk above answers what columns a chain makes without
    /// touching a row — which is right for interactivity, and blind to the
    /// class of failure that only execution finds: an expression whose
    /// *types* line up but whose values cannot be produced, a duration text
    /// that does not parse, an offset handed a number. A saved chain like
    /// that is worse than a refused one, because the person who wrote it has
    /// walked away by the time it detonates, and undo is the only way out.
    /// One row bounds the cost: `limit(1)` pushes into the plan, so this is
    /// an existence proof, not a second full read.
    fn smoke_prepared_chain(&self, prepared: &ReplicatedOperation) -> Result<(), CoreError> {
        let frame_id = match prepared {
            ReplicatedOperation::SetFrameDerivation { frame_id, .. }
            | ReplicatedOperation::SetFrameSteps { frame_id, .. } => frame_id.clone(),
            _ => return Ok(()),
        };
        let mut probe = self.clone();
        probe.apply_replicated(prepared.clone())?;
        probe
            .materialize_frame_lazy(&frame_id, Layer::Data, &mut Default::default())
            .and_then(|plan| {
                plan.limit(1)
                    .collect()
                    .map_err(|error| crate::engine::in_plain_words(error.to_string()))
            })
            .map_err(CoreError::Formula)?;
        Ok(())
    }
}

/// A chain walked one step at a time, with the schema at every position.
pub(crate) struct PipelineWalk {
    pub(crate) source_frame_id: Option<Id>,
    /// What the first step sees.
    pub(crate) input_columns: Vec<Column>,
    pub(crate) steps: Vec<FrameStep>,
    /// Columns visible *after* each parsed step, one entry per step.
    pub(crate) schemas: Vec<Vec<Column>>,
    /// The plan as far as the walk got. Nothing has run: building a plan
    /// and running one are separate, which is what lets a sample be a
    /// `.limit()` on top rather than a scan.
    pub(crate) plan: polars::prelude::LazyFrame,
}

// The written list resolves against `scope` — the columns this step can
// see — so an entry naming something no earlier step produces is refused by
// the resolution itself, with the name in the sentence.
#[allow(clippy::too_many_arguments)]
fn prepare_unpivot_step(
    columns: &str,
    name_column_id: Id,
    name_column_name: String,
    value_column_id: Id,
    value_column_name: String,
    scope: &FrameObject,
    names: &mut HashMap<Id, String>,
) -> Result<FrameStep, CoreError> {
    let column_ids = parse_column_list(columns, scope)?;
    if column_ids.is_empty() {
        return Err(CoreError::InvalidOperation(
            "Name at least one column to unpivot".into(),
        ));
    }
    if name_column_name.trim().is_empty() || value_column_name.trim().is_empty() {
        return Err(CoreError::InvalidOperation(
            "The name and value columns each need a name".into(),
        ));
    }
    let columns = column_ids
        .into_iter()
        .map(|column_id| UnpivotColumn {
            // The label is the column's display name as it stands now,
            // captured because the plan will only ever know the id.
            label: names
                .get(&column_id)
                .cloned()
                .unwrap_or_else(|| column_id.clone()),
            column_id,
        })
        .collect();
    names.insert(name_column_id.clone(), name_column_name);
    names.insert(value_column_id.clone(), value_column_name);
    Ok(FrameStep::Unpivot {
        columns,
        name_column_id,
        value_column_id,
    })
}

/// One rendered step written back as the input a fresh save would take —
/// the same round trip the chain editor performs on every save, done here
/// so a refresh needs no editor. Baked outputs are deliberately dropped:
/// re-discovering them against current data is the entire point, and the
/// save's own keep-by-value rules preserve the ids that survive.
fn rendered_step_input(
    step: &RenderedFrameStep,
    frame: &FrameObject,
) -> Result<FrameStepInput, CoreError> {
    let name_of = |column_id: &str| {
        frame
            .columns
            .iter()
            .find(|column| column.id == column_id)
            .map(|column| column.name.clone())
            .unwrap_or_else(|| column_id.to_string())
    };
    let formula_inputs = |columns: &[RenderedDerivedExpression]| {
        columns
            .iter()
            .map(|column| ExistingFormulaInput {
                output_column_id: column.output_column_id.clone(),
                name: name_of(&column.output_column_id),
                formula: column.formula.clone(),
            })
            .collect()
    };
    Ok(match step {
        RenderedFrameStep::Filter {
            predicates,
            match_all,
        } => FrameStepInput::Filter {
            predicates: predicates.clone(),
            match_all: *match_all,
        },
        RenderedFrameStep::WithColumns { columns } => FrameStepInput::WithColumns {
            columns: formula_inputs(columns),
        },
        RenderedFrameStep::Select { column_ids } => FrameStepInput::Select {
            column_ids: column_ids.clone(),
        },
        RenderedFrameStep::Summarize {
            group_keys,
            aggregates,
            maintain_order,
        } => FrameStepInput::Summarize {
            group_keys: formula_inputs(group_keys),
            aggregates: formula_inputs(aggregates),
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
        // The melt list is rewritten as the exact columns the saved step
        // holds. A written selector such as starts_with(...) was baked to
        // these columns when the step was saved, so this is what the step
        // means now — a refresh re-reads data, never re-runs selectors.
        RenderedFrameStep::Unpivot {
            columns,
            name_column_id,
            name_column_name,
            value_column_id,
            value_column_name,
        } => FrameStepInput::Unpivot {
            columns: columns
                .iter()
                .map(|column| format!("`{}`", column.label.replace('`', "")))
                .collect::<Vec<_>>()
                .join(", "),
            name_column_id: name_column_id.clone(),
            name_column_name: name_column_name.clone(),
            value_column_id: value_column_id.clone(),
            value_column_name: value_column_name.clone(),
        },
        RenderedFrameStep::Comment { text } => FrameStepInput::Comment { text: text.clone() },
        RenderedFrameStep::Join { .. } => {
            return Err(CoreError::InvalidOperation(
                "A join derivation refreshes through its join editor".into(),
            ));
        }
    })
}

/// The columns a step makes, with the formulas that make them. `Filter`,
/// `Select`, `Join` and `Sort` name columns rather than producing any, so
/// they hold none — what passes through them is written the way it arrived.
fn step_outputs(step: &FrameStep) -> Box<dyn Iterator<Item = (&Id, &Expr)> + '_> {
    fn pair(derived: &DerivedExpression) -> (&Id, &Expr) {
        (&derived.output_column_id, &derived.expression)
    }
    match step {
        FrameStep::WithColumns { columns } => Box::new(columns.iter().map(pair)),
        FrameStep::Summarize {
            group_keys,
            aggregates,
            ..
        } => Box::new(group_keys.iter().chain(aggregates).map(pair)),
        FrameStep::Filter { .. }
        | FrameStep::Select { .. }
        | FrameStep::Join { .. }
        | FrameStep::Sort { .. }
        | FrameStep::Union { .. }
        | FrameStep::Expand { .. }
        | FrameStep::Pivot { .. }
        | FrameStep::Unpivot { .. }
        | FrameStep::Comment { .. } => Box::new(std::iter::empty()),
    }
}
