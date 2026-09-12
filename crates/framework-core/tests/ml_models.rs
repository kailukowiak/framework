use framework_core::*;
use framework_ml::{CovarianceMethod, Method};

pub(super) fn fixture() -> (Store, ModelSpec) {
    let mut store = Store::new(Document::blank("Models"));
    let text = std::iter::once("Input\tOutcome\tClass".to_string())
        .chain((0..30).map(|i| {
            format!(
                "{}\t{}\t{}",
                i,
                2.0 + 3.0 * i as f64 + (i % 3) as f64 * 0.2,
                usize::from(i % 4 == 0 || i > 20)
            )
        }))
        .collect::<Vec<_>>()
        .join("\n");
    store
        .apply(Operation::AddFrameFromPastedText {
            name: "Observations".into(),
            text,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = store
        .document()
        .objects
        .iter()
        .find_map(|o| match o {
            DataObject::Frame(f) => Some(f),
            _ => None,
        })
        .unwrap();
    let spec = ModelSpec {
        source_frame_id: frame.id.clone(),
        target_column_id: Some(frame.columns[1].id.clone()),
        feature_column_ids: vec![frame.columns[0].id.clone()],
        method: Method::Ols,
        covariance: CovarianceMethod::Hc3,
        confidence_level: 0.95,
        holdout_fraction: 0.2,
        seed: 42,
        forest: Default::default(),
    };
    (store, spec)
}
fn model_id(store: &Store) -> Id {
    store
        .document()
        .objects
        .iter()
        .find_map(|o| match o {
            DataObject::Model(m) => Some(m.id.clone()),
            _ => None,
        })
        .unwrap()
}
pub(super) fn fit(store: &mut Store, spec: &ModelSpec) -> Id {
    store
        .apply(Operation::AddModel {
            name: "Demand".into(),
            spec: spec.clone(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let id = model_id(store);
    store
        .apply(Operation::FitModel {
            model_id: id.clone(),
        })
        .unwrap();
    id
}
pub(super) fn predictions(store: &mut Store, model_id: &str, spec: &ModelSpec) -> Id {
    store
        .apply(Operation::AddModelPredictions {
            model_id: model_id.into(),
            source_frame_id: spec.source_frame_id.clone(),
            feature_column_ids: spec.feature_column_ids.clone(),
            name: "Predictions".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .document()
        .objects
        .iter()
        .find(|o| o.name() == "Predictions")
        .unwrap()
        .id()
        .into()
}

#[test]
fn ml_fit_holdout_live_predictions_history_and_persistence() {
    let (mut store, spec) = fixture();
    let model_id = fit(&mut store, &spec);
    let trained = store
        .document()
        .model(&model_id)
        .unwrap()
        .fitted
        .as_ref()
        .unwrap()
        .clone();
    assert_eq!(trained.result.summary.observations, Some(24));
    assert_eq!(trained.evaluation_metrics.as_ref().unwrap().rows, 6);
    assert_eq!(trained.split.as_ref().unwrap().training_rows.len(), 24);
    assert!(!store.view().computed_models[&model_id].stale);
    let prediction_id = predictions(&mut store, &model_id, &spec);
    let frame = store.document().frame(&prediction_id).unwrap();
    assert!(frame.rows.is_empty());
    assert!(!frame.owns_its_rows());
    let output_column = frame.columns[0].id.clone();
    let output_name = frame.columns[0].name.clone();
    store
        .apply(Operation::AddBlock {
            name: "Totals".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let block_id = store
        .document()
        .objects
        .iter()
        .find(|o| o.name() == "Totals")
        .unwrap()
        .id()
        .to_owned();
    store
        .apply(Operation::SetBlockSource {
            block_id: block_id.clone(),
            source: format!("total = `Predictions`.`{output_name}`.sum()"),
            editing: None,
        })
        .unwrap();
    let total = store.view().computed_blocks[&block_id].lines[0]
        .cell
        .value
        .unwrap();
    let source = store.document().frame(&spec.source_frame_id).unwrap();
    let row_id = source.rows[0].id.clone();
    store
        .apply(Operation::SetCell {
            frame_id: spec.source_frame_id.clone(),
            row_id,
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
        trained.id
    );
    let new_total = store.view().computed_blocks[&block_id].lines[0]
        .cell
        .value
        .unwrap();
    assert!(
        (new_total - total).abs() > 100.0,
        "Scratchwork reads live model predictions"
    );
    store.undo();
    assert!(!store.view().computed_models[&model_id].stale);
    assert!(
        (store.view().computed_blocks[&block_id].lines[0]
            .cell
            .value
            .unwrap()
            - total)
            .abs()
            < 1e-9
    );
    check_refit_and_reopen(
        &mut store,
        &model_id,
        &prediction_id,
        &output_column,
        &trained.id,
        &block_id,
        total,
    );
}

#[test]
fn ml_failed_retrain_preserves_fit_and_recipe_undo_restores_freshness() {
    let (mut store, spec) = fixture();
    let id = fit(&mut store, &spec);
    let previous = store.document().model(&id).unwrap().fitted.clone();
    let mut invalid = spec.clone();
    invalid.method = Method::Logistic; // continuous labels cannot be logistic targets
    invalid.covariance = CovarianceMethod::Classical;
    store
        .apply(Operation::SetModelSpec {
            model_id: id.clone(),
            spec: invalid,
        })
        .unwrap();
    assert!(store.view().computed_models[&id].stale);
    let revision = store.document().revision;
    assert!(
        store
            .apply(Operation::FitModel {
                model_id: id.clone()
            })
            .is_err()
    );
    assert_eq!(store.document().revision, revision);
    assert_eq!(store.document().model(&id).unwrap().fitted, previous);
    store.undo();
    assert!(!store.view().computed_models[&id].stale);
}

#[test]
fn ml_dependencies_protect_ids_and_renames_keep_fit_fresh() {
    let (mut store, spec) = fixture();
    let id = fit(&mut store, &spec);
    let output = predictions(&mut store, &id, &spec);
    assert!(
        store
            .apply(Operation::DeleteObject {
                object_id: id.clone()
            })
            .is_err()
    );
    assert!(
        store
            .apply(Operation::DeleteObject {
                object_id: spec.source_frame_id.clone()
            })
            .is_err()
    );
    let saved_fit = store.document().model(&id).unwrap().fitted.clone();
    store
        .apply(Operation::DeleteColumn {
            frame_id: spec.source_frame_id.clone(),
            column_id: spec.feature_column_ids[0].clone(),
        })
        .unwrap();
    let broken = store.view();
    assert!(broken.computed_models[&id].stale);
    assert!(broken.computed_models[&id].error.is_some());
    assert!(store.get_frame_page(&output, 0, 10).is_err());
    assert_eq!(store.document().model(&id).unwrap().fitted, saved_fit);
    let directory = std::env::temp_dir().join(format!("framework-model-{}", framework_core::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("missing-input.fw");
    store.save(&path).unwrap();
    let reopened = Store::load(&path).unwrap();
    assert!(reopened.view().computed_models[&id].error.is_some());
    assert_eq!(reopened.document().model(&id).unwrap().fitted, saved_fit);
    assert!(
        store
            .apply(Operation::FitModel {
                model_id: id.clone()
            })
            .is_err()
    );
    assert_eq!(store.document().model(&id).unwrap().fitted, saved_fit);
    store.undo();
    assert!(!store.view().computed_models[&id].stale);
    assert!(store.get_frame_page(&output, 0, 10).is_ok());
    store
        .apply(Operation::RenameColumn {
            frame_id: spec.source_frame_id.clone(),
            column_id: spec.feature_column_ids[0].clone(),
            name: "Renamed input".into(),
        })
        .unwrap();
    assert!(!store.view().computed_models[&id].stale);
    assert!(!store.view().computed_frames[&output].rows.is_empty());
    assert!(
        store.view().computed_frames[&output]
            .rows
            .values()
            .flat_map(|row| row.values())
            .all(|cell| cell.error.is_none())
    );
    store.set_safe_mode(true);
    assert!(store.apply(Operation::FitModel { model_id: id }).is_err());
    assert!(store.view().computed_models.is_empty());
}

#[test]
fn ml_logistic_and_summary_exports_are_normal_referenceable_frames() {
    let (mut store, mut spec) = fixture();
    spec.method = Method::Logistic;
    spec.covariance = CovarianceMethod::Classical;
    spec.target_column_id = Some(
        store
            .document()
            .frame(&spec.source_frame_id)
            .unwrap()
            .columns[2]
            .id
            .clone(),
    );
    let id = fit(&mut store, &spec);
    let prediction = predictions(&mut store, &id, &spec);
    assert_eq!(
        store.document().frame(&prediction).unwrap().columns.len(),
        3
    );
    for kind in [
        ModelSummaryKind::Coefficients,
        ModelSummaryKind::TrainingMetrics,
        ModelSummaryKind::EvaluationMetrics,
    ] {
        store
            .apply(Operation::AddModelSummary {
                model_id: id.clone(),
                kind,
                name: "Summary".into(),
                x: 0.0,
                y: 0.0,
            })
            .unwrap();
    }
    let summary = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == "Summary" => Some(frame),
            _ => None,
        })
        .unwrap();
    assert_eq!(summary.rows.len(), 2);
    assert!(
        summary
            .comment
            .as_ref()
            .unwrap()
            .contains("fitted revision")
    );
}

#[test]
fn ml_imported_xgboost_predicts_live_without_training_staleness() {
    let mut store = Store::new(Document::blank("Imported model"));
    store
        .apply(Operation::AddFrameFromPastedText {
            name: "Features".into(),
            text: "a\tb\tc\n0\t1\t2\n2\t1\t0".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(f) => Some(f.clone()),
            _ => None,
        })
        .unwrap();
    store
        .apply(Operation::ImportModel {
            name: "Imported".into(),
            source_frame_id: frame.id.clone(),
            feature_column_ids: frame.columns.iter().map(|c| c.id.clone()).collect(),
            json: include_str!("../../framework-ml/tests/fixtures/xgboost-binary.model.json")
                .into(),
            iteration_range: Some(framework_ml::IterationRange { start: 0, end: 1 }),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let id = model_id(&store);
    let fit = store
        .document()
        .model(&id)
        .unwrap()
        .fitted
        .as_ref()
        .unwrap()
        .clone();
    assert!(fit.training_fingerprint.is_none());
    let input = ModelSpec {
        source_frame_id: frame.id.clone(),
        feature_column_ids: frame.columns.iter().map(|c| c.id.clone()).collect(),
        target_column_id: None,
        method: Method::Xgboost,
        covariance: CovarianceMethod::Classical,
        confidence_level: 0.95,
        holdout_fraction: 0.0,
        seed: 42,
        forest: Default::default(),
    };
    let predictions = predictions(&mut store, &id, &input);
    let before = store.view().computed_frames[&predictions]
        .fingerprint
        .clone();
    store
        .apply(Operation::SetCell {
            frame_id: frame.id,
            row_id: frame.rows[0].id.clone(),
            column_id: frame.columns[0].id.clone(),
            raw: "10".into(),
        })
        .unwrap();
    assert_ne!(
        store.view().computed_frames[&predictions].fingerprint,
        before
    );
    assert!(!store.view().computed_models[&id].stale);
    assert_eq!(
        store
            .document()
            .model(&id)
            .unwrap()
            .fitted
            .as_ref()
            .unwrap()
            .id,
        fit.id
    );
    assert!(store.apply(Operation::FitModel { model_id: id }).is_err());
}

fn check_refit_and_reopen(
    store: &mut Store,
    model_id: &str,
    prediction_id: &str,
    output_column: &str,
    trained_id: &str,
    block_id: &str,
    total: f64,
) {
    store
        .apply(Operation::FitModel {
            model_id: model_id.to_string(),
        })
        .unwrap();
    assert_ne!(
        store
            .document()
            .model(model_id)
            .unwrap()
            .fitted
            .as_ref()
            .unwrap()
            .id,
        trained_id
    );
    assert_eq!(
        store.document().frame(prediction_id).unwrap().columns[0].id,
        output_column
    );
    store.undo();
    assert_eq!(
        store
            .document()
            .model(model_id)
            .unwrap()
            .fitted
            .as_ref()
            .unwrap()
            .id,
        trained_id
    );
    let directory = std::env::temp_dir().join(format!("framework-ml-{}", id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("models.fw");
    store.save(&path).unwrap();
    let loaded = Store::load(&path).unwrap();
    assert_eq!(
        loaded
            .document()
            .model(model_id)
            .unwrap()
            .fitted
            .as_ref()
            .unwrap()
            .id,
        trained_id
    );
    assert!(
        (loaded.view().computed_blocks[block_id].lines[0]
            .cell
            .value
            .unwrap()
            - total)
            .abs()
            < 1e-9
    );
    std::fs::remove_dir_all(directory).unwrap();
}
