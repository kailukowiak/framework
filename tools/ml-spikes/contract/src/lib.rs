//! Executable design contract, not yet a `.fw` object or an inference engine.
//!
//! The three backend spikes consume this vocabulary while their own fixtures
//! establish which transformations and estimators we can actually support.
//! Keeping it isolated avoids promising a document format before that evidence.

pub mod recipe;
pub mod records;
pub mod semantics;
mod validate;

use recipe::{Input, Recipe};
use records::{Artifact, Output, Provenance};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelContract {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub specification: Specification,
    pub fitted: FittedRevision,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Specification {
    pub inputs: Vec<Input>,
    pub recipe: Recipe,
    pub estimator: Estimator,
}

/// Method and engine are separate even when the first release offers only
/// one engine. A backend adapter still has to check that it supports the exact
/// method and settings; engine interchangeability is not a serialization claim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "camelCase", deny_unknown_fields)]
pub enum Estimator {
    Linear { engine: String, intercept: bool },
    Logistic { engine: String, intercept: bool },
    Boosting { engine: String, task: Task },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Task {
    Regression,
    Classification,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FittedRevision {
    pub id: String,
    pub specification_fingerprint: String,
    pub provenance: Provenance,
    pub recipe: Recipe,
    pub prediction: semantics::PredictionPolicy,
    pub parameters: Artifact,
    pub outputs: Vec<Output>,
    pub summaries: Vec<records::Summary>,
}

impl ModelContract {
    pub fn validate(&self) -> Result<(), String> {
        validate::model(self)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod semantics_tests;
