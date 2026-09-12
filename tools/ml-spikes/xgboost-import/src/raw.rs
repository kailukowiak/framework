use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct File {
    pub version: [u32; 3],
    pub learner: Learner,
}
#[derive(Deserialize)]
pub(crate) struct Learner {
    pub learner_model_param: LearnerParam,
    pub gradient_booster: Booster,
    pub objective: Objective,
    pub feature_names: Vec<String>,
    pub feature_types: Vec<String>,
}
#[derive(Deserialize)]
pub(crate) struct LearnerParam {
    pub base_score: String,
    pub num_class: String,
    pub num_feature: String,
    pub num_target: String,
}
#[derive(Deserialize)]
pub(crate) struct Objective {
    pub name: String,
}
#[derive(Deserialize)]
pub(crate) struct Booster {
    pub name: String,
    pub model: Option<serde_json::Value>,
}
#[derive(Deserialize)]
pub(crate) struct Ensemble {
    pub gbtree_model_param: EnsembleParam,
    pub iteration_indptr: Vec<usize>,
    pub tree_info: Vec<usize>,
    pub trees: Vec<Tree>,
}
#[derive(Deserialize)]
pub(crate) struct EnsembleParam {
    pub num_trees: String,
    pub num_parallel_tree: String,
}
#[derive(Deserialize)]
pub(crate) struct Tree {
    pub id: usize,
    pub tree_param: TreeParam,
    pub left_children: Vec<i32>,
    pub right_children: Vec<i32>,
    pub parents: Vec<u32>,
    pub split_conditions: Vec<f32>,
    pub split_indices: Vec<usize>,
    pub split_type: Vec<u8>,
    pub default_left: Vec<u8>,
    pub base_weights: Vec<f32>,
    pub loss_changes: Vec<f32>,
    pub sum_hessian: Vec<f32>,
    pub categories: Vec<u32>,
    pub categories_nodes: Vec<u32>,
    pub categories_segments: Vec<u32>,
    pub categories_sizes: Vec<u32>,
}
#[derive(Deserialize)]
pub(crate) struct TreeParam {
    pub num_nodes: String,
    pub num_feature: String,
    pub num_deleted: String,
    pub size_leaf_vector: String,
}
