use super::{Model, Objective, Settings};
use crate::{MlError, NumericDataset, Result};
#[cfg(any(target_os = "macos", target_os = "windows"))]
use xgb::{Booster, DMatrix, parameters::BoosterParameters};

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn train(
    _data: &NumericDataset,
    _feature_names: &[String],
    _settings: &Settings,
) -> Result<Model> {
    Err(MlError::Unsupported(
        "Native XGBoost training is available on macOS and Windows".into(),
    ))
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) fn train(
    data: &NumericDataset,
    feature_names: &[String],
    settings: &Settings,
) -> Result<Model> {
    validate(data, settings)?;
    let rows = data.rows.len();
    let mut values = Vec::with_capacity(rows.saturating_mul(feature_names.len()));
    for &value in data.rows.iter().flatten() {
        let value = value as f32;
        if !value.is_finite() {
            return Err(MlError::InvalidInput(
                "XGBoost predictors must fit in finite 32-bit numbers".into(),
            ));
        }
        values.push(value);
    }
    let labels: Vec<f32> = data.targets.iter().map(|&value| value as f32).collect();
    if labels.iter().any(|value| !value.is_finite()) {
        return Err(MlError::InvalidInput(
            "XGBoost targets must fit in finite 32-bit numbers".into(),
        ));
    }
    let classes = validate_targets(&labels, settings.objective)?;
    let mut matrix = DMatrix::from_dense(&values, rows).map_err(native_error)?;
    matrix.set_labels(&labels).map_err(native_error)?;
    let mut booster = Booster::new_with_cached_dmats(&BoosterParameters::default(), &[&matrix])
        .map_err(native_error)?;
    let objective = match settings.objective {
        Objective::Regression => "reg:squarederror",
        Objective::Binary => "binary:logistic",
        Objective::Multiclass => "multi:softprob",
    };
    let parameters = [
        ("objective", objective.to_owned()),
        ("tree_method", "hist".to_owned()),
        ("max_depth", settings.max_depth.to_string()),
        ("eta", settings.learning_rate.to_string()),
        ("min_child_weight", settings.min_child_weight.to_string()),
        ("subsample", settings.subsample.to_string()),
        ("colsample_bytree", settings.column_subsample.to_string()),
        ("lambda", settings.l2_regularization.to_string()),
        ("seed", settings.seed.to_string()),
        ("nthread", settings.threads.to_string()),
        ("num_class", classes.unwrap_or(0).to_string()),
    ];
    for (name, value) in parameters {
        booster.set_param(name, &value).map_err(native_error)?;
    }
    let names: Vec<&str> = feature_names.iter().map(String::as_str).collect();
    booster.set_feature_names(&names).map_err(native_error)?;
    for round in 0..u32::from(settings.rounds) {
        booster
            .update(&matrix, round as i32)
            .map_err(native_error)?;
    }
    let bytes = booster.save_buffer(false).map_err(native_error)?;
    let model = Model::from_json(&bytes).map_err(|error| {
        MlError::Numerical(format!(
            "XGBoost produced an unsupported saved model: {error}"
        ))
    })?;
    model.validate().map_err(MlError::InvalidModel)?;
    Ok(model)
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn validate(data: &NumericDataset, settings: &Settings) -> Result<()> {
    let features = data.rows[0].len();
    let valid_fraction = |value: f32| value.is_finite() && value > 0.0 && value <= 1.0;
    if settings.rounds == 0
        || settings.rounds > 1000
        || settings.max_depth == 0
        || settings.max_depth > 16
        || !settings.learning_rate.is_finite()
        || settings.learning_rate <= 0.0
        || settings.learning_rate > 1.0
        || !settings.min_child_weight.is_finite()
        || settings.min_child_weight < 0.0
        || settings.min_child_weight > 1_000_000.0
        || !valid_fraction(settings.subsample)
        || !valid_fraction(settings.column_subsample)
        || !settings.l2_regularization.is_finite()
        || settings.l2_regularization < 0.0
        || settings.l2_regularization > 1_000_000.0
        || settings.threads == 0
        || settings.threads > 16
    {
        return Err(MlError::InvalidInput("XGBoost settings require 1–1000 rounds, depth 1–16, learning rate in (0, 1], row and column samples in (0, 1], nonnegative bounded weights, and 1–16 threads".into()));
    }
    if data
        .rows
        .len()
        .saturating_mul(features)
        .saturating_mul(settings.rounds as usize)
        > 500_000_000
    {
        return Err(MlError::ResourceLimit(
            "XGBoost training exceeds its current data-by-round work budget".into(),
        ));
    }
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn validate_targets(labels: &[f32], objective: Objective) -> Result<Option<usize>> {
    if objective == Objective::Regression {
        return Ok(None);
    }
    let mut classes: Vec<usize> = labels
        .iter()
        .map(|&value| {
            if value < 0.0 || value.fract() != 0.0 || value >= 1000.0 {
                Err(MlError::InvalidInput(
                    "XGBoost classification needs whole-number class indices starting at 0".into(),
                ))
            } else {
                Ok(value as usize)
            }
        })
        .collect::<Result<_>>()?;
    classes.sort_unstable();
    classes.dedup();
    if classes.len() < 2
        || classes
            .iter()
            .enumerate()
            .any(|(index, class)| index != *class)
    {
        return Err(MlError::InvalidInput(
            "XGBoost classification needs at least two classes numbered consecutively from 0"
                .into(),
        ));
    }
    if objective == Objective::Binary && classes.len() != 2 {
        return Err(MlError::InvalidInput(
            "Binary XGBoost requires exactly classes 0 and 1".into(),
        ));
    }
    if objective == Objective::Multiclass && classes.len() < 3 {
        return Err(MlError::InvalidInput(
            "Multiclass XGBoost requires at least three classes".into(),
        ));
    }
    Ok((objective == Objective::Multiclass).then_some(classes.len()))
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn native_error(error: xgb::XGBError) -> MlError {
    MlError::Numerical(format!("XGBoost training failed: {error}"))
}
