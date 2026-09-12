use super::{ImportedLinear, Result, ensure};
use crate::pipeline_types::{
    Dtype, MissingValue, NumericPrecision, Scalar, ScaleOperation, Step, UnknownCategory,
};
use std::collections::HashMap;

impl ImportedLinear {
    /// Documents may contain edited payloads. Validate saved plans with the same
    /// bounds as the translator before scoring; successful protobuf import is
    /// not a permanent trust boundary after serialization.
    pub fn validate(&self) -> Result<()> {
        ensure(self.inputs.len() == 3, "expected three pipeline inputs")?;
        let mut slots = HashMap::new();
        for (i, input) in self.inputs.iter().enumerate() {
            let expected = if i == 2 { Dtype::String } else { Dtype::Number };
            ensure(
                input.dtype == expected && !input.name.is_empty() && input.name.len() <= 1024,
                "invalid pipeline input",
            )?;
            insert(&mut slots, &input.slot, expected)?;
        }
        ensure(
            self.recipe.precision == NumericPrecision::Float32 && self.recipe.steps.len() == 6,
            "unsupported fitted recipe",
        )?;
        for step in &self.recipe.steps {
            validate_step(step, &mut slots)?;
        }
        let width = self.recipe.features.len();
        ensure((3..=34).contains(&width), "invalid fitted feature count")?;
        for feature in &self.recipe.features {
            ensure(
                slots.get(feature) == Some(&Dtype::Number),
                "invalid estimator feature",
            )?;
        }
        let classes = if self.labels.is_empty() { 1 } else { 2 };
        ensure(
            self.coefficients.len() == classes * width
                && self.intercepts.len() == classes
                && self
                    .coefficients
                    .iter()
                    .chain(&self.intercepts)
                    .all(|v| v.is_finite()),
            "invalid fitted coefficient dimensions or values",
        )?;
        if classes == 2 {
            ensure(
                self.labels.len() == 2
                    && self.labels[0] != self.labels[1]
                    && self.intercepts[0] == -self.intercepts[1]
                    && self.coefficients[..width]
                        .iter()
                        .zip(&self.coefficients[width..])
                        .all(|(a, b)| *a == -*b),
                "invalid binary classifier layout",
            )?;
        }
        Ok(())
    }
}

fn insert(slots: &mut HashMap<String, Dtype>, name: &str, dtype: Dtype) -> Result<()> {
    ensure(
        !name.is_empty() && name.len() <= 1024 && !slots.contains_key(name),
        "invalid or duplicate fitted slot",
    )?;
    slots.insert(name.into(), dtype);
    Ok(())
}

fn validate_step(step: &Step, slots: &mut HashMap<String, Dtype>) -> Result<()> {
    match step {
        Step::Impute {
            input,
            output,
            missing,
            learned: Some(fill),
            ..
        } => {
            let dtype = match (missing, fill) {
                (MissingValue::NaN, Scalar::Number(v)) if (*v as f32).is_finite() => Dtype::Number,
                (MissingValue::String(s), Scalar::String(v))
                    if s.len() <= 1024 && v.len() <= 1024 =>
                {
                    Dtype::String
                }
                _ => return Err(super::Error("invalid fitted imputation".into())),
            };
            ensure(
                slots.get(input) == Some(&dtype),
                "imputation input type mismatch",
            )?;
            insert(slots, output, dtype)
        }
        Step::Scale {
            input,
            output,
            learned: Some(p),
        } => {
            ensure(
                slots.get(input) == Some(&Dtype::Number)
                    && (p.offset as f32).is_finite()
                    && (p.scale as f32).is_finite()
                    && (p.operation != ScaleOperation::Divide || p.scale as f32 != 0.0),
                "invalid fitted scale",
            )?;
            insert(slots, output, Dtype::Number)
        }
        Step::OneHot {
            input,
            outputs,
            unknown: UnknownCategory::AllZero,
            learned: Some(categories),
        } => {
            ensure(
                slots.get(input) == Some(&Dtype::String)
                    && !categories.is_empty()
                    && categories.len() <= 32
                    && outputs.len() == categories.len()
                    && categories.iter().all(|s| s.len() <= 1024)
                    && categories
                        .iter()
                        .collect::<std::collections::HashSet<_>>()
                        .len()
                        == categories.len(),
                "invalid fitted category encoder",
            )?;
            for output in outputs {
                insert(slots, output, Dtype::Number)?;
            }
            Ok(())
        }
        _ => Err(super::Error("unsupported or unfitted recipe step".into())),
    }
}
