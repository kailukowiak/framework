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
        let mut ordinary = Vec::new();
        for column in columns {
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
        Ok(if ordinary.is_empty() {
            plan
        } else {
            plan.with_columns(ordinary)
        })
    }

    fn apply_recurrence(
        &self,
        plan: pl::LazyFrame,
        output_column_id: &str,
        recurrence: RecurrenceParts<'_>,
    ) -> Result<pl::LazyFrame, String> {
        let output_type = plan
            .clone()
            .select([recurrence
                .seed
                .to_polars(self)?
                .alias(output_column_id.to_string())])
            .collect_schema()
            .map_err(|error| error.to_string())?
            .get(output_column_id)
            .cloned()
            .ok_or_else(|| "Could not determine the first-row value type".to_string())?;
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
