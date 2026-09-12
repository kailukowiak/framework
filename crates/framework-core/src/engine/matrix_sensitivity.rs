//! Sensitivity is a matrix evaluation mode, not a sequence of edits to the
//! user's workbook. A private document per cell preserves the ordinary
//! dependency compiler (including live frame plans) without emitting history,
//! collaboration events, or temporary values to another reader.
use crate::*;
use std::collections::HashSet;

impl Document {
    pub(crate) fn compute_matrix_sensitivity(
        &self,
        matrix: &CalculationMatrixObject,
        rows: Vec<Vec<String>>,
        columns: Vec<Vec<String>>,
    ) -> Result<ComputedCalculationMatrix, String> {
        // Each point may execute a whole model. Bound work before cloning or
        // constructing the cross product; ordinary vectorized matrices keep
        // their existing behavior.
        if rows.len().saturating_mul(columns.len()) > 2500 {
            return Err("Sensitivity matrices support at most 2,500 cells".into());
        }
        let mut targets = HashSet::new();
        for axis in matrix.rows.iter().chain(&matrix.columns) {
            if let Some(target) = &axis.target_id {
                if !targets.insert(target) {
                    return Err("A sensitivity input can be varied by only one axis field".into());
                }
                self.matrix_input_type(target)?;
            }
        }
        let scope = crate::operation::prepare::calculation_matrix::matrix_scope(
            &matrix.id,
            &matrix.rows,
            &matrix.columns,
        );
        let mut cells = Vec::new();
        let mut output_rows = Vec::new();
        let value_id = format!("{}:value", matrix.id);
        let mut output_type = None;
        for (row_index, row) in rows.iter().enumerate() {
            let mut cell_row = Vec::new();
            for (column_index, column) in columns.iter().enumerate() {
                let mut output = sensitivity_row(matrix, row, column, row_index, column_index);
                let evaluated = self.evaluate_sensitivity_point(matrix, &scope, &output);
                let cell = match evaluated {
                    Ok((_, ScalarValue::Null)) => computed_cell(Ok(ScalarValue::Null), output_type.unwrap_or(DataType::String), false),
                    Ok((data_type, value)) => {
                        // A live filter can change the inferred integer/float
                        // representation without changing the scalar's type.
                        let data_type = if matches!(value, ScalarValue::Number(_)) { DataType::Number } else { data_type };
                        let expected = *output_type.get_or_insert(data_type);
                        if expected != data_type {
                            computed_cell(
                                Err("Sensitivity output type changed between cells".into()),
                                expected,
                                false,
                            )
                        } else {
                            computed_cell(Ok(value), data_type, false)
                        }
                    }
                    Err(error) => {
                        computed_cell(Err(error), output_type.unwrap_or(DataType::String), false)
                    }
                };
                output.cells.insert(
                    value_id.clone(),
                    Cell {
                        raw: scalar_value_to_raw(cell.typed_value.clone()),
                        override_formula: None,
                    },
                );
                output_rows.push(output);
                cell_row.push(cell);
            }
            cells.push(cell_row);
        }
        let mut output_columns = scope.columns;
        output_columns.push(Column {
            id: value_id,
            name: "Value".into(),
            source_name: None,
            data_type: output_type.unwrap_or(DataType::String),
            categories: Vec::new(),
            format: None,
            formula: None,
        });
        Ok(ComputedCalculationMatrix {
            row_tuples: rows
                .into_iter()
                .map(|values| CalculationMatrixTuple { values })
                .collect(),
            column_tuples: columns
                .into_iter()
                .map(|values| CalculationMatrixTuple { values })
                .collect(),
            cells,
            output: Some(CalculationMatrixOutput {
                columns: output_columns,
                rows: output_rows,
            }),
            error: None,
        })
    }

    fn evaluate_sensitivity_point(
        &self,
        matrix: &CalculationMatrixObject,
        scope: &FrameObject,
        row: &Row,
    ) -> Result<(DataType, ScalarValue), String> {
        let mut evaluation = self.clone();
        for axis in matrix.rows.iter().chain(&matrix.columns) {
            let Some(target) = &axis.target_id else {
                continue;
            };
            let data_type = self.matrix_input_type(target)?;
            let value = trial_value(&row.cells[&axis.id].raw, axis.data_type)?;
            // Parse with the axis type first: a string "10" must not silently
            // become the numeric assumption 10 just because raw text matches.
            let raw = scalar_value_to_raw(value.clone());
            let typed = trial_value(&raw, data_type)?;
            if typed != value {
                return Err("Sensitivity values must match the input's type".into());
            }
            if matches!(evaluation.object(target), Ok(DataObject::Value(_))) {
                let DataObject::Value(input) =
                    evaluation.object_mut(target).map_err(|e| e.to_string())?
                else {
                    unreachable!()
                };
                input.raw = raw;
                for scenario in &mut evaluation.scenarios {
                    scenario.values.remove(target);
                }
            } else {
                let operation = evaluation
                    .prepare_set_parameter_value(target.clone(), value)
                    .map_err(|e| e.to_string())?;
                evaluation
                    .apply_replicated(operation)
                    .map_err(|e| e.to_string())?;
            }
        }
        let body = &matrix
            .body
            .formula
            .as_ref()
            .ok_or("Invalid matrix formula")?
            .expression;
        super::matrix_dependencies::prepare(&mut evaluation, body, &mut HashSet::new())?;
        let frame = FrameObject {
            rows: vec![row.clone()],
            ..scope.clone()
        };
        let data = frame.materialize_polars_frame(&evaluation)?;
        let series = frame.evaluate_polars_series(&evaluation, &data, body)?;
        if series.len() != 1 {
            return Err("A sensitivity cell must produce one value".into());
        }
        Ok((
            framework_type_from_polars(series.dtype()).unwrap_or(DataType::String),
            polars_value_at(&series, 0)?,
        ))
    }

    pub(crate) fn matrix_input_type(&self, target: &str) -> Result<DataType, String> {
        if self.frozen_values.contains_key(target) {
            return Err("A frozen value cannot be a sensitivity input".into());
        }
        if let Ok(DataObject::Value(value)) = self.object(target) {
            return Ok(value.data_type);
        }
        let input = self
            .parameter_inputs()
            .into_iter()
            .find(|input| input.id == target)
            .ok_or("Choose a value or a named control variable as the sensitivity input")?;
        Ok(match input.control {
            ParameterControl::Slider { .. } => DataType::Number,
            ParameterControl::DateInput { .. } => DataType::Date,
            ParameterControl::Dropdown { value, .. } => match value {
                ScalarValue::Number(_) => DataType::Number,
                ScalarValue::Boolean(_) => DataType::Boolean,
                ScalarValue::Date(_) => DataType::Date,
                _ => DataType::String,
            },
        })
    }
}

fn trial_value(raw: &str, data_type: DataType) -> Result<ScalarValue, String> {
    if matches!(data_type, DataType::String | DataType::Categorical) {
        Ok(ScalarValue::String(raw.into()))
    } else {
        parse_scalar_value(raw, data_type)
    }
}

fn sensitivity_row(matrix: &CalculationMatrixObject, row: &[String], column: &[String], row_index: usize, column_index: usize) -> Row {
    Row {
        id: format!("{}:{row_index}:{column_index}", matrix.id),
        cells: matrix.rows.iter().chain(&matrix.columns).zip(row.iter().chain(column))
            .map(|(axis, raw)| (axis.id.clone(), Cell { raw: raw.clone(), override_formula: None })).collect(),
    }
}
