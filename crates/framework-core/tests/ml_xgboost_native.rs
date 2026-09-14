use framework_core::*;
use framework_ml::{Method, ModelPayload, xgboost::Objective};

fn native_spec(objective: Objective) -> (Store, ModelSpec) {
    let (store, mut spec) = super::ml_models::fixture();
    spec.method = Method::Xgboost;
    spec.xgboost.objective = objective;
    spec.xgboost.rounds = 24;
    spec.xgboost.max_depth = 3;
    spec.xgboost.learning_rate = 0.3;
    spec.xgboost.seed = spec.seed;
    if objective != Objective::Regression {
        let class = store
            .document()
            .frame(&spec.source_frame_id)
            .unwrap()
            .columns[2]
            .id
            .clone();
        spec.target_column_id = Some(class);
    }
    (store, spec)
}

#[test]
fn native_xgboost_regression_survives_store_history_and_reopen() {
    let (mut store, spec) = native_spec(Objective::Regression);
    let model_id = super::ml_models::fit(&mut store, &spec);
    let prediction_id = super::ml_models::predictions(&mut store, &model_id, &spec);
    let fitted = store
        .document()
        .model(&model_id)
        .unwrap()
        .fitted
        .as_ref()
        .unwrap();
    assert_eq!(fitted.result.summary.observations, Some(24));
    assert_eq!(fitted.evaluation_metrics.as_ref().unwrap().rows, 6);
    assert!(fitted.evaluation_metrics.as_ref().unwrap().rmse.is_some());
    assert!(matches!(
        &fitted.result.payload,
        ModelPayload::Xgboost { model } if model.objective() == Objective::Regression
    ));
    let fitted_id = fitted.id.clone();
    let page = store.get_frame_page(&prediction_id, 0, 100).unwrap();
    assert_eq!(page.total_rows, 30);
    // The three source columns carried through, then the prediction.
    assert_eq!(page.columns.len(), 4);

    let directory =
        std::env::temp_dir().join(format!("framework-native-xgboost-{}", framework_core::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("regression.fw");
    store.save(&path).unwrap();
    let reopened = Store::load(&path).unwrap();
    assert_eq!(
        reopened
            .get_frame_page(&prediction_id, 0, 100)
            .unwrap()
            .rows,
        page.rows
    );

    let source = store.document().frame(&spec.source_frame_id).unwrap();
    store
        .apply(Operation::SetCell {
            frame_id: spec.source_frame_id.clone(),
            row_id: source.rows[0].id.clone(),
            column_id: spec.feature_column_ids[0].clone(),
            raw: "100".into(),
        })
        .unwrap();
    assert!(store.view().computed_models[&model_id].stale);
    assert_eq!(
        store
            .document()
            .model(&model_id)
            .unwrap()
            .fitted
            .as_ref()
            .unwrap()
            .id,
        fitted_id
    );
    store.save(&path).unwrap();
    let stale_reopen = Store::load(&path).unwrap();
    assert!(stale_reopen.view().computed_models[&model_id].stale);
    assert_eq!(
        stale_reopen
            .document()
            .model(&model_id)
            .unwrap()
            .fitted
            .as_ref()
            .unwrap()
            .id,
        fitted_id
    );
    store.undo();
    assert!(!store.view().computed_models[&model_id].stale);
    assert_eq!(
        store.get_frame_page(&prediction_id, 0, 100).unwrap().rows,
        page.rows
    );
}

#[test]
fn native_xgboost_binary_fit_exposes_probabilities_and_holdout_metrics() {
    let (mut store, spec) = native_spec(Objective::Binary);
    let model_id = super::ml_models::fit(&mut store, &spec);
    let prediction_id = super::ml_models::predictions(&mut store, &model_id, &spec);
    let fitted = store
        .document()
        .model(&model_id)
        .unwrap()
        .fitted
        .as_ref()
        .unwrap();
    assert!(matches!(
        &fitted.result.payload,
        ModelPayload::Xgboost { model } if model.objective() == Objective::Binary
    ));
    let evaluation = fitted.evaluation_metrics.as_ref().unwrap();
    assert_eq!(evaluation.rows, 6);
    assert!(evaluation.accuracy.is_some());
    assert!(evaluation.log_loss.is_some());
    assert_eq!(
        fitted
            .result
            .training_class_probabilities
            .as_deref()
            .unwrap()
            .len(),
        2
    );
    let page = store.get_frame_page(&prediction_id, 0, 100).unwrap();
    assert_eq!(page.total_rows, 30);
    // The three source columns carried through, then class and two
    // probabilities.
    assert_eq!(page.columns.len(), 6);
    for row in page.rows {
        let predicted: usize = row[3].parse().unwrap();
        let p0: f64 = row[4].parse().unwrap();
        let p1: f64 = row[5].parse().unwrap();
        assert!(predicted < 2);
        assert!((p0 + p1 - 1.0).abs() < 0.001);
    }
}
