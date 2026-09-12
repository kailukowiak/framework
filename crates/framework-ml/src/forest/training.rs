use super::{ForestModel, ForestNode, ForestSettings};
use crate::{MlError, NumericDataset, Result};
use serde::Deserialize;
use smartcore::{
    ensemble::{
        random_forest_classifier::{RandomForestClassifier, RandomForestClassifierParameters},
        random_forest_regressor::{RandomForestRegressor, RandomForestRegressorParameters},
    },
    linalg::basic::matrix::DenseMatrix,
};

// Only the pinned backend's serialized training output crosses this adapter.
// The application persists the typed ForestModel below, never backend JSON or
// backend private structs whose layout could silently change on an upgrade.
#[derive(Deserialize)]
struct RawRegressor {
    forest_regressor: RawForest,
}
#[derive(Deserialize)]
struct RawForest {
    trees: Vec<RawTree>,
}
#[derive(Deserialize)]
struct RawClassifier {
    trees: Vec<RawTree>,
    classes: Vec<u32>,
}
#[derive(Deserialize)]
struct RawTree {
    nodes: Vec<RawNode>,
}
#[derive(Deserialize)]
struct RawNode {
    output: f64,
    split_feature: usize,
    split_value: Option<f64>,
    true_child: Option<usize>,
    false_child: Option<usize>,
}

pub(crate) fn train(
    data: &NumericDataset,
    settings: &ForestSettings,
    classification: bool,
) -> Result<ForestModel> {
    let features = data.rows[0].len();
    validate(settings, features, data.rows.len())?;
    let matrix =
        DenseMatrix::from_2d_vec(&data.rows).map_err(|e| MlError::Numerical(e.to_string()))?;
    let (raw, classes) = if classification {
        classifier(&matrix, &data.targets, settings)?
    } else {
        (regressor(&matrix, &data.targets, settings)?, None)
    };
    let trees = raw
        .into_iter()
        .map(|tree| {
            tree.nodes
                .into_iter()
                .map(convert)
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let model = ForestModel {
        features,
        classes,
        trees,
        settings: settings.clone(),
    };
    model.validate(features)?;
    Ok(model)
}
fn validate(settings: &ForestSettings, features: usize, rows: usize) -> Result<()> {
    if settings.trees == 0
        || settings.trees > 1000
        || settings.max_depth == 0
        || settings.max_depth > 24
        || settings.min_samples_leaf == 0
        || settings
            .max_features
            .is_some_and(|m| m == 0 || m > features)
    {
        return Err(MlError::InvalidInput("Forest settings require 1–1000 trees, depth 1–24, a positive leaf size and valid feature count".into()));
    }
    if rows
        .saturating_mul(features)
        .saturating_mul(settings.trees as usize)
        > 50_000_000
    {
        return Err(MlError::ResourceLimit(
            "Forest training exceeds its current data-by-tree work budget".into(),
        ));
    }
    Ok(())
}
fn classifier(
    matrix: &DenseMatrix<f64>,
    targets: &[f64],
    settings: &ForestSettings,
) -> Result<(Vec<RawTree>, Option<usize>)> {
    if targets
        .iter()
        .any(|v| *v < 0. || v.fract() != 0. || *v >= 1000.)
    {
        return Err(MlError::InvalidInput(
            "Forest classes must be whole-number indices starting at 0".into(),
        ));
    }
    let mut classes: Vec<u32> = targets.iter().map(|v| *v as u32).collect();
    classes.sort_unstable();
    classes.dedup();
    if classes.len() < 2 || classes.iter().enumerate().any(|(i, c)| *c != i as u32) {
        return Err(MlError::InvalidInput(
            "Forest classification needs at least two classes numbered consecutively from 0".into(),
        ));
    }
    let parameters = RandomForestClassifierParameters {
        max_depth: Some(settings.max_depth),
        min_samples_leaf: settings.min_samples_leaf,
        n_trees: settings.trees,
        m: settings.max_features,
        seed: u64::from(settings.seed),
        ..Default::default()
    };
    let labels: Vec<u32> = targets.iter().map(|v| *v as u32).collect();
    let fit = RandomForestClassifier::fit(matrix, &labels, parameters)
        .map_err(|e| MlError::Numerical(e.to_string()))?;
    let raw: RawClassifier = serde_json::from_slice(
        &serde_json::to_vec(&fit).map_err(|e| MlError::Numerical(e.to_string()))?,
    )
    .map_err(|e| MlError::Numerical(format!("Unsupported SmartCore forest shape: {e}")))?;
    if raw.classes != classes {
        return Err(MlError::Numerical(
            "Forest class order changed unexpectedly".into(),
        ));
    }
    Ok((raw.trees, Some(classes.len())))
}
fn regressor(
    matrix: &DenseMatrix<f64>,
    targets: &[f64],
    settings: &ForestSettings,
) -> Result<Vec<RawTree>> {
    let parameters = RandomForestRegressorParameters {
        max_depth: Some(settings.max_depth),
        min_samples_leaf: settings.min_samples_leaf,
        n_trees: settings.trees as usize,
        m: settings.max_features,
        seed: u64::from(settings.seed),
        ..Default::default()
    };
    let fit = RandomForestRegressor::fit(matrix, &targets.to_vec(), parameters)
        .map_err(|e| MlError::Numerical(e.to_string()))?;
    let raw: RawRegressor = serde_json::from_slice(
        &serde_json::to_vec(&fit).map_err(|e| MlError::Numerical(e.to_string()))?,
    )
    .map_err(|e| MlError::Numerical(format!("Unsupported SmartCore forest shape: {e}")))?;
    Ok(raw.forest_regressor.trees)
}
fn convert(node: RawNode) -> Result<ForestNode> {
    match (node.true_child, node.false_child, node.split_value) {
        (Some(left), Some(right), Some(threshold)) => Ok(ForestNode::Split {
            feature: node.split_feature,
            threshold,
            left,
            right,
        }),
        (None, None, _) => Ok(ForestNode::Leaf { value: node.output }),
        _ => Err(MlError::Numerical("Invalid trained forest node".into())),
    }
}
