use framework_ml::{
    CovarianceMethod, FitRequest, Method, MlError, ModelPayload, NumericDataset,
    XgboostSettings, evaluate, fit, predict,
};
use framework_ml::xgboost::Objective;

fn request(objective: Objective) -> FitRequest {
    FitRequest {
        method: Method::Xgboost,
        feature_names: vec!["signal".into(), "noise".into()],
        target_name: "outcome".into(),
        confidence_level: 0.95,
        covariance: CovarianceMethod::Classical,
        max_iterations: 100,
        forest: Default::default(),
        xgboost: XgboostSettings {
            objective,
            rounds: 24,
            max_depth: 3,
            learning_rate: 0.3,
            seed: 42,
            ..Default::default()
        },
    }
}

fn classification_data(classes: usize) -> NumericDataset {
    let targets: Vec<f64> = (0..192).map(|row| (row % classes) as f64).collect();
    NumericDataset {
        rows: targets
            .iter()
            .enumerate()
            .map(|(row, target)| vec![*target, (row % 11) as f64])
            .collect(),
        targets,
    }
}

#[test]
fn trains_typed_regression_model_and_round_trips_predictions() {
    let data = NumericDataset {
        rows: (0..128)
            .map(|row| vec![row as f64 / 10.0, (row % 7) as f64])
            .collect(),
        targets: (0..128).map(|row| 2.0 * row as f64 / 10.0 - 3.0).collect(),
    };
    let model = fit(&request(Objective::Regression), &data).unwrap();
    let ModelPayload::Xgboost { model: booster } = &model.payload else {
        panic!("expected typed XGBoost payload")
    };
    assert_eq!(booster.objective(), Objective::Regression);
    assert_eq!(booster.feature_names(), ["signal", "noise"]);
    assert!(model.summary.training_metrics.rmse.unwrap() < 0.5);
    let before = predict(&model, &data.rows).unwrap();
    let restored = serde_json::from_str(&serde_json::to_string(&model).unwrap()).unwrap();
    assert_eq!(before, predict(&restored, &data.rows).unwrap());
}

#[test]
fn trains_binary_and_multiclass_probabilities_with_shared_evaluation() {
    for (objective, classes) in [(Objective::Binary, 2), (Objective::Multiclass, 3)] {
        let training = classification_data(classes);
        let model = fit(&request(objective), &training).unwrap();
        let output = predict(&model, &training.rows).unwrap();
        let probabilities = output.probabilities.unwrap();
        assert_eq!(probabilities[0].len(), classes);
        assert!(probabilities.iter().all(|row| {
            row.iter().all(|value| (0.0..=1.0).contains(value))
                && (row.iter().sum::<f64>() - 1.0).abs() < 1e-5
        }));
        assert!(evaluate(&model, &training).unwrap().accuracy.unwrap() > 0.95);
        assert_eq!(model.training_class_probabilities.as_ref().unwrap().len(), classes);
    }
}

#[test]
fn rejects_invalid_class_shapes_and_bounded_settings() {
    let mut data = classification_data(3);
    assert!(matches!(
        fit(&request(Objective::Binary), &data),
        Err(MlError::InvalidInput(_))
    ));
    data.targets[0] = 4.0;
    assert!(matches!(
        fit(&request(Objective::Multiclass), &data),
        Err(MlError::InvalidInput(_))
    ));
    let mut request = request(Objective::Regression);
    request.xgboost.rounds = 0;
    assert!(matches!(
        fit(&request, &classification_data(2)),
        Err(MlError::InvalidInput(_))
    ));
}
