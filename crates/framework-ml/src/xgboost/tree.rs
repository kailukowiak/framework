use super::{Result, integer, raw, require};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", content = "data", rename_all = "camelCase")]
#[ts(rename = "XgboostNode")]
pub enum Node {
    Leaf(f32),
    Split {
        feature: usize,
        threshold: f32,
        left: usize,
        right: usize,
        default_left: bool,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(rename = "XgboostTree")]
pub struct Tree(pub Vec<Node>);
impl Tree {
    pub fn parse(raw: raw::Tree, features: usize, expected_id: usize) -> Result<Self> {
        let n = integer(&raw.tree_param.num_nodes)?;
        require(n > 0 && n <= 100_000, "tree node resource limit")?;
        require(raw.id == expected_id, "tree id mismatch")?;
        require(
            integer(&raw.tree_param.num_feature)? == features,
            "tree feature count mismatch",
        )?;
        require(
            raw.tree_param.num_deleted == "0",
            "deleted nodes unsupported",
        )?;
        require(
            raw.tree_param.size_leaf_vector == "1",
            "vector leaves unsupported",
        )?;
        let lengths = [
            raw.left_children.len(),
            raw.right_children.len(),
            raw.parents.len(),
            raw.split_conditions.len(),
            raw.split_indices.len(),
            raw.split_type.len(),
            raw.default_left.len(),
            raw.base_weights.len(),
            raw.loss_changes.len(),
            raw.sum_hessian.len(),
        ];
        require(
            lengths.into_iter().all(|len| len == n),
            "inconsistent node arrays",
        )?;
        require(
            raw.categories.is_empty()
                && raw.categories_nodes.is_empty()
                && raw.categories_segments.is_empty()
                && raw.categories_sizes.is_empty()
                && raw.split_type.iter().all(|t| *t == 0),
            "categorical splits unsupported",
        )?;
        require(
            raw.split_conditions
                .iter()
                .chain(&raw.base_weights)
                .chain(&raw.loss_changes)
                .chain(&raw.sum_hessian)
                .all(|x| x.is_finite()),
            "non-finite node data",
        )?;
        require(
            raw.default_left.iter().all(|d| *d <= 1),
            "invalid default branch",
        )?;
        require(raw.parents[0] == 2_147_483_647, "invalid root parent")?;
        let mut nodes = Vec::with_capacity(n);
        let mut incoming = vec![0_u8; n];
        for i in 0..n {
            let (left, right) = (raw.left_children[i], raw.right_children[i]);
            if left == -1 && right == -1 {
                nodes.push(Node::Leaf(raw.split_conditions[i]));
            } else {
                require(
                    left >= 0
                        && right >= 0
                        && (left as usize) < n
                        && (right as usize) < n
                        && left != right,
                    "invalid child indices",
                )?;
                let (left, right) = (left as usize, right as usize);
                for child in [left, right] {
                    require(
                        child != 0 && incoming[child] == 0,
                        "cycle or multiple parents",
                    )?;
                    incoming[child] += 1;
                    require(raw.parents[child] as usize == i, "parent index mismatch")?;
                }
                require(raw.split_indices[i] < features, "invalid split feature")?;
                nodes.push(Node::Split {
                    feature: raw.split_indices[i],
                    threshold: raw.split_conditions[i],
                    left,
                    right,
                    default_left: raw.default_left[i] == 1,
                });
            }
        }
        validate_reachable(&nodes)?;
        Ok(Self(nodes))
    }

    pub(super) fn validate(&self, features: usize) -> Result<()> {
        require(!self.0.is_empty(), "empty stored tree")?;
        for node in &self.0 {
            match node {
                Node::Leaf(value) => require(value.is_finite(), "nonfinite stored leaf")?,
                Node::Split {
                    feature,
                    threshold,
                    left,
                    right,
                    ..
                } => {
                    require(
                        *feature < features && threshold.is_finite(),
                        "invalid stored split",
                    )?;
                    require(
                        *left < self.0.len() && *right < self.0.len() && left != right,
                        "invalid stored children",
                    )?;
                }
            }
        }
        validate_reachable(&self.0)
    }

    pub fn score(&self, row: &[f32]) -> f32 {
        let mut current = 0;
        loop {
            match self.0[current] {
                Node::Leaf(value) => return value,
                Node::Split {
                    feature,
                    threshold,
                    left,
                    right,
                    default_left,
                } => {
                    let x = row[feature];
                    let go_left = if x.is_nan() {
                        default_left
                    } else {
                        x < threshold
                    };
                    current = if go_left { left } else { right };
                }
            }
        }
    }
}

fn validate_reachable(nodes: &[Node]) -> Result<()> {
    // Iterative reachability both rejects disconnected cycles and avoids
    // consuming the call stack for a deeply nested but valid input tree.
    let mut pending = vec![0];
    let mut seen = vec![false; nodes.len()];
    while let Some(index) = pending.pop() {
        require(!seen[index], "tree cycle")?;
        seen[index] = true;
        if let Node::Split { left, right, .. } = nodes[index] {
            pending.extend([left, right]);
        }
    }
    require(
        seen.into_iter().all(|s| s),
        "unreachable nodes or disconnected cycle",
    )?;
    Ok(())
}
