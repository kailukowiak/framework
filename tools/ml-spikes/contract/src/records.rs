use crate::semantics::ClassLabel;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Artifact {
    pub sha256: String,
    pub format: String,
    pub format_version: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Provenance {
    Native {
        source_frame_id: String,
        source_revision: String,
        training_fingerprint: String,
        backend_version: String,
        trained_at: String,
    },
    Imported {
        source: Artifact,
        exporter_version: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Output {
    Prediction {
        name: String,
    },
    Class {
        name: String,
        labels: Vec<ClassLabel>,
    },
    Probability {
        name: String,
        label: ClassLabel,
    },
}

/// The first spike names retained results rather than fabricating statistical
/// values. Format-specific artifacts will provide the coefficient/covariance
/// payload; unsupported result types must remain absent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Summary {
    pub shape: SummaryShape,
    pub artifact: Artifact,
    pub covariance: Option<Covariance>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SummaryShape {
    Components,
    Model,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Covariance {
    Classical,
    Hc3,
}

/// Evaluations reference one immutable fit and a particular assessment set.
/// Comparing two fits therefore cannot silently compare different random splits.
/// Membership is an artifact tied to a source revision, never a live ordinal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Evaluation {
    pub fitted_revision_id: String,
    pub split_id: String,
    pub source_revision: String,
    pub membership: Artifact,
    pub role: AssessmentRole,
    pub metrics: Vec<Metric>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssessmentRole {
    Validation,
    Test,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Metric {
    pub name: String,
    pub estimate: Option<f64>,
    pub unavailable_reason: Option<String>,
    pub event_class: Option<ClassLabel>,
}
