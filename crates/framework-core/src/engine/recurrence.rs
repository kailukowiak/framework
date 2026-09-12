//! Sequential calculated columns.
//!
//! Polars is normally asked for whole-column expressions, which is exactly
//! right until a row needs the answer calculated immediately above it. A
//! recurrence deliberately crosses that boundary: collect the ordered input,
//! evaluate one row at a time, and put the finished column back into the lazy
//! chain. It is slower and honest. Pretending `previous()` is vectorized would
//! be faster only by changing what it means.

use crate::formula::ast::{PREVIOUS_RESULT_COLUMN_ID, RecurrenceParts};
use crate::*;
use polars::prelude as pl;
use polars::prelude::IntoLazy;
use std::collections::HashMap;
use std::fmt::Write;

impl Document {
    pub(crate) fn apply_with_columns_step(
        &self,
        frame_id: &str,
        mut plan: pl::LazyFrame,
        columns: &[DerivedExpression],
    ) -> Result<pl::LazyFrame, String> {
        if columns.len() > 1
            && columns
                .iter()
                .any(|column| column.expression.uses_recurrence())
        {
            return Err(
                "Calculate down rows must be its own Wrangle step so its input is unambiguous"
                    .into(),
            );
        }
        // A prior-period read joins the frame to itself on the declared
        // period column, so it is lifted out before anything compiles —
        // the same shape as a mapping call below, and first so a lookup
        // around a prior (or the reverse) resolves in the second pass.
        let (joined, columns, mut answers) =
            crate::formula::financial_period::join_prior_periods(self, frame_id, plan, columns)?;
        plan = joined;
        // A mapping call reads another frame, so it joins that frame into
        // the plan here rather than compiling to a literal; the columns
        // below are the same step with each call pointing at its answer.
        let (joined, columns, mapping_answers) =
            crate::formula::dictionary::join_mappings(self, plan, &columns)?;
        plan = joined;
        answers.extend(mapping_answers);
        let mut ordinary = Vec::new();
        for column in &columns {
            match column.expression.recurrence_parts()? {
                Some(recurrence) => {
                    if !ordinary.is_empty() {
                        plan = plan.with_columns(std::mem::take(&mut ordinary));
                    }
                    plan = self.apply_recurrence(plan, &column.output_column_id, recurrence)?;
                }
                None => ordinary.push(
                    column
                        .expression
                        .to_polars(self)?
                        .alias(column.output_column_id.clone()),
                ),
            }
        }
        if !ordinary.is_empty() {
            plan = plan.with_columns(ordinary);
        }
        Ok(if answers.is_empty() {
            plan
        } else {
            plan.drop(pl::cols(answers))
        })
    }

    fn apply_recurrence(
        &self,
        plan: pl::LazyFrame,
        output_column_id: &str,
        recurrence: RecurrenceParts<'_>,
    ) -> Result<pl::LazyFrame, String> {
        let output_type = recurrence_type(self, &plan, &recurrence)?;
        let mut frame = plan.collect().map_err(|error| error.to_string())?;
        let mut histories: HashMap<String, pl::Series> = HashMap::new();
        let mut output: Option<pl::Series> = None;

        for row_index in 0..frame.height() {
            let key = partition_key(&frame, row_index, &recurrence.restart_by)?;
            let previous = histories.get(&key).cloned();
            let mut row = frame.slice(row_index as i64, 1);
            let expression = match previous {
                Some(mut previous) => {
                    previous.rename(PREVIOUS_RESULT_COLUMN_ID.into());
                    row.with_column(previous.into())
                        .map_err(|error| error.to_string())?;
                    recurrence.next
                }
                None => recurrence.seed,
            };
            let result = row
                .lazy()
                .select([expression
                    .to_polars(self)?
                    .cast(output_type.clone())
                    .alias(output_column_id.to_string())])
                .collect()
                .map_err(|error| error.to_string())?
                .column(output_column_id)
                .map_err(|error| error.to_string())?
                .as_materialized_series()
                .clone();
            if result.len() != 1 {
                return Err("Each recurrence row must calculate exactly one value".into());
            }
            histories.insert(key, result.clone());
            if let Some(values) = &mut output {
                values.append(&result).map_err(|error| error.to_string())?;
            } else {
                output = Some(result);
            }
        }

        let column = match output {
            Some(mut values) => {
                values.rename(output_column_id.to_string().into());
                values.into()
            }
            None => pl::Column::full_null(output_column_id.to_string().into(), 0, &output_type),
        };
        frame
            .with_column(column)
            .map_err(|error| error.to_string())?;
        Ok(frame.lazy())
    }
}

/// Resolve both branches before calculating even the first row. Casting every
/// answer to the seed's type used to discard cents at every step of an integer-
/// seeded loan. Feed the promoted type back into previous() until the schema
/// settles: the next expression can itself depend on the previous value's type.
fn recurrence_type(
    document: &Document,
    plan: &pl::LazyFrame,
    recurrence: &RecurrenceParts<'_>,
) -> Result<pl::DataType, String> {
    let seed = recurrence.seed.to_polars(document)?;
    let next = recurrence.next.to_polars(document)?;
    let mut probe = plan.clone().select([seed.clone().alias("answer")]);
    let mut dtype = probe
        .collect_schema()
        .map_err(|e| e.to_string())?
        .get("answer")
        .cloned()
        .ok_or("Could not determine the recurrence seed type")?;
    for _ in 0..8 {
        let mut probe = plan
            .clone()
            .with_column(
                pl::lit(pl::NULL)
                    .cast(dtype.clone())
                    .alias(PREVIOUS_RESULT_COLUMN_ID),
            )
            .select([pl::when(pl::lit(true))
                .then(seed.clone())
                .otherwise(next.clone())
                .alias("answer")]);
        let promoted = probe
            .collect_schema()
            .map_err(|e| e.to_string())?
            .get("answer")
            .cloned()
            .ok_or("Could not determine the recurrence step type")?;
        if promoted == dtype {
            return Ok(dtype);
        }
        dtype = promoted;
    }
    Err("Could not determine a stable recurrence result type; cast the seed and next value explicitly".into())
}

fn partition_key(
    frame: &pl::DataFrame,
    row_index: usize,
    restart_by: &[&Id],
) -> Result<String, String> {
    if restart_by.is_empty() {
        return Ok(String::new());
    }
    let mut key = String::new();
    for column_id in restart_by {
        let series = frame
            .column(column_id)
            .map_err(|error| error.to_string())?
            .as_materialized_series();
        let value = series.get(row_index).map_err(|error| error.to_string())?;
        let component = format!("{:?}:{value:?}", series.dtype());
        write!(&mut key, "{}:{component}", component.len()).map_err(|error| error.to_string())?;
    }
    Ok(key)
}
