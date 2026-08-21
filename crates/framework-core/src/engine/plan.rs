//! Building and running the Polars plan behind a frame.

use crate::*;
use polars::prelude as pl;
use std::borrow::Cow;
use std::collections::HashSet;

/// Column name for the row index a page read carries between the data and
/// display layers. Not a column id, so it cannot collide with one.
const ROW_INDEX: &str = "__framework_row";

/// A page never exceeds 1,000 rows and never runs past the end.
fn page_limit(offset: usize, limit: usize, total_rows: usize) -> usize {
    limit.min(total_rows.saturating_sub(offset)).min(1000)
}

fn sorts_in(steps: &[FrameStep]) -> bool {
    steps
        .iter()
        .any(|step| matches!(step, FrameStep::Sort { keys } if !keys.is_empty()))
}

/// The type an expression would produce against a plan, read off the
/// schema — no rows.
///
/// `None` when the plan cannot answer at all, which is a different failure
/// and one the caller is already about to report.
fn expression_type(plan: &pl::LazyFrame, expression: &pl::Expr) -> Option<pl::DataType> {
    let mut projected = plan.clone().select([expression.clone()]);
    let schema = projected.collect_schema().ok()?;
    schema.iter().next().map(|(_, dtype)| dtype.clone())
}

/// Refuses a filter condition that does not answer true or false.
///
/// Polars catches this too, but only once it resolves the plan, and it
/// reports it by printing that plan — so a condition that is merely
/// unfinished comes back as a wall of column ids that names neither the
/// condition nor the column anyone typed. The schema already knows the
/// answer and costs no scan, so it is asked here instead, while the step
/// and the position of the condition within it are still in hand.
fn check_predicate_type(
    plan: &pl::LazyFrame,
    expression: &pl::Expr,
    index: usize,
    total: usize,
) -> Result<(), String> {
    let Some(data_type) = expression_type(plan, expression) else {
        return Ok(());
    };
    if matches!(data_type, pl::DataType::Boolean) {
        return Ok(());
    }
    let produces = framework_type_from_polars(&data_type)
        .map(|kind| format!("a {}", data_type_name(kind)))
        .unwrap_or_else(|_| "something else".to_string());
    let condition = if total > 1 {
        format!("Condition {}", index + 1)
    } else {
        "This condition".to_string()
    };
    Err(format!(
        "{condition} has to be a yes/no test, but it produces {produces}. \
         Compare it against something, so that every row answers true or false."
    ))
}

/// Reads both sides of a join as text when their keys declare different
/// lists of allowed values.
///
/// A declared list is an order, and two different orders have no shared one
/// — so Polars refuses to compare them, which is the right answer for `<`
/// and the wrong one for a join. A join asks whether two labels are the same
/// label, and that question has an answer no matter which lists they came
/// from. Matching lists are left alone, so a key that means the same thing on
/// both sides stays itself all the way through.
fn match_key_types(
    plan: pl::LazyFrame,
    lookup: pl::LazyFrame,
    primary_key_column_ids: &[Id],
    lookup_key_column_ids: &[Id],
) -> Result<(pl::LazyFrame, pl::LazyFrame), String> {
    let mut plan = plan;
    let mut lookup = lookup;
    let primary_schema = plan.collect_schema().map_err(|error| error.to_string())?;
    let lookup_schema = lookup.collect_schema().map_err(|error| error.to_string())?;
    let mismatched: Vec<(&Id, &Id)> = primary_key_column_ids
        .iter()
        .zip(lookup_key_column_ids)
        .filter(|(primary, lookup)| {
            let (Some(left), Some(right)) = (
                primary_schema.get(primary.as_str()),
                lookup_schema.get(lookup.as_str()),
            ) else {
                return false;
            };
            left != right && (is_category_type(left) || is_category_type(right))
        })
        .collect();
    if mismatched.is_empty() {
        return Ok((plan, lookup));
    }
    let as_text = |ids: Vec<&Id>| {
        ids.into_iter()
            .map(|id| pl::col(id.as_str()).cast(pl::DataType::String))
            .collect::<Vec<_>>()
    };
    let (primary_keys, lookup_keys): (Vec<&Id>, Vec<&Id>) = mismatched.into_iter().unzip();
    Ok((
        plan.with_columns(as_text(primary_keys)),
        lookup.with_columns(as_text(lookup_keys)),
    ))
}

fn is_category_type(data_type: &pl::DataType) -> bool {
    matches!(
        data_type,
        pl::DataType::Enum(_, _) | pl::DataType::Categorical(_, _)
    )
}

/// A Polars error with its plan dump cut off.
///
/// Polars appends the whole resolved plan to a failure — every projection,
/// every column by id — and the ids are the part nobody can read, because
/// they are not the names anyone typed. The line before it says what went
/// wrong, which is the part worth showing.
fn without_plan_dump(message: impl Into<String>) -> String {
    let message = message.into();
    match message.split_once("Resolved plan until failure:") {
        Some((reason, _)) if !reason.trim().is_empty() => reason.trim_end().to_string(),
        _ => message,
    }
}

/// A Polars failure in the words this app uses for the thing that failed.
///
/// Most of what Polars says is already the clearest account of what went
/// wrong, and is passed along. The exceptions are where it names a concept
/// this app named differently — a list of allowed values is an "enum" there,
/// and the fix it suggests is a cast, which is not a word anyone using this
/// has been given.
pub(crate) fn in_plain_words(message: impl Into<String>) -> String {
    let message = without_plan_dump(message);
    if message.contains("Enum mismatch") {
        return "These columns allow different sets of values, so a comparison between \
                them has no one order to read along. Give them the same allowed values."
            .to_string();
    }
    message
}

/// Which of a frame's two layers a plan should include.
///
/// The whole propagation rule is one line further down: every recursive call
/// in [`Document::materialize_frame_lazy`] passes [`Layer::Data`]. A frame
/// reading its own rows sees its display filter and sort; a frame reading
/// *another* frame never does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// The wrangle chain alone — what the frame is, and what anything
    /// derived from it reads.
    Data,
    /// The wrangle chain with the display filter and sort on top — what the
    /// frame's own reads show.
    Display,
}

impl Document {
    pub(crate) fn materialize_frame_lazy(
        &self,
        frame_id: &str,
        layer: Layer,
        visiting: &mut HashSet<Id>,
    ) -> Result<pl::LazyFrame, String> {
        let plan = self.materialize_data_layer(frame_id, visiting)?;
        match layer {
            Layer::Data => Ok(plan),
            Layer::Display => {
                self.apply_display_layer(plan, self.frame(frame_id).map_err(|e| e.to_string())?)
            }
        }
    }

    /// The display filter and sort on top of an already-built data plan.
    ///
    /// Separate from [`Document::materialize_frame_lazy`] only so a paged
    /// read can slip a row index in between the two layers; there is still
    /// exactly one place display steps are applied.
    pub(crate) fn apply_display_layer(
        &self,
        plan: pl::LazyFrame,
        frame: &FrameObject,
    ) -> Result<pl::LazyFrame, String> {
        let mut plan = plan;
        for step in &frame.display.steps {
            plan = self.apply_step(plan, step, &mut HashSet::new())?;
        }
        Ok(plan)
    }

    fn materialize_data_layer(
        &self,
        frame_id: &str,
        visiting: &mut HashSet<Id>,
    ) -> Result<pl::LazyFrame, String> {
        let frame = self.frame(frame_id).map_err(|error| error.to_string())?;
        // A cached frame reads its snapshot instead of re-running anything.
        // The parquet was written with column *ids* as its names, so it needs
        // none of the renaming a source artifact does -- and it is read as
        // written even when stale, which is what "stale" means here: the
        // rows on screen keep being the snapshot's until someone refreshes.
        //
        // Nothing is added to `visiting` on this path, and nothing needs to
        // be: reading a file is where a lineage stops rather than another
        // step along it.
        if let Some(materialization) = &frame.materialization {
            return pl::LazyFrame::scan_parquet(
                pl::PlRefPath::new(&materialization.artifact.path),
                pl::ScanArgsParquet::default(),
            )
            .map_err(|error| error.to_string());
        }
        self.recompute_data_layer(frame_id, visiting)
    }

    /// The data layer worked out from the frame's own definition, ignoring
    /// any snapshot it holds — what refreshing one has to compute, since
    /// reading the snapshot would only copy it back over itself.
    ///
    /// Only *this* frame's snapshot is set aside. Everything below still
    /// reads its own, which is what keeps a refresh from running away: a
    /// frame whose lineage passes back through a formula that reads this one
    /// finds the snapshot that is being replaced, sitting there as the
    /// recorded value it is, and stops.
    pub(crate) fn recompute_data_layer(
        &self,
        frame_id: &str,
        visiting: &mut HashSet<Id>,
    ) -> Result<pl::LazyFrame, String> {
        if !visiting.insert(frame_id.to_string()) {
            return Err("Circular derived-frame dependency".into());
        }
        let frame = self.frame(frame_id).map_err(|error| error.to_string())?;
        let result = if let Some(derivation) = &frame.derivation {
            // `Layer::Data`, always: this is the one line that keeps a
            // display filter from leaking into everything downstream.
            let mut plan = self.materialize_data_layer(&derivation.source_frame_id, visiting)?;
            let steps = derivation.steps();
            let joins_lookup = steps
                .iter()
                .any(|step| matches!(step, FrameStep::Join { .. }));
            for step in steps.iter() {
                plan = self.apply_step(plan, step, visiting)?;
            }
            // Entered values join on after the chain: the chain makes the
            // rows, the entries decorate them by key.
            let plan = self.apply_entry_columns(plan, frame)?;
            // The declared columns are the frame's schema contract, so the
            // chain ends by selecting exactly them. A legacy join derivation
            // already selected its outputs and never carried declared
            // columns matching them, so it keeps the join's own projection.
            let result = if joins_lookup && derivation.steps.is_empty() {
                Ok(plan)
            } else {
                Ok(plan.select(
                    frame
                        .columns
                        .iter()
                        .map(|column| pl::col(column.id.clone()))
                        .collect::<Vec<_>>(),
                ))
            };
            visiting.remove(frame_id);
            return result;
        } else {
            // A frame with no derivation still gets its own chain, run on
            // top of its own data. Same steps, same evaluator, same closing
            // select onto the declared columns -- only the starting plan
            // differs, which is the whole distinction between a source frame
            // and a derived one.
            frame.materialize_polars_lazy(self).and_then(|plan| {
                if frame.steps.is_empty() && frame.entry_columns.is_empty() {
                    return Ok(plan);
                }
                let mut plan = plan;
                for step in &frame.steps {
                    plan = self.apply_step(plan, step, visiting)?;
                }
                let plan = self.apply_entry_columns(plan, frame)?;
                Ok(plan.select(
                    frame
                        .columns
                        .iter()
                        .map(|column| pl::col(column.id.clone()))
                        .collect::<Vec<_>>(),
                ))
            })
        };
        visiting.remove(frame_id);
        result
    }

    /// Joins each entry column's stored values onto the computed rows, by
    /// the entry column's own key columns.
    ///
    /// A left join, deliberately: a row nobody has entered anything for
    /// shows blank, and an entry whose row has gone shows nowhere — it
    /// stays stored, waiting for the key to come back, which is the whole
    /// contract of an entry column. Many-to-one validation holds because
    /// entries are upserted by key, so each key appears once.
    fn apply_entry_columns(
        &self,
        plan: pl::LazyFrame,
        frame: &FrameObject,
    ) -> Result<pl::LazyFrame, String> {
        use polars::prelude::IntoLazy;
        let mut plan = plan;
        for entry_column in &frame.entry_columns {
            let column = frame
                .columns
                .iter()
                .find(|column| column.id == entry_column.column_id)
                .ok_or("An entry column is missing from this frame's columns")?;
            if entry_column.entries.is_empty() {
                // The column has to exist even before the first entry: it is
                // part of the schema the closing select is contracted to.
                plan = plan.with_column(
                    pl::lit(pl::NULL)
                        .cast(polars_type_for(column.data_type))
                        .alias(&column.id),
                );
                continue;
            }
            let mut lookup_columns = Vec::new();
            for (index, key_column_id) in entry_column.key_column_ids.iter().enumerate() {
                let key_column = frame
                    .columns
                    .iter()
                    .find(|column| column.id == *key_column_id)
                    .ok_or("An entry column's key column is missing from this frame")?;
                lookup_columns.push(typed_series(
                    key_column_id,
                    key_column.data_type,
                    entry_column
                        .entries
                        .iter()
                        .map(|entry| entry.key.get(index).map(String::as_str).unwrap_or_default()),
                )?);
            }
            lookup_columns.push(typed_series(
                &column.id,
                column.data_type,
                entry_column.entries.iter().map(|entry| entry.raw.as_str()),
            )?);
            let lookup = pl::DataFrame::new(
                entry_column.entries.len(),
                lookup_columns.into_iter().map(Into::into).collect(),
            )
            .map_err(|error| error.to_string())?
            .lazy();
            let (matched_plan, lookup) = match_key_types(
                plan,
                lookup,
                &entry_column.key_column_ids,
                &entry_column.key_column_ids,
            )?;
            let mut arguments = pl::JoinArgs::new(pl::JoinType::Left);
            arguments.validation = pl::JoinValidation::ManyToOne;
            arguments.maintain_order = pl::MaintainOrderJoin::Left;
            let keys = entry_column
                .key_column_ids
                .iter()
                .map(pl::col)
                .collect::<Vec<_>>();
            plan = matched_plan.join(lookup, keys.clone(), keys, arguments);
        }
        Ok(plan)
    }

    pub(crate) fn materialize_frame_frame(
        &self,
        frame_id: &str,
        layer: Layer,
        visiting: &mut HashSet<Id>,
    ) -> Result<pl::DataFrame, String> {
        self.materialize_frame_lazy(frame_id, layer, visiting)?
            .collect()
            .map_err(|error| format!("Polars error materializing frame: {error}"))
    }

    /// Applies one step to the plan built so far.
    ///
    /// Each step reads the schema the previous ones produced, which is what
    /// makes the chain expressive: a `with_columns` can build on a column an
    /// earlier `with_columns` added, and a filter after a summarize sees the
    /// aggregates rather than the source rows.
    pub(crate) fn apply_step(
        &self,
        plan: pl::LazyFrame,
        step: &FrameStep,
        visiting: &mut HashSet<Id>,
    ) -> Result<pl::LazyFrame, String> {
        let aliased = |items: &[DerivedExpression]| {
            items
                .iter()
                .map(|item| {
                    item.expression
                        .to_polars(self)
                        .map(|expression| expression.alias(item.output_column_id.clone()))
                })
                .collect::<Result<Vec<_>, _>>()
        };
        match step {
            // A remark for the reader; the plan passes through untouched.
            FrameStep::Comment { .. } => Ok(plan),
            FrameStep::Filter {
                predicates,
                match_all,
            } => {
                let mut compiled = Vec::with_capacity(predicates.len());
                for (index, filter) in predicates.iter().enumerate() {
                    let expression = filter.to_polars(self)?;
                    check_predicate_type(&plan, &expression, index, predicates.len())?;
                    compiled.push(expression);
                }
                let mut folded = compiled.into_iter();
                let Some(first) = folded.next() else {
                    return Ok(plan);
                };
                let predicate = folded.fold(first, |combined, filter| {
                    if *match_all {
                        combined.logical_and(filter)
                    } else {
                        combined.logical_or(filter)
                    }
                });
                Ok(plan.filter(predicate))
            }
            FrameStep::WithColumns { columns } => self.apply_with_columns_step(plan, columns),
            FrameStep::Select { column_ids } => Ok(plan.select(
                column_ids
                    .iter()
                    .map(|column_id| pl::col(column_id.clone()))
                    .collect::<Vec<_>>(),
            )),
            FrameStep::Summarize {
                group_keys,
                aggregates,
                maintain_order,
            } => {
                let keys = aliased(group_keys)?;
                let values = aliased(aggregates)?;
                // No keys means one row for the whole frame, which is a
                // plain select of the aggregates rather than a group_by.
                Ok(if keys.is_empty() {
                    plan.select(values)
                } else if *maintain_order {
                    plan.group_by_stable(keys).agg(values)
                } else {
                    plan.group_by(keys).agg(values)
                })
            }
            FrameStep::Join { join } => {
                let lookup = self.materialize_data_layer(&join.lookup_frame_id, visiting)?;
                let (plan, lookup) = match_key_types(
                    plan,
                    lookup,
                    &join.primary_key_column_ids,
                    &join.lookup_key_column_ids,
                )?;
                let mut arguments = pl::JoinArgs::new(match join.join_type {
                    FrameJoinType::Left => pl::JoinType::Left,
                    FrameJoinType::Inner => pl::JoinType::Inner,
                    FrameJoinType::Anti => pl::JoinType::Anti,
                    FrameJoinType::Semi => pl::JoinType::Semi,
                });
                // Polars only supports many-to-one validation on left and
                // inner joins; anti and semi joins cannot multiply rows, so
                // they run without it.
                if join.join_type.keeps_lookup_columns() {
                    arguments.validation = pl::JoinValidation::ManyToOne;
                }
                arguments.maintain_order = pl::MaintainOrderJoin::Left;
                let joined = plan.join(
                    lookup,
                    join.primary_key_column_ids
                        .iter()
                        .map(pl::col)
                        .collect::<Vec<_>>(),
                    join.lookup_key_column_ids
                        .iter()
                        .map(pl::col)
                        .collect::<Vec<_>>(),
                    arguments,
                );
                Ok(joined.select(
                    join.outputs
                        .iter()
                        .map(|output| {
                            pl::col(&output.source_column_id).alias(&output.output_column_id)
                        })
                        .collect::<Vec<_>>(),
                ))
            }
            FrameStep::Union { frame_id, mapping } => {
                let stacked = self.materialize_data_layer(frame_id, visiting)?;
                let mut plan = plan;
                let plan_schema = plan.collect_schema().map_err(|error| error.to_string())?;
                let mut stacked = stacked;
                let stacked_schema = stacked
                    .collect_schema()
                    .map_err(|error| error.to_string())?;
                // Both sides are selected down to the mapping, so the step's
                // output is the mapping and nothing else — the same stance
                // the chain's closing select takes. A column that appeared
                // upstream after this step was written does not stack until
                // the step is saved again, rather than stacking against
                // nothing.
                //
                // A mapped pair whose two sides declare different lists of
                // allowed values is read as text, for the same reason a
                // join's keys are (see `match_key_types`): two orders have
                // no shared one, but stacking only asks that a label stay
                // the label it is.
                let mut top = Vec::with_capacity(mapping.len());
                let mut bottom = Vec::with_capacity(mapping.len());
                for column in mapping {
                    let own_type = plan_schema.get(column.column_id.as_str());
                    match &column.source_column_id {
                        Some(source_column_id) => {
                            let stacked_type = stacked_schema.get(source_column_id.as_str());
                            let mismatched_categories =
                                own_type.zip(stacked_type).is_some_and(|(own, theirs)| {
                                    own != theirs
                                        && (is_category_type(own) || is_category_type(theirs))
                                });
                            if mismatched_categories {
                                top.push(
                                    pl::col(column.column_id.as_str()).cast(pl::DataType::String),
                                );
                                bottom.push(
                                    pl::col(source_column_id.as_str())
                                        .cast(pl::DataType::String)
                                        .alias(&column.column_id),
                                );
                            } else if let Some(own_type) = own_type {
                                // A Union column keeps the type declared by
                                // the frame above it. Imported Parquet can
                                // retain a narrower physical width (a CSV
                                // integer is often UInt32) even though its
                                // FrameWork column is Integer/Int64. Casting
                                // the incoming side explicitly avoids asking
                                // Polars to invent a supertype that disagrees
                                // with the output column's declared type.
                                top.push(pl::col(column.column_id.as_str()));
                                bottom.push(
                                    pl::col(source_column_id.as_str())
                                        .cast(own_type.clone())
                                        .alias(&column.column_id),
                                );
                            } else {
                                top.push(pl::col(column.column_id.as_str()));
                                bottom.push(
                                    pl::col(source_column_id.as_str()).alias(&column.column_id),
                                );
                            }
                        }
                        None => {
                            top.push(pl::col(column.column_id.as_str()));
                            // Typed to match the column it sits under, so the
                            // concat has nothing to reconcile.
                            bottom.push(
                                pl::lit(pl::Null {})
                                    .cast(own_type.cloned().unwrap_or(pl::DataType::Null))
                                    .alias(&column.column_id),
                            );
                        }
                    }
                }
                pl::concat(
                    [plan.select(top), stacked.select(bottom)],
                    pl::UnionArgs {
                        // Each incoming expression was cast to the schema of
                        // the frame above it. Letting concat find another
                        // supertype can look through those projections and
                        // reintroduce a file scan's physical integer width.
                        to_supertypes: false,
                        ..Default::default()
                    },
                )
                .map_err(|error| error.to_string())
            }
            FrameStep::Expand { frame_id, outputs } => {
                let expanded = self.materialize_data_layer(frame_id, visiting)?;
                let expanded = expanded.select(
                    outputs
                        .iter()
                        .map(|output| {
                            pl::col(output.source_column_id.as_str())
                                .alias(&output.output_column_id)
                        })
                        .collect::<Vec<_>>(),
                );
                Ok(plan.cross_join(expanded, None))
            }
            FrameStep::Pivot {
                names_column_id,
                values_column_id,
                aggregate,
                outputs,
            } => {
                // Every column the pivot does not consume becomes a group
                // key, so the rows that survive are one per combination of
                // what is left — which is what "the rest of the frame" means
                // once two of its columns have been folded into a grid.
                let mut plan = plan;
                let schema = plan.collect_schema().map_err(|error| error.to_string())?;
                let index = schema
                    .iter_names()
                    .filter(|name| {
                        name.as_str() != names_column_id && name.as_str() != values_column_id
                    })
                    .map(|name| pl::col(name.clone()))
                    .collect::<Vec<_>>();
                let cells = outputs
                    .iter()
                    .map(|output| {
                        // Cast to text on the comparison side only: the baked
                        // value is a rendering, and comparing renderings is
                        // what keeps this working when the names column is a
                        // list of allowed values rather than plain text.
                        let matched = pl::col(values_column_id.as_str()).filter(
                            pl::col(names_column_id.as_str())
                                .cast(pl::DataType::String)
                                .eq(pl::lit(output.value.as_str())),
                        );
                        match aggregate {
                            PivotAggregate::Sum => matched.sum(),
                            PivotAggregate::Count => matched.count(),
                            PivotAggregate::Mean => matched.mean(),
                            PivotAggregate::Min => matched.min(),
                            PivotAggregate::Max => matched.max(),
                            PivotAggregate::First => matched.first(),
                            // The refusing policy: the cell keeps the one row
                            // it gets, and a second row landing there is an
                            // error, not something quietly combined. The
                            // check has to run per cell, which is what
                            // `apply` is — each group's matched rows arrive
                            // as one column, and a count above one is the
                            // very fact the person picked this policy to be
                            // told about.
                            PivotAggregate::None => {
                                let value = output.value.clone();
                                // `agg_with_fmt_str` rather than `apply`:
                                // both run the closure per group, but only
                                // the aggregation flavour tells the schema
                                // the result is a scalar per group rather
                                // than a list of what went in.
                                matched.agg_with_fmt_str(
                                    move |cell: pl::Column| {
                                        if cell.len() > 1 {
                                            return Err(pl::PolarsError::ComputeError(
                                                format!(
                                                    "{} rows land in ‘{value}’ for the same \
                                                     combination of the other columns, and \
                                                     this pivot was told not to combine \
                                                     rows. Choose an aggregate, or make the \
                                                     rows unique.",
                                                    cell.len()
                                                )
                                                .into(),
                                            ));
                                        }
                                        if cell.is_empty() {
                                            return Ok(pl::Column::full_null(
                                                cell.name().clone(),
                                                1,
                                                cell.dtype(),
                                            ));
                                        }
                                        Ok(cell)
                                    },
                                    |_schema, field| Ok(field.clone()),
                                    "pivot_cell",
                                )
                            }
                        }
                        .alias(&output.output_column_id)
                    })
                    .collect::<Vec<_>>();
                // No index columns means the whole frame folds to one row,
                // which is a select — the same shape a keyless summarize
                // takes, for the same reason.
                Ok(if index.is_empty() {
                    plan.select(cells)
                } else {
                    plan.group_by_stable(index).agg(cells)
                })
            }
            FrameStep::Unpivot {
                columns,
                name_column_id,
                value_column_id,
            } => {
                let mut plan = plan;
                let schema = plan.collect_schema().map_err(|error| error.to_string())?;
                let melted: HashSet<&str> = columns
                    .iter()
                    .map(|column| column.column_id.as_str())
                    .collect();
                let index = schema
                    .iter_names()
                    .filter(|name| !melted.contains(name.as_str()))
                    .map(|name| pl::col(name.clone()))
                    .collect::<Vec<_>>();
                // One pass per melted column, stacked. Polars has a native
                // unpivot, but its name column would hold column *ids* —
                // the plan knows no other names — and the whole point of
                // that column is the label a person had at the top. The
                // repeated scans collapse in optimization: common-subplan
                // elimination sees N reads of one plan.
                let parts = columns
                    .iter()
                    .map(|column| {
                        let mut selection = index.clone();
                        selection
                            .push(pl::lit(column.label.as_str()).alias(name_column_id.as_str()));
                        selection.push(
                            pl::col(column.column_id.as_str()).alias(value_column_id.as_str()),
                        );
                        plan.clone().select(selection)
                    })
                    .collect::<Vec<_>>();
                pl::concat(
                    parts,
                    pl::UnionArgs {
                        // Melted columns need not share a width — a count
                        // column and a price column unpivot together.
                        to_supertypes: true,
                        ..Default::default()
                    },
                )
                .map_err(|error| error.to_string())
            }
            FrameStep::Sort { keys } => {
                if keys.is_empty() {
                    return Ok(plan);
                }
                Ok(plan.sort_by_exprs(
                    keys.iter()
                        .map(|sort| pl::col(sort.column_id.clone()))
                        .collect::<Vec<_>>(),
                    pl::SortMultipleOptions {
                        descending: keys.iter().map(|sort| sort.descending).collect(),
                        // Nulls last in both directions, which is the
                        // spreadsheet convention an accountant expects and
                        // not the Polars default. Every sort in the product
                        // runs through here, so there is one answer.
                        nulls_last: vec![true; keys.len()],
                        maintain_order: true,
                        ..Default::default()
                    },
                ))
            }
        }
    }

    /// The columns visible to the step at `step_index`, as (id, type) pairs
    /// in plan order.
    ///
    /// Resolved from the plan rather than from data: Polars answers this
    /// from the parquet footer and the expressions themselves, so asking
    /// what a step can see costs no scan even on a frame of millions of
    /// rows.
    pub(crate) fn schema_at_step(
        &self,
        frame_id: &str,
        step_index: usize,
    ) -> Result<Vec<(Id, DataType)>, CoreError> {
        let frame = self.frame(frame_id)?;
        let mut visiting = HashSet::new();
        // Where the chain starts is the only difference between a derived
        // frame and a source one: the source frame's own data, or the plan
        // of the frame it derives from.
        let (mut plan, steps) = match &frame.derivation {
            Some(derivation) => (
                self.materialize_data_layer(&derivation.source_frame_id, &mut visiting)
                    .map_err(CoreError::Import)?,
                derivation.steps(),
            ),
            None => (
                frame
                    .materialize_polars_lazy(self)
                    .map_err(CoreError::Import)?,
                Cow::Borrowed(frame.steps.as_slice()),
            ),
        };
        for step in steps.iter().take(step_index) {
            plan = self
                .apply_step(plan, step, &mut visiting)
                .map_err(CoreError::Import)?;
        }
        let schema = plan
            .collect_schema()
            .map_err(|error| CoreError::Import(error.to_string()))?;
        schema
            .iter()
            .map(|(name, dtype)| {
                framework_type_from_polars(dtype)
                    .map(|data_type| (name.to_string(), data_type))
                    .map_err(CoreError::Import)
            })
            .collect()
    }

    /// Whether reading `frame_id` sorts: its own display sort, its own
    /// persistent sort, or one it inherits from a frame it derives from.
    ///
    /// A sorted plan cannot produce any page without ordering every row, so
    /// paging one straight off the plan re-sorts the whole frame on every
    /// page fetch -- twice over, because a derived frame has no recorded row
    /// count and `lazy_row_count` is a second full pass. Sorted reads go
    /// through the page cache instead, which pays for the ordering once.
    pub(crate) fn plan_sorts(&self, frame_id: &str) -> bool {
        // The display sort is checked here rather than in the recursion
        // because it applies to this frame's own reads only -- including on
        // top of a snapshot, which the data layer stops at.
        self.frame(frame_id)
            .is_ok_and(|frame| sorts_in(&frame.display.steps))
            || self.data_layer_sorts(frame_id, &mut HashSet::new())
    }

    fn data_layer_sorts(&self, frame_id: &str, visiting: &mut HashSet<Id>) -> bool {
        if !visiting.insert(frame_id.to_string()) {
            return false;
        }
        let sorts = self.frame(frame_id).is_ok_and(|frame| {
            // A materialized frame reads its own parquet, so whatever
            // produced it -- ordering included -- is not part of this read.
            if frame.materialization.is_some() {
                return false;
            }
            sorts_in(&frame.steps)
                || frame.derivation.as_ref().is_some_and(|derivation| {
                    sorts_in(&derivation.steps())
                        || self.data_layer_sorts(&derivation.source_frame_id, visiting)
                })
                || frame
                    .lookup_frame_ids()
                    .iter()
                    .any(|lookup_id| self.data_layer_sorts(lookup_id, visiting))
        });
        visiting.remove(frame_id);
        sorts
    }

    /// The plan behind a frame, as text, for the query-plan panel.
    pub(crate) fn frame_query_plan(&self, frame_id: &str) -> Result<FrameQueryPlan, CoreError> {
        let plan = self
            .materialize_frame_lazy(frame_id, Layer::Display, &mut HashSet::new())
            .map_err(CoreError::Import)?;
        let explain = |optimized: bool| {
            plan.explain(optimized)
                .map_err(|error| CoreError::Import(error.to_string()))
        };
        Ok(FrameQueryPlan {
            frame_id: frame_id.to_string(),
            logical: explain(false)?,
            optimized: explain(true)?,
        })
    }

    pub(crate) fn frame_depends_on_artifact(
        &self,
        frame_id: &str,
        visiting: &mut HashSet<Id>,
    ) -> bool {
        if !visiting.insert(frame_id.to_string()) {
            return false;
        }
        let depends = self.frame(frame_id).is_ok_and(|frame| {
            // A snapshot is a parquet file like any other, so a materialized
            // frame is read through pages whatever it derives from. A chain
            // is read through pages too, whatever it runs over: the rows on
            // screen are the chain's output, while `frame.rows` still holds
            // the untouched input, so the stored rows must never be the
            // thing that gets displayed.
            frame.artifact.is_some()
                || frame.source_file.is_some()
                || frame.materialization.is_some()
                || !frame.steps.is_empty()
                || frame.derivation.as_ref().is_some_and(|derivation| {
                    self.frame_depends_on_artifact(&derivation.source_frame_id, visiting)
                })
                || frame
                    .lookup_frame_ids()
                    .iter()
                    .any(|lookup_id| self.frame_depends_on_artifact(lookup_id, visiting))
        });
        visiting.remove(frame_id);
        depends
    }

    /// One page of a frame as it is displayed: wrangle chain, then display
    /// filter and sort, then the slice.
    ///
    /// Three routes to the same frame, differing only in how the slice and
    /// the row count are obtained:
    ///
    /// - **sorted** — ordering any page means ordering every row, so the
    ///   whole result is computed once into the page cache and sliced from
    ///   there. That covers a column header the user clicked and a sort
    ///   buried in a derivation equally.
    /// - **paged** — the slice pushes down into the scan, so only the page
    ///   is read. The row count comes off the artifact when nothing has
    ///   filtered it away, and costs a counting pass otherwise.
    /// - **in memory** — a small frame with no artifact behind it; collect
    ///   and slice.
    pub(crate) fn get_frame_page(
        &self,
        frame_id: &str,
        offset: usize,
        limit: usize,
        sorted_page_cache: &SortedPageCache,
    ) -> Result<FramePage, CoreError> {
        let frame = self.frame(frame_id)?;
        let paged = self.frame_depends_on_artifact(frame_id, &mut HashSet::new());
        let filtered = frame
            .display
            .filter()
            .is_some_and(|(predicates, _)| !predicates.is_empty());
        // The display layer can hide and reorder rows, so a page cannot be
        // matched back to the frame's own rows by position. A row index
        // carried between the two layers survives both, and is what the
        // returned ids are read off. Not done on the plain paged path,
        // where it would block the slice from pushing into the scan -- and
        // where rows have no editable identity to recover anyway.
        let indexed = |plan: pl::LazyFrame| {
            self.apply_display_layer(plan.with_row_index(ROW_INDEX, None), frame)
                .map_err(CoreError::Import)
        };
        let plan = || {
            self.materialize_frame_lazy(frame_id, Layer::Display, &mut HashSet::new())
                .map_err(CoreError::Import)
        };
        let data_plan = || {
            self.materialize_data_layer(frame_id, &mut HashSet::new())
                .map_err(CoreError::Import)
        };
        // A rule may ask something of the whole column -- the ends of a ramp,
        // an average to compare against -- so its hidden column belongs above
        // the slice, never over the page. Each route below says how it holds
        // to that: two of them already have every row, and the third puts the
        // columns in the plan and lets Polars push the slice through them.
        let (rule_columns, _) = frame.style_rule_columns(self);
        let mut style_matches: Option<StyleRuleMatches> = None;
        let (data_frame, total_rows) = if paged && self.plan_sorts(frame_id) {
            let entry = sorted_page_cache.get_or_compute(
                SortedPageCacheKey {
                    frame_id: frame_id.to_string(),
                    fingerprint: self.frame_fingerprint(frame_id),
                },
                || {
                    let frame = indexed(data_plan()?)?
                        .collect()
                        .map_err(|error| CoreError::Import(error.to_string()))?;
                    let total_rows = frame.height();
                    Ok(SortedPageCacheEntry { frame, total_rows })
                },
            )?;
            let limit = page_limit(offset, limit, entry.total_rows);
            // Ordering a page meant ordering every row, so the rules run
            // over the frame that was materialized anyway and the page takes
            // its own slice of the answers. Deliberately not cached with the
            // frame: the cache key is lineage, and a rule is not lineage --
            // caching styles under it would repaint edited rules with the
            // colors they had before.
            style_matches = Some(
                frame
                    .evaluate_style_rules(self, &entry.frame)
                    .slice(offset, limit),
            );
            (entry.frame.slice(offset as i64, limit), entry.total_rows)
        } else if paged {
            let plan = plan()?;
            let total_rows = frame
                .artifact
                .as_ref()
                .filter(|_| !filtered)
                .map(|artifact| Ok(artifact.row_count))
                .unwrap_or_else(|| lazy_row_count(plan.clone()))?;
            let limit = page_limit(offset, limit, total_rows);
            // The one route where the rows in hand are not all the rows.
            // The hidden columns go in above the slice so an aggregate sees
            // the whole column; the elementwise ones still let the slice
            // push down into the scan. A rule that will not run must never
            // cost somebody their data, so a plan that fails with the rule
            // columns is retried without them and the page arrives unpainted.
            let sliced = |plan: pl::LazyFrame| {
                plan.slice(offset as i64, limit as u32)
                    .collect()
                    .map_err(|error| CoreError::Import(error.to_string()))
            };
            let data_frame = match rule_columns.is_empty() {
                true => sliced(plan)?,
                false => match sliced(plan.clone().with_columns(rule_columns.clone())) {
                    Ok(painted) => {
                        style_matches = Some(frame.read_style_rules(&painted));
                        painted
                    }
                    Err(_) => sliced(plan)?,
                },
            };
            (data_frame, total_rows)
        } else {
            let data_frame = indexed(data_plan()?)?
                .collect()
                .map_err(|error| CoreError::Import(error.to_string()))?;
            let total_rows = data_frame.height();
            let limit = page_limit(offset, limit, total_rows);
            // Small enough to have been collected whole, so the rules see
            // every row here too, and the page slices the answers.
            style_matches = Some(
                frame
                    .evaluate_style_rules(self, &data_frame)
                    .slice(offset, limit),
            );
            (data_frame.slice(offset as i64, limit), total_rows)
        };

        let row_ids = self.page_row_ids(frame, &data_frame, offset);
        let mut series_by_column = Vec::new();
        for column in &frame.columns {
            if let Ok(series) = data_frame
                .column(&column.id)
                .or_else(|_| data_frame.column(&column.name))
            {
                series_by_column.push(series.as_materialized_series().clone());
            }
        }

        let mut rows = Vec::with_capacity(data_frame.height());
        for row_index in 0..data_frame.height() {
            let mut row = Vec::with_capacity(series_by_column.len());
            for series in &series_by_column {
                let value = polars_value_at(series, row_index).map_err(CoreError::Import)?;
                row.push(scalar_value_to_raw(value));
            }
            rows.push(row);
        }

        Ok(FramePage {
            frame_id: frame_id.to_string(),
            total_rows,
            offset,
            limit: data_frame.height(),
            columns: frame.columns.clone(),
            row_ids,
            rows,
            style_matches: style_matches
                .map(|matches| matches.rows)
                .unwrap_or_default(),
        })
    }

    /// The identity of each row on the page, in page order.
    ///
    /// Read off the row index the plan carried when there is one; otherwise
    /// synthesized from the page position, which is all a row streamed
    /// straight out of a parquet scan ever had.
    fn page_row_ids(
        &self,
        frame: &FrameObject,
        data_frame: &pl::DataFrame,
        offset: usize,
    ) -> Vec<Id> {
        let indices = data_frame.column(ROW_INDEX).ok().and_then(|column| {
            column
                .as_materialized_series()
                .u32()
                .ok()
                .map(|values| values.into_no_null_iter().map(|v| v as usize).collect())
        });
        let indices: Vec<usize> =
            indices.unwrap_or_else(|| (offset..offset + data_frame.height()).collect());
        indices
            .into_iter()
            .map(|index| match frame.rows.get(index) {
                // A frame that still holds its own rows keeps their ids, so
                // an edit to a filtered, sorted page lands on the right one.
                Some(row)
                    if frame.steps.iter().all(|step| {
                        matches!(step, FrameStep::Filter { .. } | FrameStep::Sort { .. })
                    }) =>
                {
                    row.id.clone()
                }
                _ if frame.derivation.is_some() || frame.generator.is_some() => {
                    format!("derived:{}:{index}", frame.id)
                }
                _ => format!("source:{}:{index}", frame.id),
            })
            .collect()
    }

    pub(crate) fn parse_formula_for_frame(
        &self,
        frame_id: &str,
        source: &str,
    ) -> Result<Expr, CoreError> {
        let frame = self.frame(frame_id)?;
        Parser::new(source, frame, self)?.parse()
    }

    /// Parse a frame formula at the operation-preparation boundary, where a
    /// draft is about to become document state and its frame lineage exists.
    pub(crate) fn prepare_formula_for_frame(
        &self,
        frame_id: &str,
        source: &str,
    ) -> Result<Expr, CoreError> {
        let expression = self.parse_formula_for_frame(frame_id, source)?;
        self.validate_formula_ordering(frame_id, &expression)?;
        Ok(expression)
    }

    /// Refuse position-dependent formulas until the frame declares what row
    /// position means. This lives on formula write preparation rather than in
    /// the compiler: scalar formulas and completion probes compile without a
    /// frame, while persisted frame formulas have enough lineage context to
    /// answer the question.
    pub(crate) fn validate_formula_ordering(
        &self,
        frame_id: &str,
        expression: &Expr,
    ) -> Result<(), CoreError> {
        self.validate_expression_ordering(expression, self.plan_sorts(frame_id))
    }

    pub(crate) fn validate_expression_ordering(
        &self,
        expression: &Expr,
        ordering_is_declared: bool,
    ) -> Result<(), CoreError> {
        if expression.uses_row_shift() && !ordering_is_declared {
            return Err(CoreError::Formula(
                "Shift requires declared row ordering. Sort the frame or bind a sort column before saving this formula."
                    .into(),
            ));
        }
        if expression.uses_recurrence() && !ordering_is_declared {
            return Err(CoreError::Formula(
                "Calculate down rows requires declared row ordering. Sort the frame before saving this recurrence."
                    .into(),
            ));
        }
        if expression.uses_running_calculation() && !ordering_is_declared {
            return Err(CoreError::Formula(
                "A running calculation requires declared row ordering. Sort the frame before saving this formula."
                    .into(),
            ));
        }
        if expression.uses_frame_sequence() && !ordering_is_declared {
            return Err(CoreError::Formula(
                "A frame sequence requires declared row ordering. Sort the frame before filling the column."
                    .into(),
            ));
        }
        Ok(())
    }

    /// Parses a formula that sits in no frame — a result's.
    ///
    /// The scope is an empty frame, which is exactly the point: there are no
    /// columns for a bare name to land on, so everything a result names is a
    /// canvas object or a `` `Frame`.`Column` `` reference, resolved the same
    /// way a frame formula resolves them.
    pub(crate) fn parse_formula_scalar(&self, source: &str) -> Result<Expr, CoreError> {
        Parser::new_scalar(source, &FrameObject::default(), self)?.parse()
    }

    /// The scalar scope with a list-shaped answer welcome: what a generated
    /// frame's rule is parsed with, since a rule exists to make many rows.
    pub(crate) fn parse_formula_rule(&self, source: &str) -> Result<Expr, CoreError> {
        Parser::new_scalar_list(source, &FrameObject::default(), self)?.parse()
    }

    /// The formula as a person would retype it, for a formula outside any
    /// frame. Same empty scope as [`Document::parse_formula_scalar`].
    pub(crate) fn render_formula_scalar(&self, expression: &Expr) -> String {
        expression.render(&FrameObject::default(), self, 0)
    }

    /// Parses a formula on one of a block's lines: the scalar scope with
    /// the block's sibling lines resolvable bare in front of it.
    /// The same, against a block that is not in the document yet.
    ///
    /// Retyping a scratchpad rebuilds every line at once, and each line has
    /// to resolve its siblings' names as they will be *after* the edit — the
    /// line the author just renamed included.
    pub(crate) fn parse_formula_in_draft_block(
        &self,
        block: &BlockObject,
        source: &str,
    ) -> Result<Expr, CoreError> {
        Parser::new_in_block(source, block, &FrameObject::default(), self)?.parse()
    }
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

    /// Timing probe against real imported data, for answering "why does this
    /// feel slow?" with numbers instead of guesses. Ignored by default: it
    /// needs `demo-data/`, which is generated rather than committed. Run it
    /// with `cargo test -p framework-core --release perf_probe -- --ignored
    /// --nocapture`.
    #[test]
    #[ignore]
    pub(crate) fn perf_probe_import_view_and_page_costs() {
        use std::time::Instant;

        let source =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../demo-data/general_ledger.csv");
        if !source.exists() {
            println!("skipping: {} is missing", source.display());
            return;
        }

        let mut store = Store::new(Document {
            id: id(),
            name: "Perf".into(),
            revision: 0,
            objects: Vec::new(),
            views: Vec::new(),
            frozen_values: Default::default(),
        });
        // The desktop app imports through ImportFrameFromArtifact (it stages
        // a parquet artifact first), so measure that path, not the
        // artifact-less ImportFrameFromFile.
        let staging = temporary_test_directory("perf-probe-artifacts");
        let started = Instant::now();
        let artifact = create_data_artifact(&source, &staging).unwrap();
        println!("stage artifact: {:?}", started.elapsed());
        let started = Instant::now();
        store
            .apply(Operation::ImportFrameFromArtifact {
                name: "Ledger".into(),
                artifact,
                connector: None,
                x: 0.0,
                y: 0.0,
            })
            .unwrap();
        println!("import: {:?}", started.elapsed());

        let frame = frame_named(&store.document, "Ledger").clone();
        println!(
            "rows: {:?}, columns: {}, artifact: {:?}, source_file: {:?}",
            store.view().computed_frames[&frame.id].total_rows,
            frame.columns.len(),
            frame
                .artifact
                .as_ref()
                .map(|artifact| (artifact.row_count, artifact.path.clone())),
            frame.source_file.is_some(),
        );

        // `view()` runs on every operation and every get_document, so its
        // cost is paid on every interaction, not just on load.
        for round in 0..3 {
            let started = Instant::now();
            let view = store.view();
            println!(
                "view() #{round}: {:?} ({} objects)",
                started.elapsed(),
                view.document.objects.len()
            );
        }

        // Break view() down so the cost lands on a specific stage.
        let started = Instant::now();
        let materialized = store.document.materialized_for_view();
        println!("  materialized_for_view: {:?}", started.elapsed());
        let started = Instant::now();
        let computed = materialized.compute_frames();
        println!(
            "  compute_frames: {:?} ({} frames)",
            started.elapsed(),
            computed.len()
        );
        let started = Instant::now();
        let catalog = formula_function_catalog();
        println!(
            "  formula_function_catalog: {:?} ({} functions)",
            started.elapsed(),
            catalog.len()
        );

        let started = Instant::now();
        store.get_frame_page(&frame.id, 0, 1000).unwrap();
        println!("unsorted page 0: {:?}", started.elapsed());

        let started = Instant::now();
        store.get_frame_page(&frame.id, 500_000, 1000).unwrap();
        println!("unsorted page @500k: {:?}", started.elapsed());

        store
            .apply(Operation::SetFrameDisplaySort {
                frame_id: frame.id.clone(),
                keys: vec![DerivedSort {
                    column_id: frame.columns[0].id.clone(),
                    descending: true,
                }],
            })
            .unwrap();
        let started = Instant::now();
        store.get_frame_page(&frame.id, 0, 1000).unwrap();
        println!("first sorted page (fills cache): {:?}", started.elapsed());
        let started = Instant::now();
        store.get_frame_page(&frame.id, 1000, 1000).unwrap();
        println!("second sorted page (cached): {:?}", started.elapsed());

        // A grouped result derived from the import has no artifact of its
        // own, so its row count cannot be read off one -- which is the
        // shape a real accounting document has on the canvas.
        store
            .apply(Operation::AddDerivedFrame {
                source_frame_id: frame.id.clone(),
                name: "By period".into(),
                group_keys: vec![NamedFormulaInput {
                    name: frame.columns[0].name.clone(),
                    formula: format!("`{}`", frame.columns[0].name),
                }],
                aggregates: vec![NamedFormulaInput {
                    name: "Rows".into(),
                    formula: format!("`{}`.count()", frame.columns[0].name),
                }],
                maintain_order: true,
                x: 600.0,
                y: 0.0,
            })
            .unwrap();
        for round in 0..3 {
            let started = Instant::now();
            store.view();
            println!(
                "view() with a grouped frame #{round}: {:?}",
                started.elapsed()
            );
        }
        let started = Instant::now();
        let materialized = store.document.materialized_for_view();
        println!("  materialized_for_view: {:?}", started.elapsed());
        let started = Instant::now();
        materialized.compute_frames();
        println!("  compute_frames: {:?}", started.elapsed());

        // Caching the grouped result: reads should stop re-aggregating.
        let grouped_id = frame_named(&store.document, "By period").id.clone();
        let started = Instant::now();
        store.get_frame_page(&grouped_id, 0, 100).unwrap();
        println!("live grouped page: {:?}", started.elapsed());

        let started = Instant::now();
        store.materialize_frame(&grouped_id, &staging).unwrap();
        println!("materialize: {:?}", started.elapsed());

        let started = Instant::now();
        store.get_frame_page(&grouped_id, 0, 100).unwrap();
        println!("cached grouped page: {:?}", started.elapsed());
        let started = Instant::now();
        store.view();
        println!(
            "view() with a cached grouped frame: {:?}",
            started.elapsed()
        );
        println!(
            "  total_rows now: {:?}",
            store.view().computed_frames[&grouped_id].total_rows
        );

        fs::remove_dir_all(staging).unwrap();
    }
}
