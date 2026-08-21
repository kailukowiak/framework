//! On-demand frame footer aggregates.
//!
//! A footer is display state, but its answers may require a pass over a
//! million-row source. It therefore cannot be part of `Store::view()`, which
//! is rebuilt after every accepted edit anywhere on the canvas. The frame
//! asks for this projection only while it has summary rows to draw, and all
//! requested cells share one Polars `select` so the source is scanned once.

use crate::*;
use polars::prelude as pl;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameSummary {
    pub frame_id: Id,
    pub rows: Vec<FrameSummaryRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameSummaryRow {
    pub operation: SummaryOperation,
    pub label: String,
    /// Unsupported columns are deliberately absent. The interface renders
    /// that absence as `n/a`, which distinguishes it from a supported
    /// aggregate whose answer is null.
    pub cells: BTreeMap<Id, ComputedCell>,
}

struct SummaryCellRequest {
    row_index: usize,
    column_id: Id,
    alias: String,
    data_type: DataType,
}

pub(crate) fn literal_summary_cells(
    frame: &FrameObject,
    rows: &HashMap<Id, HashMap<Id, ComputedCell>>,
) -> HashMap<Id, ComputedCell> {
    frame
        .summaries
        .iter()
        .map(|summary| {
            let cells = frame
                .rows
                .iter()
                .filter_map(|row| rows.get(&row.id)?.get(&summary.column_id))
                .filter(|cell| cell.error.is_none())
                .collect::<Vec<_>>();
            let missing = cells
                .iter()
                .filter(|cell| matches!(cell.typed_value, ScalarValue::Null))
                .count();
            let values = cells
                .into_iter()
                .map(|cell| cell.typed_value.clone())
                .filter(|value| !matches!(value, ScalarValue::Null))
                .collect::<Vec<_>>();
            let data_type = frame
                .columns
                .iter()
                .find(|column| column.id == summary.column_id)
                .map(|column| column.data_type)
                .unwrap_or(DataType::Number);
            (
                summary.id.clone(),
                computed_cell(
                    literal_aggregate(summary.operation, data_type, &values, missing),
                    summary.operation.output_type(data_type),
                    false,
                ),
            )
        })
        .collect()
}

impl Store {
    pub fn get_frame_summary(&self, frame_id: &str) -> Result<FrameSummary, CoreError> {
        self.document.frame_summary(frame_id)
    }
}

impl Document {
    fn frame_summary(&self, frame_id: &str) -> Result<FrameSummary, CoreError> {
        let frame = self.frame(frame_id)?;
        let operations = displayed_summary_operations(frame);
        let explicit_rows = frame.display.summary_rows.is_some();
        let mut rows = operations
            .iter()
            .map(|operation| FrameSummaryRow {
                operation: *operation,
                label: operation.label().into(),
                cells: BTreeMap::new(),
            })
            .collect::<Vec<_>>();
        let mut requests = Vec::new();
        let mut expressions = Vec::new();

        for (row_index, operation) in operations.iter().copied().enumerate() {
            for column in &frame.columns {
                let requested = explicit_rows
                    || frame.summaries.iter().any(|summary| {
                        summary.operation == operation && summary.column_id == column.id
                    });
                if !requested || !operation.supports(column.data_type) {
                    continue;
                }
                let alias = format!("__framework_summary_{}", requests.len());
                expressions.push(summary_expression(operation, &column.id).alias(alias.clone()));
                requests.push(SummaryCellRequest {
                    row_index,
                    column_id: column.id.clone(),
                    alias,
                    data_type: operation.output_type(column.data_type),
                });
            }
        }

        if expressions.is_empty() {
            return Ok(FrameSummary {
                frame_id: frame_id.into(),
                rows,
            });
        }
        let frame = self
            .materialize_frame_lazy(frame_id, Layer::Display, &mut Default::default())
            .map_err(CoreError::Import)?
            .select(expressions)
            .collect()
            .map_err(|error| CoreError::Import(format!("frame summary: {error}")))?;
        for request in requests {
            let result = frame
                .column(&request.alias)
                .map_err(|error| error.to_string())
                .and_then(|column| polars_value_at(column.as_materialized_series(), 0));
            rows[request.row_index].cells.insert(
                request.column_id,
                computed_cell(result, request.data_type, false),
            );
        }
        Ok(FrameSummary {
            frame_id: frame_id.into(),
            rows,
        })
    }
}

fn displayed_summary_operations(frame: &FrameObject) -> Vec<SummaryOperation> {
    if let Some(summary_rows) = &frame.display.summary_rows {
        return summary_rows.clone();
    }
    let mut operations = Vec::new();
    for summary in &frame.summaries {
        if !operations.contains(&summary.operation) {
            operations.push(summary.operation);
        }
    }
    operations
}

fn summary_expression(operation: SummaryOperation, column_id: &str) -> pl::Expr {
    let column = pl::col(column_id.to_string());
    match operation {
        SummaryOperation::Sum => column.sum(),
        SummaryOperation::Mean => column.mean(),
        SummaryOperation::Quartile25 => column.quantile(pl::lit(0.25), pl::QuantileMethod::Linear),
        SummaryOperation::Median => column.median(),
        SummaryOperation::Quartile75 => column.quantile(pl::lit(0.75), pl::QuantileMethod::Linear),
        SummaryOperation::Min => column.min(),
        SummaryOperation::Max => column.max(),
        SummaryOperation::Count => column.count().cast(pl::DataType::Int64),
        SummaryOperation::Missing => column.null_count().cast(pl::DataType::Int64),
        SummaryOperation::CountDistinct => column.drop_nulls().n_unique().cast(pl::DataType::Int64),
        // A mode can contain several tied values. A footer cell has room for
        // one, so stable source order breaks the tie rather than expanding
        // the row or inventing a comma-delimited value of a different type.
        SummaryOperation::Mode => column.drop_nulls().mode(true).first(),
    }
}

fn literal_aggregate(
    operation: SummaryOperation,
    data_type: DataType,
    values: &[ScalarValue],
    missing: usize,
) -> Result<ScalarValue, String> {
    if !operation.supports(data_type) {
        return Err(format!(
            "{} does not apply to this column type",
            operation.label()
        ));
    }
    match operation {
        SummaryOperation::Sum => Ok(ScalarValue::Number(numeric(values)?.iter().sum())),
        SummaryOperation::Mean => mean(values),
        SummaryOperation::Quartile25 => quantile(values, 0.25),
        SummaryOperation::Median => median(values),
        SummaryOperation::Quartile75 => quantile(values, 0.75),
        SummaryOperation::Min => extremum(values, Ordering::Less),
        SummaryOperation::Max => extremum(values, Ordering::Greater),
        SummaryOperation::Count => Ok(ScalarValue::Number(values.len() as f64)),
        SummaryOperation::Missing => Ok(ScalarValue::Number(missing as f64)),
        SummaryOperation::CountDistinct => {
            let distinct = values
                .iter()
                .map(scalar_key)
                .collect::<std::collections::BTreeSet<_>>();
            Ok(ScalarValue::Number(distinct.len() as f64))
        }
        SummaryOperation::Mode => mode(values),
    }
}

fn numeric(values: &[ScalarValue]) -> Result<Vec<f64>, String> {
    values
        .iter()
        .map(|value| match value {
            ScalarValue::Number(number) => Ok(*number),
            _ => Err("Summary requires numeric values".into()),
        })
        .collect()
}

fn mean(values: &[ScalarValue]) -> Result<ScalarValue, String> {
    let values = numeric(values)?;
    Ok(if values.is_empty() {
        ScalarValue::Null
    } else {
        ScalarValue::Number(values.iter().sum::<f64>() / values.len() as f64)
    })
}

fn median(values: &[ScalarValue]) -> Result<ScalarValue, String> {
    quantile(values, 0.5)
}

fn quantile(values: &[ScalarValue], probability: f64) -> Result<ScalarValue, String> {
    let mut values = numeric(values)?;
    if values.is_empty() {
        return Ok(ScalarValue::Null);
    }
    values.sort_by(f64::total_cmp);
    let position = (values.len() - 1) as f64 * probability;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let value = values[lower] + (values[upper] - values[lower]) * position.fract();
    Ok(ScalarValue::Number(value))
}

fn extremum(values: &[ScalarValue], wanted: Ordering) -> Result<ScalarValue, String> {
    let Some(first) = values.first() else {
        return Ok(ScalarValue::Null);
    };
    let mut answer = first;
    for value in &values[1..] {
        if scalar_order(value, answer)? == wanted {
            answer = value;
        }
    }
    Ok(answer.clone())
}

fn scalar_order(left: &ScalarValue, right: &ScalarValue) -> Result<Ordering, String> {
    match (left, right) {
        (ScalarValue::Number(left), ScalarValue::Number(right)) => Ok(left.total_cmp(right)),
        (ScalarValue::Date(left), ScalarValue::Date(right)) => Ok(left.cmp(right)),
        _ => Err("Summary values do not share a comparable type".into()),
    }
}

fn mode(values: &[ScalarValue]) -> Result<ScalarValue, String> {
    let Some(first) = values.first() else {
        return Ok(ScalarValue::Null);
    };
    let mut counts = HashMap::<String, usize>::new();
    for value in values {
        *counts.entry(scalar_key(value)).or_default() += 1;
    }
    let mut best = first;
    let mut best_count = 0;
    for value in values {
        let count = counts.get(&scalar_key(value)).copied().unwrap_or_default();
        if count > best_count {
            best = value;
            best_count = count;
        }
    }
    Ok(best.clone())
}

fn scalar_key(value: &ScalarValue) -> String {
    match value {
        ScalarValue::Null => "null".into(),
        ScalarValue::Number(value) => format!("number:{:016x}", value.to_bits()),
        ScalarValue::String(value) => format!("string:{value}"),
        ScalarValue::Boolean(value) => format!("boolean:{value}"),
        ScalarValue::Date(value) => format!("date:{value}"),
    }
}
