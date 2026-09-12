//! Native seeded random forests. SmartCore trains; our typed, validated trees score.
mod training;
use crate::{MlError, Result};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", default)]
pub struct ForestSettings {
    pub trees: u16,
    pub max_depth: u16,
    pub min_samples_leaf: usize,
    pub max_features: Option<usize>,
    pub seed: u32,
}
impl Default for ForestSettings {
    fn default() -> Self {
        Self {
            trees: 100,
            max_depth: 8,
            min_samples_leaf: 2,
            max_features: None,
            seed: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ForestModel {
    pub(crate) features: usize,
    pub(crate) classes: Option<usize>,
    pub(crate) trees: Vec<Vec<ForestNode>>,
    pub settings: ForestSettings,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ForestNode {
    Leaf {
        value: f64,
    },
    Split {
        feature: usize,
        threshold: f64,
        left: usize,
        right: usize,
    },
}

impl ForestModel {
    pub fn classes(&self) -> Option<usize> {
        self.classes
    }
    pub fn validate(&self, features: usize) -> Result<()> {
        if self.features != features
            || self.features == 0
            || self.trees.is_empty()
            || self.trees.len() > 1000
            || self.classes.is_some_and(|k| !(2..=1000).contains(&k))
        {
            return Err(MlError::InvalidModel(
                "Invalid stored forest dimensions".into(),
            ));
        }
        let total = self
            .trees
            .iter()
            .try_fold(0_usize, |n, t| n.checked_add(t.len()))
            .ok_or_else(|| MlError::InvalidModel("Stored forest node count overflow".into()))?;
        if total > 1_000_000 {
            return Err(MlError::ResourceLimit(
                "Stored forest exceeds the one-million-node budget".into(),
            ));
        }
        for tree in &self.trees {
            validate_tree(tree, features, self.classes)?;
        }
        Ok(())
    }
    pub(crate) fn score(&self, row: &[f64]) -> Result<Vec<f64>> {
        if row.len() != self.features || row.iter().any(|v| !v.is_finite()) {
            return Err(MlError::InvalidInput(
                "Random forests require the fitted number of finite numeric predictors".into(),
            ));
        }
        let mut result = vec![0.; self.classes.unwrap_or(1)];
        for tree in &self.trees {
            let mut at = 0;
            loop {
                match tree[at] {
                    ForestNode::Leaf { value } => {
                        if self.classes.is_some() {
                            result[value as usize] += 1.;
                        } else {
                            result[0] += value / self.trees.len() as f64;
                        }
                        break;
                    }
                    ForestNode::Split {
                        feature,
                        threshold,
                        left,
                        right,
                    } => {
                        at = if row[feature] <= threshold {
                            left
                        } else {
                            right
                        }
                    }
                }
            }
        }
        if self.classes.is_some() {
            for value in &mut result {
                *value /= self.trees.len() as f64;
            }
        }
        if result.iter().any(|v| !v.is_finite()) {
            return Err(MlError::Numerical("Forest prediction overflowed".into()));
        }
        Ok(result)
    }
}

fn validate_tree(tree: &[ForestNode], features: usize, classes: Option<usize>) -> Result<()> {
    if tree.is_empty() {
        return Err(MlError::InvalidModel("Empty forest tree".into()));
    }
    let mut pending = vec![0];
    let mut seen = vec![false; tree.len()];
    while let Some(at) = pending.pop() {
        if at >= tree.len() || seen[at] {
            return Err(MlError::InvalidModel(
                "Invalid forest child or cycle".into(),
            ));
        }
        seen[at] = true;
        match tree[at] {
            ForestNode::Leaf { value } => {
                if !value.is_finite()
                    || classes
                        .is_some_and(|k| value < 0. || value.fract() != 0. || value >= k as f64)
                {
                    return Err(MlError::InvalidModel("Invalid forest leaf".into()));
                }
            }
            ForestNode::Split {
                feature,
                threshold,
                left,
                right,
            } => {
                if feature >= features || !threshold.is_finite() || left == right {
                    return Err(MlError::InvalidModel("Invalid forest split".into()));
                }
                pending.extend([left, right]);
            }
        }
    }
    if seen.iter().any(|s| !*s) {
        return Err(MlError::InvalidModel("Unreachable forest node".into()));
    }
    Ok(())
}
pub(crate) use training::train;
