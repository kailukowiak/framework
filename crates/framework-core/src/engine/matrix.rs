use crate::*;
use std::collections::HashMap;

impl Document {
    pub(crate) fn compute_calculation_matrices(&self) -> HashMap<Id, ComputedCalculationMatrix> {
        self.objects
            .iter()
            .filter_map(|object| match object {
                DataObject::CalculationMatrix(matrix) => {
                    Some((matrix.id.clone(), self.compute_calculation_matrix(matrix)))
                }
                _ => None,
            })
            .collect()
    }

    pub(crate) fn compute_calculation_matrix(
        &self,
        matrix: &CalculationMatrixObject,
    ) -> ComputedCalculationMatrix {
        let fail = |error| ComputedCalculationMatrix {
            row_tuples: Vec::new(),
            column_tuples: Vec::new(),
            cells: Vec::new(),
            output: None,
            error: Some(error),
        };
        if let Some(error) = matrix
            .rows
            .iter()
            .chain(&matrix.columns)
            .find_map(|item| item.error.clone())
        {
            return fail(error);
        }
        if matrix.body.formula.is_none() {
            return ComputedCalculationMatrix {
                row_tuples: Vec::new(),
                column_tuples: Vec::new(),
                cells: Vec::new(),
                output: None,
                error: matrix.body.error.clone(),
            };
        }
        let rows = match self.evaluate_matrix_axis(&matrix.rows) {
            Ok(rows) => rows,
            Err(error) => return fail(error),
        };
        let columns = match self.evaluate_matrix_axis(&matrix.columns) {
            Ok(columns) => columns,
            Err(error) => return fail(error),
        };
        if matrix
            .rows
            .iter()
            .chain(&matrix.columns)
            .any(|axis| axis.target_id.is_some())
        {
            return self
                .compute_matrix_sensitivity(matrix, rows, columns)
                .unwrap_or_else(fail);
        }
        self.finish_calculation_matrix(matrix, rows, columns)
            .unwrap_or_else(fail)
    }

    fn finish_calculation_matrix(
        &self,
        matrix: &CalculationMatrixObject,
        rows: Vec<Vec<String>>,
        columns: Vec<Vec<String>>,
    ) -> Result<ComputedCalculationMatrix, String> {
        let scope = crate::operation::prepare::calculation_matrix::matrix_scope(
            &matrix.id,
            &matrix.rows,
            &matrix.columns,
        );
        let mut source_rows = Vec::new();
        for (row_index, row) in rows.iter().enumerate() {
            for (column_index, column) in columns.iter().enumerate() {
                let cells = matrix
                    .rows
                    .iter()
                    .chain(&matrix.columns)
                    .zip(row.iter().chain(column))
                    .map(|(item, raw)| {
                        (
                            item.id.clone(),
                            Cell {
                                raw: raw.clone(),
                                override_formula: None,
                            },
                        )
                    })
                    .collect();
                source_rows.push(Row {
                    id: format!("{}:{row_index}:{column_index}", matrix.id),
                    cells,
                });
            }
        }
        let evaluation_frame = FrameObject {
            rows: source_rows.clone(),
            ..scope.clone()
        };
        let frame = evaluation_frame.materialize_polars_frame(self)?;
        let body = matrix
            .body
            .formula
            .as_ref()
            .expect("checked before matrix evaluation");
        let series = evaluation_frame.evaluate_polars_series(self, &frame, &body.expression)?;
        let data_type =
            crate::framework_type_from_polars(series.dtype()).unwrap_or(DataType::String);
        let mut cell_rows = Vec::new();
        let mut output_rows = Vec::new();
        let value_id = format!("{}:value", matrix.id);
        for (row_index, _) in rows.iter().enumerate() {
            let mut cell_row = Vec::new();
            for column_index in 0..columns.len() {
                let index = row_index * columns.len() + column_index;
                let value = crate::polars_value_at(&series, index);
                let raw = value
                    .clone()
                    .map(crate::engine::values::scalar_value_to_raw)
                    .unwrap_or_default();
                cell_row.push(computed_cell(value, data_type, false));
                let mut output = source_rows[index].clone();
                output.cells.insert(
                    value_id.clone(),
                    Cell {
                        raw,
                        override_formula: None,
                    },
                );
                output_rows.push(output);
            }
            cell_rows.push(cell_row);
        }
        let mut output_columns = scope.columns;
        output_columns.push(Column {
            id: value_id,
            name: "Value".into(),
            source_name: None,
            data_type,
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
            cells: cell_rows,
            output: Some(CalculationMatrixOutput {
                columns: output_columns,
                rows: output_rows,
            }),
            error: None,
        })
    }

    fn evaluate_matrix_axis(
        &self,
        axis: &[CalculationMatrixAxisFormula],
    ) -> Result<Vec<Vec<String>>, String> {
        if axis.is_empty() {
            return Ok(vec![Vec::new()]);
        }
        let series = axis
            .iter()
            .map(|item| {
                let formula = item.formula.as_ref().ok_or_else(|| {
                    item.error
                        .clone()
                        .unwrap_or_else(|| "Invalid axis formula".into())
                })?;
                let (_, values) = self.evaluate_to_series(&formula.expression)?;
                if values.is_empty() {
                    return Err(format!("{} produced no values", item.name));
                }
                (0..values.len())
                    .map(|index| {
                        crate::polars_value_at(&values, index)
                            .map(crate::engine::values::scalar_value_to_raw)
                    })
                    .collect()
            })
            .collect::<Result<Vec<Vec<String>>, String>>()?;
        let maximum = series.iter().map(Vec::len).max().unwrap_or(1);
        if let Some((index, values)) = series
            .iter()
            .enumerate()
            .find(|(_, values)| maximum % values.len() != 0)
        {
            return Err(format!(
                "{} has {} values, which does not evenly divide the axis length {maximum}",
                axis[index].name,
                values.len()
            ));
        }
        Ok((0..maximum)
            .map(|index| {
                series
                    .iter()
                    .map(|values| values[index % values.len()].clone())
                    .collect()
            })
            .collect())
    }
}
