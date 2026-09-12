//! Generate the ML interaction fixture through production operations and export
//! the canonical bindings. Pass an output path to avoid hand-authored UI data.
use framework_core::*;
use framework_ml::{CovarianceMethod, Method};
use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    DocumentView::export_all(&ts_rs::Config::from_env())?;
    Operation::export_all(&ts_rs::Config::from_env())?;
    let mut store = Store::new(Document::blank("Model examples"));
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
    store.apply(Operation::AddFrameFromPastedText {
        name: "Observations".into(),
        text,
        x: 0.0,
        y: 0.0,
    })?;
    let source = store
        .document()
        .objects
        .iter()
        .find_map(|o| match o {
            DataObject::Frame(f) => Some(f.clone()),
            _ => None,
        })
        .unwrap();
    let spec = ModelSpec {
        source_frame_id: source.id.clone(),
        target_column_id: Some(source.columns[1].id.clone()),
        feature_column_ids: vec![source.columns[0].id.clone()],
        method: Method::Ols,
        covariance: CovarianceMethod::Hc3,
        confidence_level: 0.95,
        holdout_fraction: 0.2,
        seed: 42,
        forest: Default::default(),
    };
    store.apply(Operation::AddModel {
        name: "Demand".into(),
        spec: spec.clone(),
        x: 600.0,
        y: 0.0,
    })?;
    let model_id = store
        .document()
        .objects
        .iter()
        .find_map(|o| match o {
            DataObject::Model(m) => Some(m.id.clone()),
            _ => None,
        })
        .unwrap();
    store.apply(Operation::FitModel {
        model_id: model_id.clone(),
    })?;
    let view_id = store
        .document()
        .views
        .iter()
        .find(|view| view.object_id == model_id)
        .unwrap()
        .id
        .clone();
    store.apply(Operation::ResizeView {
        view_id,
        width: 560.0,
        height: 420.0,
    })?;
    store.apply(Operation::AddModelPredictions {
        model_id,
        source_frame_id: source.id,
        feature_column_ids: spec.feature_column_ids,
        name: "Predictions".into(),
        x: 0.0,
        y: 450.0,
    })?;
    let output = serde_json::to_string_pretty(&store.view())?;
    if let Some(path) = std::env::args().nth(1) {
        std::fs::write(path, output)?;
    } else {
        println!("{output}");
    }
    Ok(())
}
