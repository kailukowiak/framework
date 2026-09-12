use framework_statistics_spike::logistic::{
    fit, DetectionError, FitError, IntervalMethod, Settings,
};
use serde_json::Value;
use std::time::Duration;

fn reference() -> (Vec<Vec<f64>>, Vec<f64>, Value) {
    let values: Value = serde_json::from_str(include_str!("../reference.json")).unwrap();
    let case = &values["logistic"];
    (
        serde_json::from_value(case["x"].clone()).unwrap(),
        serde_json::from_value(case["y"].clone()).unwrap(),
        case["classical"].clone(),
    )
}
fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-8 + 1e-6 * expected.abs(),
        "actual={actual}, expected={expected}"
    );
}

#[test]
fn ordinary_fit_including_wald_intervals_matches_independent_reference() {
    let (x, y, expected) = reference();
    let result = fit(&x, &y, Settings::default()).unwrap();
    assert_eq!(result.interval_method, IntervalMethod::NormalWald);
    for (j, coefficient) in result.coefficients.iter().enumerate() {
        for (actual, key) in [
            (coefficient.estimate, "coefficients"),
            (coefficient.standard_error, "se"),
            (coefficient.z_statistic, "statistic"),
            (coefficient.p_value, "p"),
        ] {
            close(actual, expected[key][j].as_f64().unwrap());
        }
        for k in 0..2 {
            close(
                coefficient.confidence_interval[k],
                expected["ci"][j][k].as_f64().unwrap(),
            );
        }
    }
    for (i, &value) in result.probabilities.iter().enumerate() {
        close(value, expected["predictions"][i].as_f64().unwrap());
    }
}

#[test]
fn changing_feature_units_preserves_predictions_and_transforms_inference() {
    let (x, y, _) = reference();
    let original = fit(&x, &y, Settings::default()).unwrap();
    for units in [[1e12, 1e-12], [1e-12, -1e12]] {
        let scaled: Vec<Vec<_>> = x
            .iter()
            .map(|row| {
                row.iter()
                    .zip(units)
                    .map(|(value, factor)| value * factor)
                    .collect()
            })
            .collect();
        let result = fit(&scaled, &y, Settings::default()).unwrap();
        for (j, factor) in units.iter().enumerate() {
            close(
                result.coefficients[j + 1].estimate * factor,
                original.coefficients[j + 1].estimate,
            );
            close(
                result.coefficients[j + 1].standard_error * factor.abs(),
                original.coefficients[j + 1].standard_error,
            );
        }
        for (a, b) in result.probabilities.iter().zip(&original.probabilities) {
            close(*a, *b);
        }
    }
}

fn separated(quasi: bool) -> (Vec<Vec<f64>>, Vec<f64>) {
    // Neither individual column separates; x1+x2 is the separating direction.
    let mut x = vec![
        vec![2., -1.],
        vec![-1., 2.],
        vec![1., 1.],
        vec![-2., 1.],
        vec![1., -2.],
        vec![-1., -1.],
    ];
    let mut y = vec![1., 1., 1., 0., 0., 0.];
    if quasi {
        x.extend([vec![0., 0.], vec![0., 0.]]);
        y.extend([0., 1.]);
    }
    (x, y)
}

#[test]
fn complete_and_quasi_multivariate_separation_are_refused_across_units() {
    for quasi in [false, true] {
        let (x, y) = separated(quasi);
        for units in [[1., 1.], [1e12, 1e-12], [1e-12, -1e12]] {
            let scaled: Vec<Vec<_>> = x
                .iter()
                .map(|row| row.iter().zip(units).map(|(v, s)| v * s).collect())
                .collect();
            assert!(matches!(
                fit(&scaled, &y, Settings::default()),
                Err(FitError::Separated(_))
            ));
        }
    }
}

#[test]
fn crossing_classes_and_repeated_opposite_labels_are_accepted() {
    let x = vec![
        vec![-1., -1.],
        vec![-1., 1.],
        vec![1., -1.],
        vec![1., 1.],
        vec![0., 0.],
        vec![0., 0.],
    ];
    let y = vec![0., 1., 1., 0., 0., 1.];
    let result = fit(&x, &y, Settings::default()).unwrap();
    assert!(result.probabilities.iter().all(|p| (p - 0.5).abs() < 1e-8));
}

#[test]
fn invalid_inputs_and_rank_deficiency_have_typed_errors() {
    let (x, y, _) = reference();
    let duplicate: Vec<Vec<_>> = x.iter().map(|row| vec![row[0], 2. * row[0]]).collect();
    assert_eq!(
        fit(&duplicate, &y, Settings::default()).unwrap_err(),
        FitError::Detection(DetectionError::RankDeficient)
    );
    for label in [-1., 0.5, 2.] {
        let mut invalid = y.clone();
        invalid[0] = label;
        assert_eq!(
            fit(&x, &invalid, Settings::default()).unwrap_err(),
            FitError::InvalidLabels
        );
    }
    assert_eq!(
        fit(&x, &vec![1.; y.len()], Settings::default()).unwrap_err(),
        FitError::InvalidLabels
    );
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut invalid = x.clone();
        invalid[0][0] = value;
        assert_eq!(
            fit(&invalid, &y, Settings::default()).unwrap_err(),
            FitError::NonFiniteInput
        );
    }
    assert_eq!(
        fit(&[], &[], Settings::default()).unwrap_err(),
        FitError::InvalidShape
    );
    assert_eq!(
        fit(&x, &y[1..], Settings::default()).unwrap_err(),
        FitError::InvalidShape
    );
    let mut ragged = x.clone();
    ragged[0].pop();
    assert_eq!(
        fit(&ragged, &y, Settings::default()).unwrap_err(),
        FitError::InvalidShape
    );
}

#[test]
fn iteration_and_detection_limits_never_report_success() {
    let (x, y, _) = reference();
    assert_eq!(
        fit(
            &x,
            &y,
            Settings {
                max_iterations: 1,
                ..Settings::default()
            }
        )
        .unwrap_err(),
        FitError::NonConvergence
    );
    assert_eq!(
        fit(
            &x,
            &y,
            Settings {
                detection_budget: Duration::ZERO,
                ..Settings::default()
            }
        )
        .unwrap_err(),
        FitError::Detection(DetectionError::SolverLimit)
    );
    assert_eq!(
        fit(
            &x,
            &y,
            Settings {
                confidence_level: 1.,
                ..Settings::default()
            }
        )
        .unwrap_err(),
        FitError::InvalidSettings
    );
}

#[test]
fn confidence_levels_match_statsmodels_quantiles() {
    let (x, y, _) = reference();
    let cases: Value = serde_json::from_str(include_str!("../reference.json")).unwrap();
    for level in [0.8, 0.99] {
        let result = fit(
            &x,
            &y,
            Settings {
                confidence_level: level,
                ..Settings::default()
            },
        )
        .unwrap();
        for (j, term) in result.coefficients.iter().enumerate() {
            for k in 0..2 {
                close(
                    term.confidence_interval[k],
                    cases["logistic"]["intervals_by_level"][level.to_string()][j][k]
                        .as_f64()
                        .unwrap(),
                );
            }
        }
    }
}

#[test]
fn independent_highs_reference_classifications_match() {
    let fixtures: Value =
        serde_json::from_str(include_str!("../separation_reference.json")).unwrap();
    for (index, case) in fixtures["cases"].as_array().unwrap().iter().enumerate() {
        let x: Vec<Vec<f64>> = serde_json::from_value(case["x"].clone()).unwrap();
        let y: Vec<f64> = serde_json::from_value(case["y"].clone()).unwrap();
        let result = fit(&x, &y, Settings::default());
        if case["separated"].as_bool().unwrap() {
            assert!(
                matches!(result, Err(FitError::Separated(_))),
                "case {index}: {result:?}"
            );
        } else {
            assert!(result.is_ok(), "case {index}: {result:?}");
        }
    }
}

#[test]
fn upstream_separation_reproducer_is_rejected_by_adapter() {
    let reference: Value = serde_json::from_str(include_str!("../reference.json")).unwrap();
    let x: Vec<Vec<f64>> = serde_json::from_value(reference["separation"]["x"].clone()).unwrap();
    let y: Vec<f64> = serde_json::from_value(reference["separation"]["y"].clone()).unwrap();
    assert!(matches!(
        fit(&x, &y, Settings::default()),
        Err(FitError::Separated(_))
    ));
}
