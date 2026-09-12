//! Bind typed pipeline slots to stable frame columns. A missing string is not
//! the empty string: substituting it would silently change category membership.
use crate::engine::ml::ml_error;
use crate::*;
use framework_ml::pipeline_types::{Dtype, Scalar};
use polars::prelude as pl;
use std::collections::HashSet;

impl Document {
    pub(crate) fn model_binding_columns(
        &self,
        source_id: &str,
        ids: &[Id],
        fitted: &framework_ml::FittedModel,
    ) -> Result<Vec<String>, CoreError> {
        let expected = framework_ml::input_types(fitted);
        if expected.len() != ids.len() || ids.is_empty() || ids.len() > 256 {
            return Err(ml_error(
                "Select one source column for each model input, in fitted order",
            ));
        }
        let frame = self.frame(source_id)?;
        let mut seen = HashSet::new();
        ids.iter()
            .zip(expected)
            .map(|(id, expected)| {
                if !seen.insert(id) {
                    return Err(ml_error("A model feature is selected twice"));
                }
                let column = frame
                    .columns
                    .iter()
                    .find(|c| &c.id == id)
                    .ok_or(CoreError::ColumnNotFound)?;
                let valid = match expected {
                    Dtype::Number => matches!(
                        column.data_type,
                        DataType::Integer
                            | DataType::Number
                            | DataType::Currency
                            | DataType::Percentage
                            | DataType::Boolean
                    ),
                    Dtype::String => {
                        matches!(column.data_type, DataType::String | DataType::Categorical)
                    }
                    Dtype::Boolean => column.data_type == DataType::Boolean,
                };
                if !valid {
                    return Err(ml_error(format!(
                        "‘{}’ does not match the model's {:?} input",
                        column.name, expected
                    )));
                }
                Ok(column.name.clone())
            })
            .collect()
    }
}

pub(crate) fn typed_model_rows(
    source: pl::LazyFrame,
    ids: &[Id],
    fitted: &framework_ml::FittedModel,
) -> Result<Vec<Vec<Scalar>>, String> {
    let expected = framework_ml::input_types(fitted);
    if ids.len() != expected.len() {
        return Err("Prediction feature count does not match the model".into());
    }
    let selection = ids
        .iter()
        .zip(&expected)
        .map(|(id, dtype)| {
            let expression = pl::col(id.as_str());
            match dtype {
                Dtype::Number => expression.cast(pl::DataType::Float64),
                Dtype::String => expression.cast(pl::DataType::String),
                Dtype::Boolean => expression,
            }
        })
        .collect::<Vec<_>>();
    let frame = source
        .select(selection)
        .limit(100_001)
        .collect()
        .map_err(|e| e.to_string())?;
    if frame.height() > 100_000 {
        return Err("Models currently accept at most 100,000 rows".into());
    }
    (0..frame.height()).map(|row| ids.iter().zip(&expected).map(|(id, dtype)| {
        let column = frame.column(id).map_err(|e| e.to_string())?;
        match polars_value_at(column.as_materialized_series(), row)? {
            ScalarValue::Number(value) if *dtype == Dtype::Number => Ok(Scalar::Number(value)),
            ScalarValue::String(value) if *dtype == Dtype::String => Ok(Scalar::String(value)),
            ScalarValue::Boolean(value) if *dtype == Dtype::Boolean => Ok(Scalar::Boolean(value)),
            ScalarValue::Null if *dtype == Dtype::Number => Ok(Scalar::Number(f64::NAN)),
            ScalarValue::Null => Err(format!("Model input ‘{}’ is missing at row {}; a missing string/boolean cannot be replaced by an empty string", fitted.feature_names[ids.iter().position(|i|i==id).unwrap()], row+1)),
            _ => Err("Prediction input type does not match the fitted pipeline".into()),
        }
    }).collect()).collect()
}

pub(crate) fn typed_prediction_columns(
    rows: &[Vec<Scalar>],
    ids: &[Id],
    fitted: &framework_ml::FittedModel,
) -> Result<Vec<pl::Column>, String> {
    use pl::NamedFrom;
    let types = framework_ml::output_types(fitted);
    if ids.len() != types.len() || rows.iter().any(|row| row.len() != ids.len()) {
        return Err("The model output schema changed; create a new prediction frame".into());
    }
    ids.iter()
        .zip(types)
        .enumerate()
        .map(|(index, (id, dtype))| {
            let series = match dtype {
                Dtype::Number => pl::Series::new(
                    id.as_str().into(),
                    rows.iter()
                        .map(|row| match &row[index] {
                            Scalar::Number(value) => Ok(*value),
                            _ => Err("Model returned a nonnumeric output".to_string()),
                        })
                        .collect::<Result<Vec<_>, String>>()?,
                ),
                Dtype::String => pl::Series::new(
                    id.as_str().into(),
                    rows.iter()
                        .map(|row| match &row[index] {
                            Scalar::String(value) => Ok(value.as_str()),
                            _ => Err("Model returned a nontext output".to_string()),
                        })
                        .collect::<Result<Vec<_>, String>>()?,
                ),
                Dtype::Boolean => pl::Series::new(
                    id.as_str().into(),
                    rows.iter()
                        .map(|row| match &row[index] {
                            Scalar::Boolean(value) => Ok(*value),
                            _ => Err("Model returned a nonboolean output".to_string()),
                        })
                        .collect::<Result<Vec<_>, String>>()?,
                ),
            };
            Ok(series.into())
        })
        .collect()
}

pub(crate) fn model_output_type(dtype: Dtype) -> DataType {
    match dtype {
        Dtype::Number => DataType::Number,
        Dtype::String => DataType::String,
        Dtype::Boolean => DataType::Boolean,
    }
}
