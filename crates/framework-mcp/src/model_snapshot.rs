use framework_core::{DataObject, DocumentView};
use serde_json::{Value, json};

/// Return the statistical result and binding, not the potentially megabytes of
/// tree nodes. The values are serialized from the canonical model types so the
/// read API does not grow a second model schema beside the operation catalog.
pub(crate) fn snapshot(view: &DocumentView, name_or_id: &str) -> Result<Value, String> {
    let models = view
        .document
        .objects
        .iter()
        .filter_map(|object| match object {
            DataObject::Model(model) => Some(model.as_ref()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let model = if let Some(model) = models.iter().find(|model| model.id == name_or_id) {
        *model
    } else {
        let matches = models
            .iter()
            .filter(|model| model.name == name_or_id)
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [model] => **model,
            [] => return Err(format!("No model named '{name_or_id}'")),
            _ => return Err("More than one model has that name; use its stable ID".into()),
        }
    };
    let fit = model.fitted.as_ref();
    Ok(json!({
        "id":model.id, "name":model.name,
        "specification":model.spec, "scoringInput":model.imported_input,
        "fittedRevision":fit.map(|fit| &fit.id),
        "trainingRevision":fit.map(|fit| fit.training_revision),
        "featureNames":fit.map(|fit| &fit.result.feature_names),
        "summary":fit.map(|fit| &fit.result.summary),
        "evaluationMetrics":fit.and_then(|fit| fit.evaluation_metrics.as_ref()),
        "state":view.computed_models.get(&model.id),
    }))
}
