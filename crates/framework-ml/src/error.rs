use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum MlError {
    #[error("{0}")]
    InvalidInput(String),
    #[error("{0}")]
    Unsupported(String),
    #[error("{0}")]
    ResourceLimit(String),
    #[error("The predictors are linearly dependent; remove duplicate or constant columns")]
    RankDeficient,
    #[error(
        "The classes are completely or quasi-completely separated; this unpenalized logistic fit has no finite solution"
    )]
    Separation,
    #[error("Separation could not be decided reliably; no model was fitted")]
    InconclusiveSeparation,
    #[error("The model did not converge; the previous fitted model is retained")]
    NonConvergence,
    #[error("{0}")]
    Numerical(String),
    #[error("{0}")]
    InvalidModel(String),
}
