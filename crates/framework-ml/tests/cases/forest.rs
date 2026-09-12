use framework_ml::*;
use smartcore::ensemble::random_forest_classifier::{
    RandomForestClassifier, RandomForestClassifierParameters,
};
use smartcore::ensemble::random_forest_regressor::{
    RandomForestRegressor, RandomForestRegressorParameters,
};
use smartcore::linalg::basic::matrix::DenseMatrix;

fn request(method: Method) -> FitRequest {
    FitRequest {
        method,
        feature_names: vec!["x".into(), "z".into()],
        target_name: "y".into(),
        confidence_level: 0.95,
        covariance: CovarianceMethod::Classical,
        max_iterations: 200,
        forest: ForestSettings {
            trees: 31,
            max_depth: 7,
            min_samples_leaf: 1,
            max_features: Some(2),
            seed: 41,
        },
        xgboost: Default::default(),
    }
}
fn data(classification: bool) -> NumericDataset {
    let rows: Vec<Vec<f64>> = (0..120)
        .map(|i| vec![(i % 20) as f64 / 5. - 2., (i / 20) as f64 / 2. - 1.])
        .collect();
    let targets = rows
        .iter()
        .map(|row| {
            if classification {
                if row[0] < -0.5 {
                    0.
                } else if row[0] > 0.5 {
                    2.
                } else {
                    1.
                }
            } else {
                row[0].powi(2) + 0.8 * row[1]
            }
        })
        .collect();
    NumericDataset { rows, targets }
}

#[test]
fn forest_regressor_matches_training_backend_after_reload() {
    let request = request(Method::RandomForestRegressor);
    let data = data(false);
    let model = fit(&request, &data).unwrap();
    let repeated = fit(&request, &data).unwrap();
    assert_eq!(model, repeated);
    let matrix = DenseMatrix::from_2d_vec(&data.rows).unwrap();
    let direct = RandomForestRegressor::fit(
        &matrix,
        &data.targets,
        RandomForestRegressorParameters {
            max_depth: Some(7),
            min_samples_leaf: 1,
            n_trees: 31,
            m: Some(2),
            seed: 41,
            ..Default::default()
        },
    )
    .unwrap();
    let expected: Vec<f64> = direct.predict(&matrix).unwrap();
    let restored: FittedModel =
        serde_json::from_str(&serde_json::to_string(&model).unwrap()).unwrap();
    for (a, b) in predict(&restored, &data.rows)
        .unwrap()
        .values
        .iter()
        .zip(expected)
    {
        assert!((a - b).abs() < 1e-10);
    }
    assert!(restored.summary.coefficients.is_empty());
    assert!(restored.summary.covariance.is_none());
    assert!(
        restored.summary.training_metrics.rmse.unwrap()
            < restored.summary.training_metrics.baseline_rmse.unwrap() * 0.3
    );
}

#[test]
fn forest_classifier_vote_labels_match_backend_and_saved_baseline() {
    let request = request(Method::RandomForestClassifier);
    let data = data(true);
    let model = fit(&request, &data).unwrap();
    let matrix = DenseMatrix::from_2d_vec(&data.rows).unwrap();
    let labels: Vec<u32> = data.targets.iter().map(|v| *v as u32).collect();
    let direct = RandomForestClassifier::fit(
        &matrix,
        &labels,
        RandomForestClassifierParameters {
            max_depth: Some(7),
            min_samples_leaf: 1,
            n_trees: 31,
            m: Some(2),
            seed: 41,
            ..Default::default()
        },
    )
    .unwrap();
    let output = predict(&model, &data.rows).unwrap();
    let expected: Vec<u32> = direct.predict(&matrix).unwrap();
    assert_eq!(
        output.values,
        expected.iter().map(|v| f64::from(*v)).collect::<Vec<_>>()
    );
    for row in output.probabilities.unwrap() {
        assert!((row.iter().sum::<f64>() - 1.).abs() < 1e-12);
        assert!(row.iter().all(|v| (0.0..=1.0).contains(v)));
    }
    let assess = NumericDataset {
        rows: data.rows.clone(),
        targets: vec![1.; data.rows.len()],
    };
    let metrics = evaluate(&model, &assess).unwrap();
    let baseline = model.training_class_probabilities.as_ref().unwrap();
    assert!((metrics.baseline_log_loss.unwrap() + baseline[1].ln()).abs() < 1e-12);
    let restored: FittedModel =
        serde_json::from_str(&serde_json::to_string(&model).unwrap()).unwrap();
    assert_eq!(
        predict(&restored, &data.rows).unwrap(),
        predict(&model, &data.rows).unwrap()
    );
}

#[test]
fn forest_saved_cycles_and_invalid_settings_are_refused() {
    let mut request = request(Method::RandomForestRegressor);
    let data = data(false);
    let model = fit(&request, &data).unwrap();
    let mut stored = serde_json::to_value(model).unwrap();
    stored["payload"]["model"]["trees"][0][0] =
        serde_json::json!({"kind":"split","feature":0,"threshold":0.0,"left":0,"right":1});
    let malformed: FittedModel = serde_json::from_value(stored).unwrap();
    assert!(validate_fitted(&malformed).is_err());
    assert!(predict(&malformed, &data.rows).is_err());
    request.forest.trees = 0;
    assert!(matches!(
        fit(&request, &data),
        Err(MlError::InvalidInput(_))
    ));
}
