use framework_core::*;
use framework_ml::{Method, StatsMethod, StatsRequest};

fn frame_id(store: &Store, name: &str) -> Id {
    store
        .document()
        .objects
        .iter()
        .find_map(|o| match o {
            DataObject::Frame(f) if f.name == name => Some(f.id.clone()),
            _ => None,
        })
        .unwrap()
}

#[test]
fn ml_onnx_preserves_typed_labels_missing_policy_and_reopen() {
    let mut store = Store::new(Document::blank("Pipeline"));
    store
        .apply(Operation::AddFrameFromPastedText {
            name: "Inputs".into(),
            text: "age\tincome\tcity\n30\t\tunseen\n80\t8\tc\n20\t1\ta".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let source = frame_id(&store, "Inputs");
    let columns: Vec<_> = store
        .document()
        .frame(&source)
        .unwrap()
        .columns
        .iter()
        .map(|c| c.id.clone())
        .collect();
    store
        .apply(Operation::ImportOnnxModel {
            name: "Pipeline".into(),
            source_frame_id: source.clone(),
            feature_column_ids: columns.clone(),
            bytes: include_bytes!("../../framework-ml/tests/fixtures/onnx/branched-logistic.onnx")
                .to_vec(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let model = store
        .document()
        .objects
        .iter()
        .find_map(|o| match o {
            DataObject::Model(m) => Some(m.id.clone()),
            _ => None,
        })
        .unwrap();
    store
        .apply(Operation::AddModelPredictions {
            model_id: model.clone(),
            source_frame_id: source.clone(),
            feature_column_ids: columns.clone(),
            name: "Scored".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let output = frame_id(&store, "Scored");
    let outputs = store
        .document()
        .frame(&output)
        .unwrap()
        .prediction
        .as_ref()
        .unwrap()
        .output_column_ids
        .clone();
    let page = store.get_frame_page(&output, 0, 10).unwrap();
    let at = |id: &str| {
        page.columns
            .iter()
            .position(|column| column.id == id)
            .unwrap()
    };
    // The scoring rows come through beside the outputs.
    assert_eq!(page.rows[0][..3], ["30", "", "unseen"]);
    assert_eq!(
        page.rows
            .iter()
            .map(|r| r[at(&outputs[0])].as_str())
            .collect::<Vec<_>>(),
        ["no", "yes", "no"]
    );
    assert!((page.rows[0][at(&outputs[2])].parse::<f64>().unwrap() - 0.393720984).abs() < 0.0001);
    let directory = std::env::temp_dir().join(format!("framework-model-{}", framework_core::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("pipeline.fw");
    store.save(&path).unwrap();
    let reopened = Store::load(&path).unwrap();
    assert_eq!(
        reopened.get_frame_page(&output, 0, 10).unwrap().rows,
        page.rows
    );
    let row = store.document().frame(&source).unwrap().rows[0].id.clone();
    store
        .apply(Operation::SetCell {
            frame_id: source,
            row_id: row,
            column_id: columns[2].clone(),
            raw: "".into(),
        })
        .unwrap();
    assert!(store.get_frame_page(&output, 0, 10).is_err());
    assert!(store.document().model(&model).unwrap().fitted.is_some());
    store.undo();
    assert_eq!(
        store.get_frame_page(&output, 0, 10).unwrap().rows,
        page.rows
    );
}

#[test]
fn ml_random_forest_and_statistics_use_normal_history_frames() {
    let (mut store, mut spec) = super::ml_models::fixture();
    spec.method = Method::RandomForestRegressor;
    spec.forest.trees = 8;
    let model = super::ml_models::fit(&mut store, &spec);
    let prediction = super::ml_models::predictions(&mut store, &model, &spec);
    assert_eq!(
        store
            .get_frame_page(&prediction, 0, 100)
            .unwrap()
            .total_rows,
        30
    );
    store
        .apply(Operation::AddStatisticalAnalysis {
            name: "Mean".into(),
            source_frame_id: spec.source_frame_id.clone(),
            x_column_id: spec.feature_column_ids[0].clone(),
            y_column_id: None,
            request: StatsRequest {
                method: StatsMethod::MeanConfidence,
                confidence_level: 0.95,
            },
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let summary = frame_id(&store, "Mean");
    let frame = store.document().frame(&summary).unwrap();
    assert!(frame.owns_its_rows());
    assert!(
        frame
            .comment
            .as_ref()
            .unwrap()
            .contains("Mean confidence interval")
    );
    assert!(
        store
            .get_frame_page(&summary, 0, 100)
            .unwrap()
            .rows
            .iter()
            .any(|r| r[1]
                .parse::<f64>()
                .is_ok_and(|value| (value - 14.5).abs() < 1e-10))
    );
    store.undo();
    assert!(store.document().frame(&summary).is_err());
}

#[test]
fn ml_prediction_frame_supports_wrangle_with_stable_output_ids() {
    let (mut store, spec) = super::ml_models::fixture();
    let model = super::ml_models::fit(&mut store, &spec);
    let output = super::ml_models::predictions(&mut store, &model, &spec);
    let prediction_column = store
        .document()
        .frame(&output)
        .unwrap()
        .prediction
        .as_ref()
        .unwrap()
        .output_column_ids[0]
        .clone();
    let calculated = id();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: output.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: calculated.clone(),
                    name: "Double".into(),
                    formula: "`Prediction` * 2".into(),
                }],
            }],
        })
        .unwrap();
    let page = store.get_frame_page(&output, 0, 10).unwrap();
    let at = |page: &FramePage, id: &str| {
        page.columns
            .iter()
            .position(|column| column.id == id)
            .unwrap()
    };
    assert_eq!(at(&page, &prediction_column), 3);
    assert_eq!(at(&page, &calculated), 4);
    for row in &page.rows {
        let first: f64 = row[3].parse().unwrap();
        let second: f64 = row[4].parse().unwrap();
        assert!((second - first * 2.0).abs() < 0.02);
    }
    store
        .apply(Operation::FitModel { model_id: model })
        .unwrap();
    let page = store.get_frame_page(&output, 0, 10).unwrap();
    assert_eq!(at(&page, &prediction_column), 3);
    assert_eq!(at(&page, &calculated), 4);
}

#[test]
fn ml_undo_recipe_repair_restores_missing_reference_without_losing_fit() {
    let (mut store, spec) = super::ml_models::fixture();
    let model = super::ml_models::fit(&mut store, &spec);
    let saved_fit = store.document().model(&model).unwrap().fitted.clone();
    let replacement = store
        .document()
        .frame(&spec.source_frame_id)
        .unwrap()
        .columns[2]
        .id
        .clone();
    store
        .apply(Operation::DeleteColumn {
            frame_id: spec.source_frame_id.clone(),
            column_id: spec.feature_column_ids[0].clone(),
        })
        .unwrap();
    let mut repaired = spec.clone();
    repaired.feature_column_ids = vec![replacement];
    store
        .apply(Operation::SetModelSpec {
            model_id: model.clone(),
            spec: repaired,
        })
        .unwrap();
    assert!(store.view().computed_models[&model].error.is_none());
    store.undo();
    assert_eq!(store.document().model(&model).unwrap().spec, Some(spec));
    assert!(store.view().computed_models[&model].error.is_some());
    assert_eq!(store.document().model(&model).unwrap().fitted, saved_fit);
    store.undo();
    assert!(!store.view().computed_models[&model].stale);
}
