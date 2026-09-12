use crate::recipe::{ScaleOperation, ScaleParameters};
use crate::semantics::{
    ClassLabel, IterationRange, MissingValue, NumericPrecision, PredictionPolicy,
};

#[test]
fn numeric_and_string_labels_never_collapse() {
    let labels = [ClassLabel::Integer(1), ClassLabel::String("1".into())];
    let encoded = serde_json::to_string(&labels).unwrap();
    let decoded: [ClassLabel; 2] = serde_json::from_str(&encoded).unwrap();
    assert_eq!(labels, decoded);
    assert_ne!(decoded[0], decoded[1]);
}

#[test]
fn missing_policies_retain_their_exact_meaning() {
    let policies = [
        MissingValue::NaN,
        MissingValue::Null,
        MissingValue::String("".into()),
    ];
    let json = serde_json::to_string(&policies).unwrap();
    let decoded: [MissingValue; 3] = serde_json::from_str(&json).unwrap();
    assert_eq!(policies, decoded);
    assert_ne!(decoded[0], decoded[1]);
    assert_ne!(decoded[1], decoded[2]);
}

#[test]
fn iteration_ranges_are_explicit_nonempty_boosting_rounds() {
    assert!(IterationRange { start: 0, end: 1 }.validate(4).is_ok());
    for range in [
        IterationRange { start: 0, end: 0 },
        IterationRange { start: 2, end: 1 },
        IterationRange { start: 0, end: 5 },
    ] {
        assert!(range.validate(4).is_err());
    }
}

#[test]
fn original_scale_operator_survives_serialization() {
    let scale = ScaleParameters {
        offset: 2.0,
        scale: 0.1,
        operation: ScaleOperation::Multiply,
    };
    let json = serde_json::to_string(&scale).unwrap();
    let decoded: ScaleParameters = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, scale);
    assert_eq!(decoded.operation, ScaleOperation::Multiply);
}

#[test]
fn invalid_thresholds_and_empty_ranges_are_refused() {
    let mut policy = PredictionPolicy {
        precision: NumericPrecision::Float32,
        iteration_range: None,
        threshold: Some(0.5),
    };
    policy.validate().unwrap();
    for threshold in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        policy.threshold = Some(threshold);
        assert!(policy.validate().is_err());
    }
    policy.threshold = None;
    policy.iteration_range = Some(IterationRange { start: 0, end: 0 });
    assert!(policy.validate().is_err());
}
