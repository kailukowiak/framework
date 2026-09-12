use crate::ModelContract;
use crate::recipe::{Dtype, Imputation, Recipe, Role, Scalar, Step};
use crate::records::Output;
use crate::semantics::MissingValue;
use std::collections::{BTreeMap, BTreeSet};

pub fn model(model: &ModelContract) -> Result<(), String> {
    if model.schema_version != 1 {
        return Err("unsupported contract version".into());
    }
    let mut slots = BTreeMap::new();
    let mut names = BTreeSet::new();
    for input in &model.specification.inputs {
        if !names.insert(&input.slot) {
            return Err(format!("duplicate input slot {}", input.slot));
        }
        if input.role == Role::Predictor {
            slots.insert(input.slot.clone(), input.dtype);
        }
    }
    recipe(&model.specification.recipe, slots.clone(), false)?;
    recipe(&model.fitted.recipe, slots, true)?;
    model.fitted.prediction.validate()?;
    outputs(&model.fitted.outputs)
}

fn recipe(recipe: &Recipe, mut slots: BTreeMap<String, Dtype>, fitted: bool) -> Result<(), String> {
    for step in &recipe.steps {
        match step {
            Step::Impute {
                input,
                output,
                strategy,
                missing,
                learned,
            } => {
                let dtype = input_type(&slots, input)?;
                missing_type(missing, dtype)?;
                if *strategy == Imputation::Mean && dtype != Dtype::Number {
                    return Err("mean imputation requires a numeric input".into());
                }
                learned_presence(learned.is_some(), fitted)?;
                if let Some(value) = learned {
                    let value_type = match value {
                        Scalar::Number(number) if number.is_finite() => Dtype::Number,
                        Scalar::String(_) => Dtype::String,
                        Scalar::Boolean(_) => Dtype::Boolean,
                        _ => return Err("non-finite imputation value".into()),
                    };
                    if value_type != dtype {
                        return Err("imputation value has the wrong dtype".into());
                    }
                }
                insert(&mut slots, output, dtype)?;
            }
            Step::Scale {
                input,
                output,
                learned,
            } => {
                if input_type(&slots, input)? != Dtype::Number {
                    return Err("scaling requires a numeric input".into());
                }
                learned_presence(learned.is_some(), fitted)?;
                if let Some(parameters) = learned
                    && (!parameters.offset.is_finite()
                        || !parameters.scale.is_finite()
                        || parameters.scale <= 0.0)
                {
                    return Err("scale must be finite and positive".into());
                }
                insert(&mut slots, output, Dtype::Number)?;
            }
            Step::OneHot {
                input,
                outputs,
                learned,
                ..
            } => {
                if input_type(&slots, input)? != Dtype::String {
                    return Err("this spike supports string one-hot inputs only".into());
                }
                learned_presence(learned.is_some(), fitted)?;
                if let Some(levels) = learned
                    && (levels.len() != outputs.len()
                        || levels.iter().collect::<BTreeSet<_>>().len() != levels.len())
                {
                    return Err("one-hot levels must map one-to-one to output slots".into());
                }
                for output in outputs {
                    insert(&mut slots, output, Dtype::Number)?;
                }
            }
        }
    }
    let features = &recipe.features;
    if features.is_empty() || features.iter().collect::<BTreeSet<_>>().len() != features.len() {
        return Err("feature order must be nonempty and unique".into());
    }
    for feature in features {
        if input_type(&slots, feature)? != Dtype::Number {
            return Err("this spike requires numeric final features".into());
        }
    }
    Ok(())
}

fn missing_type(missing: &MissingValue, dtype: Dtype) -> Result<(), String> {
    let valid = match missing {
        MissingValue::Null => true,
        MissingValue::NaN => dtype == Dtype::Number,
        MissingValue::Number(value) => dtype == Dtype::Number && value.is_finite(),
        MissingValue::String(_) => dtype == Dtype::String,
    };
    if !valid {
        return Err("missing sentinel does not match input dtype".into());
    }
    Ok(())
}

fn learned_presence(present: bool, fitted: bool) -> Result<(), String> {
    if present != fitted {
        return Err("learned state belongs in the fitted recipe only".into());
    }
    Ok(())
}

fn input_type(slots: &BTreeMap<String, Dtype>, input: &str) -> Result<Dtype, String> {
    slots
        .get(input)
        .copied()
        .ok_or_else(|| format!("slot {input} is not an available predictor or earlier step output"))
}

fn insert(slots: &mut BTreeMap<String, Dtype>, output: &str, dtype: Dtype) -> Result<(), String> {
    if slots.insert(output.into(), dtype).is_some() {
        return Err(format!("step overwrites slot {output}"));
    }
    Ok(())
}

fn outputs(outputs: &[Output]) -> Result<(), String> {
    let mut names = BTreeSet::new();
    let mut classes = BTreeSet::new();
    for output in outputs {
        let name = match output {
            Output::Prediction { name } | Output::Probability { name, .. } => name,
            Output::Class { name, labels } => {
                for label in labels {
                    if !classes.insert(label) {
                        return Err("class labels must be unique".into());
                    }
                }
                name
            }
        };
        if !names.insert(name) {
            return Err("output names must be unique".into());
        }
    }
    for output in outputs {
        if let Output::Probability { label, .. } = output
            && !classes.contains(label)
        {
            return Err("probability output has no corresponding class label".into());
        }
    }
    Ok(())
}
