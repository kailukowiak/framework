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
        xgboost: Default::default(),
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
    // A scored table: the source's columns first, then the model's outputs.
    let source_columns = store
        .document()
        .frame(&spec.source_frame_id)
        .unwrap()
        .columns
        .clone();
    assert_eq!(
        frame.columns[..source_columns.len()]
            .iter()
            .map(|column| (column.id.as_str(), column.name.as_str()))
            .collect::<Vec<_>>(),
        source_columns
            .iter()
            .map(|column| (column.id.as_str(), column.name.as_str()))
            .collect::<Vec<_>>()
    );
    let output_column = frame.prediction.as_ref().unwrap().output_column_ids[0].clone();
    let output_name = frame
        .columns
        .iter()
        .find(|column| column.id == output_column)
        .unwrap()
        .name
        .clone();
    assert_eq!(output_name, "Prediction");
    let page = store.get_frame_page(&prediction_id, 0, 2).unwrap();
    assert_eq!(page.rows[0][..source_columns.len()], ["0", "2", "1"]);
    assert!(page.rows[0][source_columns.len()].parse::<f64>().is_ok());
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
    let frame = store.document().frame(&prediction).unwrap();
    // The three source columns carried through, then class and two
    // probabilities.
    assert_eq!(
        frame.prediction.as_ref().unwrap().output_column_ids.len(),
        3
    );
    assert_eq!(frame.columns.len(), 6);
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
        xgboost: Default::default(),
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
    let refit = store.document().frame(prediction_id).unwrap();
    assert_eq!(
        refit.prediction.as_ref().unwrap().output_column_ids[0],
        output_column
    );
    assert!(
        refit
            .columns
            .iter()
            .any(|column| column.id == *output_column)
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

/// The bare vector had no key. Carried beside its source, a prediction can
/// be joined like any column; and paired by position into a frame that
/// shares the scoring frame's rows it lands on the right row and stays
/// live, because the prediction frame is computed row for row from that
/// same source — no snapshot needed first.
#[test]
fn ml_predictions_pair_live_into_a_frame_beside_their_source() {
    let (mut store, spec) = fixture();
    let model_id = fit(&mut store, &spec);
    let prediction_id = predictions(&mut store, &model_id, &spec);
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: spec.source_frame_id.clone(),
            name: "Scored".into(),
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let scored_id = store
        .document()
        .objects
        .iter()
        .find(|o| o.name() == "Scored")
        .unwrap()
        .id()
        .to_owned();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: scored_id.clone(),
            steps: vec![FrameStepInput::ZipVector {
                output_column_id: "predicted".into(),
                name: "Predicted".into(),
                vector: "`Predictions`.`Prediction`".into(),
            }],
        })
        .unwrap();
    let last_of = |rows: &[Vec<String>]| {
        rows.iter()
            .map(|row| row.last().unwrap().clone())
            .collect::<Vec<_>>()
    };
    let predicted = store.get_frame_page(&prediction_id, 0, 30).unwrap();
    let scored = store.get_frame_page(&scored_id, 0, 30).unwrap();
    assert_eq!(scored.rows.len(), 30);
    assert_eq!(scored.rows[0][..3], ["0", "2", "1"]);
    assert_eq!(last_of(&scored.rows), last_of(&predicted.rows));

    // Live: a changed input scores differently, and the paired column
    // follows without the step being written again.
    let row_id = store.document().frame(&spec.source_frame_id).unwrap().rows[0]
        .id
        .clone();
    store
        .apply(Operation::SetCell {
            frame_id: spec.source_frame_id.clone(),
            row_id,
            column_id: spec.feature_column_ids[0].clone(),
            raw: "100".into(),
        })
        .unwrap();
    let rescored = store.get_frame_page(&scored_id, 0, 1).unwrap();
    assert_ne!(rescored.rows[0].last(), scored.rows[0].last());
    assert_eq!(
        rescored.rows[0].last(),
        store.get_frame_page(&prediction_id, 0, 1).unwrap().rows[0].last()
    );

    // The list methods read the live column as well.
    store
        .apply(Operation::AddBlock {
            name: "Peek".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let block_id = store
        .document()
        .objects
        .iter()
        .find(|o| o.name() == "Peek")
        .unwrap()
        .id()
        .to_owned();
    store
        .apply(Operation::SetBlockSource {
            block_id: block_id.clone(),
            source: "top = `Predictions`.`Prediction`.head(3).len()".into(),
            editing: None,
        })
        .unwrap();
    assert_eq!(
        store.view().computed_blocks[&block_id].lines[0].cell.value,
        Some(3.0)
    );
}

fn object_named(store: &Store, name: &str) -> Id {
    store
        .document()
        .objects
        .iter()
        .find(|o| o.name() == name)
        .unwrap()
        .id()
        .to_owned()
}

fn pair_predictions_into(store: &mut Store, scored_id: &Id) {
    store
        .apply(Operation::SetFramePipeline {
            frame_id: scored_id.clone(),
            steps: vec![FrameStepInput::ZipVector {
                output_column_id: "predicted".into(),
                name: "Predicted".into(),
                vector: "`Predictions`.`Prediction`".into(),
            }],
        })
        .unwrap();
}

/// Predictions scored from the same source grow with it: a row added to
/// the scoring frame is scored, and the pairing beside it keeps holding
/// without being written again — the pairing is checked against the rows
/// the frame has now, not the count it had when the step was saved.
#[test]
fn ml_paired_predictions_follow_the_source_as_it_grows() {
    let (mut store, spec) = fixture();
    let model_id = fit(&mut store, &spec);
    let prediction_id = predictions(&mut store, &model_id, &spec);
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: spec.source_frame_id.clone(),
            name: "Scored".into(),
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let scored_id = object_named(&store, "Scored");
    pair_predictions_into(&mut store, &scored_id);
    store
        .apply(Operation::AddRow {
            frame_id: spec.source_frame_id.clone(),
            values: std::collections::BTreeMap::from([(
                spec.feature_column_ids[0].clone(),
                "31".to_string(),
            )]),
        })
        .unwrap();
    let last_of = |rows: &[Vec<String>]| {
        rows.iter()
            .map(|row| row.last().unwrap().clone())
            .collect::<Vec<_>>()
    };
    let scored = store.get_frame_page(&scored_id, 0, 40).unwrap();
    let predicted = store.get_frame_page(&prediction_id, 0, 40).unwrap();
    assert_eq!(scored.total_rows, 31);
    assert_eq!(last_of(&scored.rows), last_of(&predicted.rows));
    assert!(!scored.rows[30].last().unwrap().is_empty());
}

/// A scored frame is its source's columns with the outputs after them, and
/// it stays that as the source's columns change: added, renamed, retyped,
/// dropped — and back again on undo. A name chosen on the scored frame
/// itself is left alone, and a column one of its own steps reads cannot be
/// dropped out from under it.
#[test]
fn ml_scored_frames_follow_their_source_columns() {
    let (mut store, spec) = fixture();
    let model_id = fit(&mut store, &spec);
    let prediction_id = predictions(&mut store, &model_id, &spec);
    let source_id = spec.source_frame_id.clone();
    let source_columns = store.document().frame(&source_id).unwrap().columns.clone();
    let names = |store: &Store| {
        store
            .document()
            .frame(&prediction_id)
            .unwrap()
            .columns
            .iter()
            .map(|column| column.name.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(names(&store), ["Input", "Outcome", "Class", "Prediction"]);

    store
        .apply(Operation::AddColumn {
            frame_id: source_id.clone(),
            name: "Note".into(),
            data_type: DataType::String,
            after_column_id: Some(source_columns[1].id.clone()),
        })
        .unwrap();
    assert_eq!(
        names(&store),
        ["Input", "Outcome", "Note", "Class", "Prediction"]
    );
    let note_id = store.document().frame(&source_id).unwrap().columns[2]
        .id
        .clone();

    store
        .apply(Operation::RenameColumn {
            frame_id: source_id.clone(),
            column_id: source_columns[2].id.clone(),
            name: "Label".into(),
        })
        .unwrap();
    assert_eq!(
        names(&store),
        ["Input", "Outcome", "Note", "Label", "Prediction"]
    );

    store
        .apply(Operation::SetColumnType {
            frame_id: source_id.clone(),
            column_id: note_id.clone(),
            data_type: DataType::Number,
        })
        .unwrap();
    assert_eq!(
        store.document().frame(&prediction_id).unwrap().columns[2].data_type,
        DataType::Number
    );

    store
        .apply(Operation::DeleteColumn {
            frame_id: source_id.clone(),
            column_id: note_id.clone(),
        })
        .unwrap();
    assert_eq!(names(&store), ["Input", "Outcome", "Label", "Prediction"]);
    assert_eq!(
        store
            .get_frame_page(&prediction_id, 0, 1)
            .unwrap()
            .columns
            .len(),
        4
    );
    store.undo();
    assert_eq!(
        names(&store),
        ["Input", "Outcome", "Note", "Label", "Prediction"]
    );

    // A name chosen here is the person's; the source's rename leaves it.
    store
        .apply(Operation::RenameColumn {
            frame_id: prediction_id.clone(),
            column_id: source_columns[1].id.clone(),
            name: "Actual".into(),
        })
        .unwrap();
    store
        .apply(Operation::RenameColumn {
            frame_id: source_id.clone(),
            column_id: source_columns[1].id.clone(),
            name: "Result".into(),
        })
        .unwrap();
    assert_eq!(
        names(&store),
        ["Input", "Actual", "Note", "Label", "Prediction"]
    );

    // A step of the scored frame reading a carried column keeps that
    // column in the source, by name.
    store
        .apply(Operation::SetFramePipeline {
            frame_id: prediction_id.clone(),
            steps: vec![FrameStepInput::Sort {
                keys: vec![SortInput {
                    column_id: source_columns[0].id.clone(),
                    descending: true,
                }],
            }],
        })
        .unwrap();
    let error = store
        .apply(Operation::DeleteColumn {
            frame_id: source_id.clone(),
            column_id: source_columns[0].id.clone(),
        })
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("Predictions") && error.contains("Input"),
        "{error}"
    );
}

/// The tour's model lesson, end to end in the core: one source frame, a
/// native fit scored beside it, then an imported booster scored from the
/// same source with every column as an input. Both scored frames read.
#[test]
fn ml_two_models_score_the_same_source_side_by_side() {
    let mut store = Store::new(Document::blank("Tour"));
    store
        .apply(Operation::AddFrameFromPastedText {
            name: "ML Training Data".into(),
            text: "MLinput\tMLtarget\tMLaux\n1\t3.1\t0\n2\t4.8\t1\n3\t7.2\t0\n4\t8.9\t1\n5\t11.1\t0\n6\t12.8\t1\n7\t15.2\t0\n8\t16.9\t1\n9\t19.1\t0\n10\t20.8\t1\n11\t23.2\t0\n12\t24.9\t1".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let source = store
        .document()
        .objects
        .iter()
        .find_map(|o| match o {
            DataObject::Frame(f) => Some(f.clone()),
            _ => None,
        })
        .unwrap();
    let ids = |names: &[&str]| -> Vec<Id> {
        names
            .iter()
            .map(|name| {
                source
                    .columns
                    .iter()
                    .find(|column| column.name == *name)
                    .unwrap()
                    .id
                    .clone()
            })
            .collect()
    };
    let spec = ModelSpec {
        source_frame_id: source.id.clone(),
        target_column_id: Some(ids(&["MLtarget"])[0].clone()),
        feature_column_ids: ids(&["MLinput", "MLaux"]),
        method: Method::Ols,
        covariance: CovarianceMethod::Hc3,
        confidence_level: 0.95,
        holdout_fraction: 0.2,
        seed: 42,
        forest: Default::default(),
        xgboost: Default::default(),
    };
    let ols_id = fit(&mut store, &spec);
    store
        .apply(Operation::AddModelPredictions {
            model_id: ols_id,
            source_frame_id: source.id.clone(),
            feature_column_ids: ids(&["MLinput", "MLaux"]),
            name: "ScoredData".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::ImportModel {
            name: "Imported XGBoost".into(),
            source_frame_id: source.id.clone(),
            feature_column_ids: ids(&["MLinput", "MLtarget", "MLaux"]),
            json: include_str!(
                "../../../tools/ml-spikes/imports/fixtures/xgboost-regression.model.json"
            )
            .into(),
            iteration_range: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let imported_id = object_named(&store, "Imported XGBoost");
    store
        .apply(Operation::AddModelPredictions {
            model_id: imported_id,
            source_frame_id: source.id.clone(),
            feature_column_ids: ids(&["MLinput", "MLtarget", "MLaux"]),
            name: "ScoredXgb".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    for name in ["ScoredData", "ScoredXgb"] {
        let frame_id = object_named(&store, name);
        let page = store.get_frame_page(&frame_id, 0, 3).unwrap();
        assert_eq!(page.columns.len(), 4, "{name}");
        assert_eq!(page.columns[3].name, "Prediction", "{name}");
        assert!(
            page.rows[0][3].parse::<f64>().is_ok(),
            "{name}: {:?}",
            page.rows[0]
        );
    }
}
