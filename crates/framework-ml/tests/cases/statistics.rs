use framework_ml::*;
use serde_json::Value;

#[test]
fn summaries_match_independent_scipy_references() {
    let fixture: Value = serde_json::from_str(include_str!("../fixtures/stats.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let x: Vec<f64> = serde_json::from_value(case["x"].clone()).unwrap();
        let y: Option<Vec<f64>> = serde_json::from_value(case["y"].clone()).unwrap();
        let request = StatsRequest {
            method: serde_json::from_value(case["method"].clone()).unwrap(),
            confidence_level: case["confidenceLevel"].as_f64().unwrap(),
        };
        let result = statistics(&request, &x, y.as_deref()).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        for key in [
            "estimate",
            "standardError",
            "statistic",
            "degreesOfFreedom",
            "pValue",
            "confidenceLower",
            "confidenceUpper",
        ] {
            if let Some(expected) = case[key].as_f64() {
                let actual = actual[key].as_f64().unwrap();
                assert!(
                    (actual - expected).abs() < 1e-8 + 1e-6 * expected.abs(),
                    "{key}: {actual} vs {expected}"
                );
            }
        }
    }
}

#[test]
fn degenerate_missing_and_misaligned_statistics_are_explicit() {
    let request = StatsRequest {
        method: StatsMethod::PearsonCorrelation,
        confidence_level: 0.95,
    };
    assert!(statistics(&request, &[1., 1., 1.], Some(&[1., 2., 3.])).is_err());
    assert!(statistics(&request, &[1., 2., 3.], Some(&[1., 2.])).is_err());
    let request = StatsRequest {
        method: StatsMethod::WelchDifference,
        confidence_level: 0.95,
    };
    assert!(statistics(&request, &[1., 1.], Some(&[2., 2.])).is_err());
    let request = StatsRequest {
        method: StatsMethod::MeanConfidence,
        confidence_level: 0.95,
    };
    assert!(statistics(&request, &[f64::NAN, 1.], None).is_err());
    assert!(statistics(&request, &[1.], None).is_err());
    let constant = statistics(&request, &[2., 2., 2.], None).unwrap();
    assert_eq!(constant.confidence_lower, Some(2.));
    assert!(!constant.warnings.is_empty());
}
