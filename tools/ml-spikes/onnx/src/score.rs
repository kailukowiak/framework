use crate::{Error, ImportedLinear, Prediction, Result, ensure};
use framework_ml_contract_spike::{
    recipe::{Scalar, ScaleOperation, Step},
    semantics::MissingValue,
};
use std::collections::HashMap;

fn number(value: &Scalar) -> Result<f32> {
    if let Scalar::Number(value) = value {
        Ok(*value as f32)
    } else {
        Err(Error("expected numeric feature".into()))
    }
}

fn lookup<'a>(values: &'a HashMap<String, Scalar>, slot: &str) -> Result<&'a Scalar> {
    values
        .get(slot)
        .ok_or_else(|| Error(format!("missing fitted slot {slot}")))
}

fn apply(step: &Step, values: &mut HashMap<String, Scalar>) -> Result<()> {
    match step {
        Step::Impute {
            input,
            output,
            missing,
            learned: Some(fill),
            ..
        } => {
            let value = lookup(values, input)?;
            let replace = match (value, missing) {
                (Scalar::Number(value), MissingValue::NaN) => value.is_nan(),
                (Scalar::String(value), MissingValue::String(sentinel)) => value == sentinel,
                _ => return Err(Error("unsupported imputation type/policy".into())),
            };
            values.insert(
                output.clone(),
                if replace { fill.clone() } else { value.clone() },
            );
        }
        Step::Scale {
            input,
            output,
            learned: Some(parameters),
        } => {
            let centered = number(lookup(values, input)?)? - parameters.offset as f32;
            let result = match parameters.operation {
                ScaleOperation::Multiply => centered * parameters.scale as f32,
                ScaleOperation::Divide => centered / parameters.scale as f32,
            };
            ensure(result.is_finite(), "nonfinite scaled result")?;
            values.insert(output.clone(), Scalar::Number(f64::from(result)));
        }
        Step::OneHot {
            input,
            outputs,
            learned: Some(categories),
            ..
        } => {
            let Scalar::String(value) = lookup(values, input)? else {
                return Err(Error("expected string category".into()));
            };
            let index = categories.iter().position(|category| category == value);
            for (i, output) in outputs.iter().enumerate() {
                values.insert(
                    output.clone(),
                    Scalar::Number(f64::from(u8::from(index == Some(i)))),
                );
            }
        }
        _ => return Err(Error("unfitted recipe step".into())),
    }
    Ok(())
}

fn sigmoid(score: f32) -> f32 {
    if score >= 0.0 {
        1.0 / (1.0 + (-score).exp())
    } else {
        let exp = score.exp();
        exp / (1.0 + exp)
    }
}

impl ImportedLinear {
    /// Inputs are in `inputs()` order, with numeric nulls converted explicitly
    /// to NaN by the caller. Empty strings are not interchangeable with nulls.
    pub fn predict(&self, rows: &[Vec<Scalar>]) -> Result<Vec<Prediction>> {
        ensure(rows.len() <= 10_000, "scoring row limit (10000)")?;
        rows.iter().map(|row| self.predict_row(row)).collect()
    }

    fn predict_row(&self, row: &[Scalar]) -> Result<Prediction> {
        ensure(row.len() == self.inputs.len(), "incorrect raw input count")?;
        for (i, value) in row.iter().enumerate() {
            let valid = match value {
                Scalar::Number(value) => i < 2 && !(*value as f32).is_infinite(),
                Scalar::String(value) => i == 2 && value.len() <= 1024,
                Scalar::Boolean(_) => false,
            };
            ensure(
                valid,
                "incorrect raw input dtype, string limit or infinite number",
            )?;
        }
        let mut values: HashMap<_, _> = self
            .inputs
            .iter()
            .zip(row)
            .map(|(input, value)| (input.slot.clone(), value.clone()))
            .collect();
        for step in &self.recipe.steps {
            apply(step, &mut values)?;
        }
        let features: Vec<_> = self
            .recipe
            .features
            .iter()
            .map(|slot| number(lookup(&values, slot)?))
            .collect::<Result<_>>()?;
        let scores: Vec<f32> = self
            .coefficients
            .chunks_exact(features.len())
            .zip(&self.intercepts)
            .map(|(coef, intercept)| {
                coef.iter()
                    .zip(&features)
                    .fold(0.0_f32, |sum, (coef, value)| sum + coef * value)
                    + intercept
            })
            .collect();
        ensure(
            scores.iter().all(|score| score.is_finite()),
            "nonfinite linear score",
        )?;
        if self.labels.is_empty() {
            return Ok(Prediction::Regression(scores[0]));
        }
        let index = usize::from(scores[1] > scores[0]);
        Ok(Prediction::Classification {
            label: self.labels[index].clone(),
            probabilities: [sigmoid(scores[0]), sigmoid(scores[1])],
        })
    }
}
