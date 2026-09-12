//! Generate the Iris multiclass XGBoost lesson and completed answer key through public operations.
use framework_core::*;
use framework_ml::{CovarianceMethod, Method, XgboostSettings, xgboost::Objective};
use std::path::PathBuf;

fn object(store: &Store, name: &str) -> DataObject {
    store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == name)
        .unwrap()
        .clone()
}
fn frame(store: &Store, name: &str) -> FrameObject {
    match object(store, name) {
        DataObject::Frame(frame) => frame,
        _ => unreachable!(),
    }
}
fn resize(store: &mut Store, name: &str, width: f64, height: f64) -> Result<(), CoreError> {
    let object_id = object(store, name).id().to_string();
    let view_id = store
        .document()
        .views
        .iter()
        .find(|view| view.object_id == object_id)
        .unwrap()
        .id
        .clone();
    store.apply(Operation::ResizeView {
        view_id,
        width,
        height,
    })?;
    Ok(())
}
fn prepare() -> Result<Store, CoreError> {
    let mut store = Store::new_tutorial(Document::blank("Classify Iris species with XGBoost"));
    store.apply(Operation::AddText { x: 30.0, y: 60.0 })?;
    let guide_id = object(&store, "Text").id().to_string();
    store.apply(Operation::RenameObject {
        object_id: guide_id.clone(),
        name: "Tutorial walkthrough".into(),
    })?;
    store.apply(Operation::SetTextSource {
        object_id: guide_id,
        source: include_str!("../../../tutorials/xgboost/README.md").into(),
    })?;
    store.apply(Operation::AddFrameFromPastedText {
        name: "Iris training".into(),
        text: include_str!("../../../tutorials/xgboost/source/training.tsv").into(),
        x: 580.0,
        y: 60.0,
    })?;
    store.apply(Operation::AddFrameFromPastedText {
        name: "New flowers".into(),
        text: include_str!("../../../tutorials/xgboost/source/scoring.tsv").into(),
        x: 580.0,
        y: 900.0,
    })?;
    resize(&mut store, "Tutorial walkthrough", 500.0, 1080.0)?;
    resize(&mut store, "Iris training", 620.0, 780.0)?;
    resize(&mut store, "New flowers", 620.0, 260.0)?;
    Ok(store)
}
fn finish(store: &mut Store) -> Result<(), CoreError> {
    let training = frame(store, "Iris training");
    let settings = XgboostSettings {
        objective: Objective::Multiclass,
        seed: 42,
        ..Default::default()
    };
    store.apply(Operation::AddModel {
        name: "Iris classifier".into(),
        x: 1240.0,
        y: 60.0,
        spec: ModelSpec {
            source_frame_id: training.id,
            target_column_id: Some(training.columns[4].id.clone()),
            feature_column_ids: training.columns[..4]
                .iter()
                .map(|column| column.id.clone())
                .collect(),
            method: Method::Xgboost,
            covariance: CovarianceMethod::Classical,
            confidence_level: 0.95,
            holdout_fraction: 0.2,
            seed: 42,
            forest: Default::default(),
            xgboost: settings,
        },
    })?;
    let model_id = object(store, "Iris classifier").id().to_string();
    store.apply(Operation::FitModel {
        model_id: model_id.clone(),
    })?;
    let metrics = store
        .document()
        .model(&model_id)?
        .fitted
        .as_ref()
        .unwrap()
        .evaluation_metrics
        .as_ref()
        .unwrap();
    assert!(metrics.accuracy.unwrap() > metrics.baseline_accuracy.unwrap());
    resize(store, "Iris classifier", 500.0, 480.0)?;
    let scoring = frame(store, "New flowers");
    store.apply(Operation::AddModelPredictions {
        model_id,
        source_frame_id: scoring.id,
        feature_column_ids: scoring
            .columns
            .iter()
            .map(|column| column.id.clone())
            .collect(),
        name: "Species predictions".into(),
        x: 1240.0,
        y: 580.0,
    })?;
    let predictions = frame(store, "Species predictions");
    assert_eq!(store.get_frame_page(&predictions.id, 0, 10)?.rows.len(), 3);
    resize(store, "Species predictions", 500.0, 300.0)?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tutorials/xgboost");
    std::fs::create_dir_all(&output)?;
    let mut store = prepare()?;
    store.save(&output.join("xgboost-start.fw"))?;
    finish(&mut store)?;
    store.save(&output.join("xgboost-finished.fw"))?;
    Ok(())
}
