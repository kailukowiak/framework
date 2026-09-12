//! Bounded ONNX import spike. Only the validated sklearn branched linear and
//! binary-logistic motifs are accepted; this is not a general ONNX runtime.
mod attributes;
mod lower;
mod motif;
mod proto;
mod score;
mod wire;

use framework_ml_contract_spike::{
    recipe::{Input, Recipe},
    semantics::ClassLabel,
};
use prost::Message;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

fn ensure(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error(message.into()))
    }
}

#[derive(Debug, Clone)]
pub struct ImportedLinear {
    inputs: Vec<Input>,
    recipe: Recipe,
    coefficients: Vec<f32>,
    intercepts: Vec<f32>,
    labels: Vec<ClassLabel>,
}

impl ImportedLinear {
    pub fn inputs(&self) -> &[Input] {
        &self.inputs
    }
    pub fn recipe(&self) -> &Recipe {
        &self.recipe
    }
    pub fn coefficients(&self) -> &[f32] {
        &self.coefficients
    }
    pub fn intercepts(&self) -> &[f32] {
        &self.intercepts
    }
    pub fn labels(&self) -> &[ClassLabel] {
        &self.labels
    }
}

pub fn import(bytes: &[u8]) -> Result<ImportedLinear> {
    wire::check(bytes)?;
    let model = proto::Model::decode(bytes).map_err(|e| Error(format!("malformed ONNX: {e}")))?;
    let (graph, classification) = motif::check(&model)?;
    lower::lower(graph, classification)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Prediction {
    Regression(f32),
    Classification {
        label: ClassLabel,
        probabilities: [f32; 2],
    },
}

#[cfg(test)]
mod tests;
