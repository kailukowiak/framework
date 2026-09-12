//! Generate the diabetes OLS/HC3 lesson and completed answer key through public operations.
use framework_core::*;
use framework_ml::{CovarianceMethod, Method};
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
    let mut store = Store::new_tutorial(Document::blank(
        "Robust standard errors with diabetes progression",
    ));
    store.apply(Operation::AddText { x: 30.0, y: 60.0 })?;
    let guide_id = object(&store, "Text").id().to_string();
    store.apply(Operation::RenameObject {
        object_id: guide_id.clone(),
        name: "Tutorial walkthrough".into(),
    })?;
    store.apply(Operation::SetTextSource {
        object_id: guide_id,
        source: include_str!("../../../tutorials/robust-linear-regression/README.md").into(),
    })?;
    store.apply(Operation::AddFrameFromPastedText {
        name: "Diabetes training".into(),
        text: include_str!("../../../tutorials/robust-linear-regression/source/training.tsv")
            .into(),
        x: 580.0,
        y: 60.0,
    })?;
    store.apply(Operation::AddFrameFromPastedText {
        name: "New patients".into(),
        text: include_str!("../../../tutorials/robust-linear-regression/source/scoring.tsv").into(),
        x: 580.0,
        y: 900.0,
    })?;
    resize(&mut store, "Tutorial walkthrough", 500.0, 1080.0)?;
    resize(&mut store, "Diabetes training", 620.0, 780.0)?;
    resize(&mut store, "New patients", 620.0, 260.0)?;
    Ok(store)
}
fn finish(store: &mut Store) -> Result<(), CoreError> {
    let training = frame(store, "Diabetes training");
    store.apply(Operation::AddModel {
        name: "Progression · HC3 OLS".into(),
        x: 1240.0,
        y: 60.0,
        spec: ModelSpec {
            source_frame_id: training.id,
            target_column_id: Some(training.columns[2].id.clone()),
            feature_column_ids: training.columns[..2]
                .iter()
                .map(|column| column.id.clone())
                .collect(),
            method: Method::Ols,
            covariance: CovarianceMethod::Hc3,
            confidence_level: 0.95,
            holdout_fraction: 0.2,
            seed: 42,
            forest: Default::default(),
            xgboost: Default::default(),
        },
    })?;
    let model_id = object(store, "Progression · HC3 OLS").id().to_string();
    store.apply(Operation::FitModel {
        model_id: model_id.clone(),
    })?;
    let fitted = store.document().model(&model_id)?.fitted.as_ref().unwrap();
    assert_eq!(
        fitted.result.summary.covariance,
        Some(CovarianceMethod::Hc3)
    );
    assert_eq!(fitted.result.summary.coefficients.len(), 3);
    let metrics = fitted.evaluation_metrics.as_ref().unwrap();
    assert!(metrics.rmse.unwrap() < metrics.baseline_rmse.unwrap());
    resize(store, "Progression · HC3 OLS", 500.0, 480.0)?;
    let scoring = frame(store, "New patients");
    store.apply(Operation::AddModelPredictions {
        model_id,
        source_frame_id: scoring.id,
        feature_column_ids: scoring
            .columns
            .iter()
            .map(|column| column.id.clone())
            .collect(),
        name: "Progression predictions".into(),
        x: 1240.0,
        y: 580.0,
    })?;
    let predictions = frame(store, "Progression predictions");
    assert_eq!(store.get_frame_page(&predictions.id, 0, 10)?.rows.len(), 3);
    resize(store, "Progression predictions", 500.0, 300.0)?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tutorials/robust-linear-regression");
    std::fs::create_dir_all(&output)?;
    let mut store = prepare()?;
    store.save(&output.join("robust-linear-regression-start.fw"))?;
    finish(&mut store)?;
    store.save(&output.join("robust-linear-regression-finished.fw"))?;
    Ok(())
}
