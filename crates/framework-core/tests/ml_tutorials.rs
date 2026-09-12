use framework_core::{DataObject, Store};
use framework_ml::{CovarianceMethod, ModelPayload};
use std::path::PathBuf;

fn lesson(folder: &str, state: &str) -> Store {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../tutorials/{folder}/{folder}-{state}.fw"));
    Store::load(&path).unwrap()
}

#[test]
fn model_tutorials_start_unfitted_and_ship_working_answers() {
    for (folder, model_name, prediction_name, training_rows) in [
        (
            "robust-linear-regression",
            "Progression · HC3 OLS",
            "Progression predictions",
            439,
        ),
        ("xgboost", "Iris classifier", "Species predictions", 147),
    ] {
        let start = lesson(folder, "start");
        assert!(
            !start
                .document()
                .objects
                .iter()
                .any(|o| matches!(o, DataObject::Model(_)))
        );
        let finished = lesson(folder, "finished");
        let model = finished
            .document()
            .objects
            .iter()
            .find_map(|o| match o {
                DataObject::Model(m) if m.name == model_name => Some(m),
                _ => None,
            })
            .unwrap();
        let fit = model.fitted.as_ref().unwrap();
        let split = fit.split.as_ref().unwrap();
        assert_eq!(
            split.training_rows.len() + split.evaluation_rows.len(),
            training_rows
        );
        assert!(fit.evaluation_metrics.is_some());
        assert!(!finished.view().computed_models[&model.id].stale);
        if folder == "robust-linear-regression" {
            assert_eq!(fit.result.summary.covariance, Some(CovarianceMethod::Hc3));
            assert_eq!(fit.result.summary.coefficients.len(), 3);
        } else {
            assert!(matches!(&fit.result.payload, ModelPayload::Xgboost { .. }));
        }
        let output = finished
            .document()
            .objects
            .iter()
            .find(|o| o.name() == prediction_name)
            .unwrap();
        let page = finished.get_frame_page(output.id(), 0, 10).unwrap();
        assert_eq!(page.total_rows, 3);
        assert!(
            page.rows
                .iter()
                .all(|row| row.iter().all(|value| !value.is_empty()))
        );
    }
}
