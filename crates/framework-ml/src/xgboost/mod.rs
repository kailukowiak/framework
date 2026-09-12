//! Bounded native XGBoost training and typed XGBoost 3.0 numerical gbtree inference.
mod raw;
mod tree;
mod training;
pub(crate) use training::train;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub objective: Objective,
    pub rounds: u16,
    pub max_depth: u8,
    pub learning_rate: f32,
    pub min_child_weight: f32,
    pub subsample: f32,
    pub column_subsample: f32,
    pub l2_regularization: f32,
    pub seed: u32,
    pub threads: u8,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            objective: Objective::Regression,
            rounds: 100,
            max_depth: 6,
            learning_rate: 0.1,
            min_child_weight: 1.0,
            subsample: 1.0,
            column_subsample: 1.0,
            l2_regularization: 1.0,
            seed: 0,
            threads: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct IterationRange {
    pub start: u32,
    pub end: u32,
}
impl IterationRange {
    fn validate(&self, rounds: u32) -> Result<()> {
        require(
            self.start < self.end && self.end <= rounds,
            "invalid iteration range",
        )
    }
}
use tree::Tree;

pub type Result<T> = std::result::Result<T, String>;
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "XgboostObjective")]
pub enum Objective {
    Regression,
    Binary,
    Multiclass,
}
impl Default for Objective {
    fn default() -> Self {
        Self::Regression
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "XgboostModel")]
pub struct Model {
    feature_names: Vec<String>,
    prediction_range: Option<IterationRange>,
    features: usize,
    groups: usize,
    objective: Objective,
    base_margin: f32,
    trees: Vec<Tree>,
    groups_by_tree: Vec<usize>,
    iterations: Vec<usize>,
}
pub(crate) fn require(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}
pub(crate) fn integer(value: &str) -> Result<usize> {
    value
        .parse()
        .map_err(|_| format!("invalid integer parameter: {value}"))
}
impl Model {
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        require(bytes.len() <= 16 * 1024 * 1024, "model byte resource limit")?;
        let file: raw::File = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        require(
            file.version[0..2] == [3, 0],
            "unsupported XGBoost version; require 3.0.x",
        )?;
        let learner = file.learner;
        require(
            learner.gradient_booster.name == "gbtree",
            &format!("unsupported booster: {}", learner.gradient_booster.name),
        )?;
        let features = integer(&learner.learner_model_param.num_feature)?;
        require(
            features > 0 && features <= 100_000,
            "feature resource limit",
        )?;
        require(
            learner.learner_model_param.num_target == "1",
            "multi-target unsupported",
        )?;
        require(
            learner.feature_names.is_empty() || learner.feature_names.len() == features,
            "feature name count mismatch",
        )?;
        require(
            learner.feature_types.is_empty() || learner.feature_types.len() == features,
            "feature type count mismatch",
        )?;
        require(
            learner
                .feature_types
                .iter()
                .all(|t| matches!(t.as_str(), "float" | "int" | "q" | "i")),
            "unsupported feature type (categorical features unsupported)",
        )?;
        let classes = integer(&learner.learner_model_param.num_class)?;
        let objective = match learner.objective.name.as_str() {
            "reg:squarederror" => Objective::Regression,
            "binary:logistic" => Objective::Binary,
            "multi:softprob" => Objective::Multiclass,
            other => return Err(format!("unsupported objective: {other}")),
        };
        let groups = if matches!(objective, Objective::Multiclass) {
            require((2..=1_000).contains(&classes), "class resource limit")?;
            classes
        } else {
            require(classes == 0, "unexpected num_class")?;
            1
        };
        let base: f32 = learner
            .learner_model_param
            .base_score
            .parse()
            .map_err(|_| "invalid base_score")?;
        require(base.is_finite(), "non-finite base_score")?;
        let base_margin = if matches!(objective, Objective::Binary) {
            require(
                base > 0.0 && base < 1.0,
                "logistic base_score must be in (0, 1)",
            )?;
            // Match XGBoost's float32 ProbToMargin operation order.
            -(1.0_f32 / base - 1.0).ln()
        } else {
            base
        };
        require(
            base_margin.is_finite(),
            "base_score transformation overflow",
        )?;
        let ensemble = learner
            .gradient_booster
            .model
            .ok_or("missing gbtree model")?;
        let mut result = Self::assemble(ensemble, features, groups, objective, base_margin)?;
        result.feature_names = if learner.feature_names.is_empty() {
            (0..features)
                .map(|i| format!("feature_{}", i + 1))
                .collect()
        } else {
            learner.feature_names
        };
        if let Some(best) = learner.attributes.best_iteration {
            let end = integer(&best)?
                .checked_add(1)
                .ok_or("best_iteration overflow")?;
            require(
                end <= result.rounds() as usize,
                "best_iteration exceeds stored rounds",
            )?;
            result.prediction_range = Some(IterationRange {
                start: 0,
                end: end as u32,
            });
        }
        Ok(result)
    }

    fn assemble(
        raw: raw::Ensemble,
        features: usize,
        groups: usize,
        objective: Objective,
        base_margin: f32,
    ) -> Result<Self> {
        let n = raw.trees.len();
        require(n > 0 && n <= 10_000, "tree resource limit")?;
        require(
            integer(&raw.gbtree_model_param.num_trees)? == n && raw.tree_info.len() == n,
            "tree count mismatch",
        )?;
        require(
            raw.gbtree_model_param.num_parallel_tree == "1",
            "parallel forest trees unsupported",
        )?;
        require(
            raw.iteration_indptr.len() >= 2
                && raw.iteration_indptr[0] == 0
                && raw.iteration_indptr.last() == Some(&n),
            "invalid iteration pointers",
        )?;
        for window in raw.iteration_indptr.windows(2) {
            require(
                window[0] < window[1] && window[1] <= n && window[1] - window[0] == groups,
                "invalid iteration grouping",
            )?;
            let mut seen = vec![false; groups];
            for &group in &raw.tree_info[window[0]..window[1]] {
                require(group < groups && !seen[group], "invalid tree output group")?;
                seen[group] = true;
            }
        }
        let mut total_nodes = 0_usize;
        let mut trees = Vec::with_capacity(n);
        for (id, tree) in raw.trees.into_iter().enumerate() {
            total_nodes += tree.left_children.len();
            require(total_nodes <= 1_000_000, "total node resource limit")?;
            trees.push(Tree::parse(tree, features, id)?);
        }
        Ok(Self {
            feature_names: Vec::new(),
            prediction_range: None,
            features,
            groups,
            objective,
            base_margin,
            trees,
            groups_by_tree: raw.tree_info,
            iterations: raw.iteration_indptr,
        })
    }

    pub fn set_prediction_range(&mut self, range: IterationRange) -> Result<()> {
        range.validate(self.rounds())?;
        self.prediction_range = Some(range);
        Ok(())
    }

    pub fn feature_names(&self) -> &[String] {
        &self.feature_names
    }
    pub fn objective(&self) -> Objective {
        self.objective
    }
    pub fn groups(&self) -> usize {
        self.groups
    }
    pub fn prediction_range(&self) -> Option<IterationRange> {
        self.prediction_range
    }

    pub fn validate(&self) -> Result<()> {
        require(
            self.features > 0
                && self.features <= 100_000
                && self.feature_names.len() == self.features,
            "stored feature count mismatch",
        )?;
        require(
            self.groups > 0 && self.groups <= 1000 && self.base_margin.is_finite(),
            "invalid stored objective shape",
        )?;
        require(
            self.trees.len() == self.groups_by_tree.len() && self.trees.len() <= 10000,
            "invalid stored ensemble",
        )?;
        require(
            self.iterations.first() == Some(&0)
                && self.iterations.last() == Some(&self.trees.len())
                && self.iterations.len() >= 2,
            "invalid stored iterations",
        )?;
        require(
            self.iterations
                .windows(2)
                .all(|v| v[0] < v[1] && v[1] <= self.trees.len()),
            "invalid stored iteration order",
        )?;
        require(
            self.groups_by_tree.iter().all(|g| *g < self.groups),
            "invalid stored output group",
        )?;
        if let Some(range) = self.prediction_range {
            range.validate(self.rounds())?;
        }
        require(
            match self.objective {
                Objective::Regression | Objective::Binary => self.groups == 1,
                Objective::Multiclass => self.groups >= 2,
            },
            "stored objective/output mismatch",
        )?;
        for window in self.iterations.windows(2) {
            require(
                window[1] - window[0] == self.groups,
                "stored iteration group width mismatch",
            )?;
            let mut seen = vec![false; self.groups];
            for &group in &self.groups_by_tree[window[0]..window[1]] {
                require(!seen[group], "duplicate stored iteration group")?;
                seen[group] = true;
            }
        }
        let nodes = self
            .trees
            .iter()
            .try_fold(0_usize, |sum, tree| sum.checked_add(tree.0.len()))
            .ok_or("stored node count overflow")?;
        require(nodes <= 1_000_000, "stored node resource limit")?;
        for tree in &self.trees {
            tree.validate(self.features)?;
        }
        Ok(())
    }

    pub fn rounds(&self) -> u32 {
        (self.iterations.len() - 1) as u32
    }

    /// An explicit range overrides the preserved imported prediction policy.
    /// Without an override, best_iteration is honored when present.
    /// Columns are in trained positional order. Null/NaN means missing.
    pub(crate) fn predict(
        &self,
        row: &[Option<f64>],
        range: Option<IterationRange>,
    ) -> Result<Vec<f32>> {
        require(
            row.len() == self.features,
            "prediction feature count mismatch",
        )?;
        let row: Vec<f32> = row.iter().map(|v| v.unwrap_or(f64::NAN) as f32).collect();
        require(
            row.iter().all(|v| v.is_finite() || v.is_nan()),
            "infinite or overflowing feature",
        )?;
        let range = range.or(self.prediction_range).unwrap_or(IterationRange {
            start: 0,
            end: self.rounds(),
        });
        range.validate(self.rounds())?;
        let mut output = vec![self.base_margin; self.groups];
        for index in self.iterations[range.start as usize]..self.iterations[range.end as usize] {
            output[self.groups_by_tree[index]] += self.trees[index].score(&row);
        }
        require(
            output.iter().all(|v| v.is_finite()),
            "prediction margin overflow",
        )?;
        match self.objective {
            Objective::Regression => {}
            Objective::Binary => output[0] = 1.0 / (1.0 + (-output[0]).exp()),
            Objective::Multiclass => {
                let max = output.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                for value in &mut output {
                    *value = (*value - max).exp();
                }
                let sum: f32 = output.iter().sum();
                for value in &mut output {
                    *value /= sum;
                }
            }
        }
        Ok(output)
    }
}
