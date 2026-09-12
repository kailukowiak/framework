use framework_ml::*;
use serde_json::Value;

fn fixture(name: &str) -> (FitRequest, NumericDataset, Value) {
    let all: Value = serde_json::from_str(include_str!("../fixtures/statistics.json")).unwrap();
    let case = all[name].clone();
    let data = NumericDataset {
        rows: serde_json::from_value(case["x"].clone()).unwrap(),
        targets: serde_json::from_value(case["y"].clone()).unwrap(),
    };
    let request = FitRequest {
        method: if name == "logistic" {
            Method::Logistic
        } else {
            Method::Ols
        },
        feature_names: (0..data.rows[0].len())
            .map(|i| format!("feature {i}"))
            .collect(),
        target_name: "Outcome".into(),
        confidence_level: 0.95,
        covariance: CovarianceMethod::Classical,
        max_iterations: 200,
        forest: Default::default(),
        xgboost: Default::default(),
    };
    (request, data, case)
}
fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-8 + 1e-6 * expected.abs(),
        "{actual} != {expected}"
    );
}

#[test]
fn ols_classical_hc3_and_logistic_inference_match_statsmodels() {
    for (name, covariance, key) in [
        ("ols", CovarianceMethod::Classical, "classical"),
        ("ols", CovarianceMethod::Hc3, "hc3"),
        ("logistic", CovarianceMethod::Classical, "classical"),
    ] {
        let (mut request, data, case) = fixture(name);
        request.covariance = covariance;
        let model = fit(&request, &data).unwrap();
        let expected = &case[key];
        for (i, term) in model.summary.coefficients.iter().enumerate() {
            for (actual, field) in [
                (term.estimate, "coefficients"),
                (term.standard_error, "se"),
                (term.statistic, "statistic"),
                (term.p_value, "p"),
            ] {
                close(actual.unwrap(), expected[field][i].as_f64().unwrap());
            }
            close(
                term.confidence_lower.unwrap(),
                expected["ci"][i][0].as_f64().unwrap(),
            );
            close(
                term.confidence_upper.unwrap(),
                expected["ci"][i][1].as_f64().unwrap(),
            );
        }
        let serialized = serde_json::to_string(&model).unwrap();
        assert!(!serialized.contains("NaN"));
        let loaded: FittedModel = serde_json::from_str(&serialized).unwrap();
        assert_eq!(model, loaded);
        let output = predict(&loaded, &data.rows).unwrap();
        for i in 0..data.rows.len() {
            let value = output
                .probabilities
                .as_ref()
                .map_or(output.values[i], |p| p[i][1]);
            close(value, expected["predictions"][i].as_f64().unwrap());
        }
    }
}

#[test]
fn held_out_baseline_comes_from_training_not_assessment_labels() {
    let (request, data, _) = fixture("ols");
    let model = fit(&request, &data).unwrap();
    let shifted = NumericDataset {
        rows: data.rows.clone(),
        targets: vec![100.; data.targets.len()],
    };
    let evaluation = evaluate(&model, &shifted).unwrap();
    close(
        evaluation.baseline_rmse.unwrap(),
        (100. - model.training_baseline.unwrap()).abs(),
    );
    assert!(evaluation.baseline_rmse.unwrap() > 90.);
    let (request, data, _) = fixture("logistic");
    let model = fit(&request, &data).unwrap();
    let shifted = NumericDataset {
        rows: data.rows.clone(),
        targets: vec![0.; data.targets.len()],
    };
    let evaluation = evaluate(&model, &shifted).unwrap();
    close(
        evaluation.baseline_log_loss.unwrap(),
        -(1. - model.training_baseline.unwrap()).ln(),
    );
}

#[test]
fn logistic_rejects_separation_and_invalid_inputs() {
    let (mut request, mut data, _) = fixture("logistic");
    let all: Value = serde_json::from_str(include_str!("../fixtures/statistics.json")).unwrap();
    data.rows = serde_json::from_value(all["separation"]["x"].clone()).unwrap();
    data.targets = serde_json::from_value(all["separation"]["y"].clone()).unwrap();
    request.feature_names = vec!["x".into()];
    assert_eq!(fit(&request, &data).unwrap_err(), MlError::Separation);
    let (mut request, mut data, _) = fixture("logistic");
    request.max_iterations = 1;
    assert_eq!(fit(&request, &data).unwrap_err(), MlError::NonConvergence);
    request.max_iterations = 200;
    data.targets[0] = 2.;
    assert!(matches!(
        fit(&request, &data),
        Err(MlError::InvalidInput(_))
    ));
    data.targets[0] = 0.;
    data.rows[0][0] = f64::NAN;
    assert!(matches!(
        fit(&request, &data),
        Err(MlError::InvalidInput(_))
    ));
}

#[test]
fn logistic_fits_more_than_256_rows_with_reference_parity() {
    let (request, data, _) = fixture("logistic");
    let original = fit(&request, &data).unwrap();
    let repeated = NumericDataset {
        rows: data
            .rows
            .iter()
            .cycle()
            .take(data.rows.len() * 20)
            .cloned()
            .collect(),
        targets: data.targets.repeat(20),
    };
    let started = std::time::Instant::now();
    let model = fit(&request, &repeated).unwrap();
    println!(
        "600-row logistic fit including exact separation: {:?}",
        started.elapsed()
    );
    for (a, b) in original
        .summary
        .coefficients
        .iter()
        .zip(&model.summary.coefficients)
    {
        close(a.estimate.unwrap(), b.estimate.unwrap());
        close(
            a.standard_error.unwrap() / 20_f64.sqrt(),
            b.standard_error.unwrap(),
        );
    }
}

#[test]
fn aliased_ols_serializes_unavailable_statistics_and_valid_predictions() {
    let (request, data, case) = fixture("rank_deficient");
    let model = fit(&request, &data).unwrap();
    assert!(
        model
            .summary
            .coefficients
            .iter()
            .any(|c| c.estimate.is_none())
    );
    let loaded: FittedModel =
        serde_json::from_str(&serde_json::to_string(&model).unwrap()).unwrap();
    for (i, value) in predict(&loaded, &data.rows)
        .unwrap()
        .values
        .iter()
        .enumerate()
    {
        close(*value, case["predictions"][i].as_f64().unwrap());
    }
}

#[test]
fn multivariate_quasi_separation_and_changed_units_are_rejected() {
    let (request, _, _) = fixture("logistic");
    let source = [
        [2., -1.],
        [-1., 2.],
        [1., 1.],
        [-2., 1.],
        [1., -2.],
        [-1., -1.],
        [0., 0.],
        [0., 0.],
    ];
    for scales in [[1., 1.], [1e12, 1e-12], [-1e-12, 1e12]] {
        let data = NumericDataset {
            rows: source
                .iter()
                .map(|row| row.iter().zip(scales).map(|(v, s)| v * s).collect())
                .collect(),
            targets: vec![1., 1., 1., 0., 0., 0., 0., 1.],
        };
        assert_eq!(fit(&request, &data).unwrap_err(), MlError::Separation);
    }
}

#[test]
fn independent_highs_cases_remain_correct_in_production_adapter() {
    let (request, _, _) = fixture("logistic");
    let cases: Value = serde_json::from_str(include_str!("../fixtures/separation.json")).unwrap();
    for case in cases["cases"].as_array().unwrap() {
        let data = NumericDataset {
            rows: serde_json::from_value(case["x"].clone()).unwrap(),
            targets: serde_json::from_value(case["y"].clone()).unwrap(),
        };
        if case["separated"].as_bool().unwrap() {
            assert_eq!(fit(&request, &data).unwrap_err(), MlError::Separation);
        } else {
            assert!(fit(&request, &data).is_ok());
        }
    }
}
