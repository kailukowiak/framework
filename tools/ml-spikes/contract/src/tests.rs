use crate::ModelContract;
use crate::recipe::Step;
use crate::records::{Evaluation, Output, Provenance};
use crate::semantics::ClassLabel;

fn read(name: &str) -> ModelContract {
    let path = format!("{}/examples/{name}.json", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn contract_examples_roundtrip_without_losing_mapping_or_provenance() {
    for name in ["linear-hc3", "branched-pipeline", "imported-classifier"] {
        let model = read(name);
        model.validate().unwrap();
        let serialized = serde_json::to_string(&model).unwrap();
        assert_eq!(model, serde_json::from_str(&serialized).unwrap());
    }
}

#[test]
fn imported_classifier_needs_no_training_frame_or_outcome() {
    let model = read("imported-classifier");
    assert!(matches!(
        model.fitted.provenance,
        Provenance::Imported { .. }
    ));
    assert!(
        model
            .specification
            .inputs
            .iter()
            .all(|input| input.binding.is_none())
    );
    model.validate().unwrap();
}

#[test]
fn a_role_column_cannot_sneak_into_the_feature_matrix() {
    let mut model = read("linear-hc3");
    model.fitted.recipe.features.push("outcome".into());
    assert!(
        model
            .validate()
            .unwrap_err()
            .contains("not an available predictor")
    );
}

#[test]
fn pipeline_rejects_forward_references_and_overwritten_slots() {
    let mut model = read("branched-pipeline");
    model.fitted.recipe.steps.swap(0, 1);
    assert!(model.validate().unwrap_err().contains("earlier step"));
    let mut model = read("branched-pipeline");
    if let Step::Scale { output, .. } = &mut model.fitted.recipe.steps[1] {
        *output = "numeric".into();
    }
    assert!(model.validate().unwrap_err().contains("overwrites"));
}

#[test]
fn fitted_preprocessing_cannot_be_missing_or_have_a_different_category_width() {
    let mut model = read("branched-pipeline");
    if let Step::Scale { learned, .. } = &mut model.fitted.recipe.steps[1] {
        *learned = None;
    }
    assert!(model.validate().unwrap_err().contains("learned state"));
    let mut model = read("branched-pipeline");
    if let Step::OneHot { learned, .. } = &mut model.fitted.recipe.steps[3] {
        learned.as_mut().unwrap().pop();
    }
    assert!(model.validate().unwrap_err().contains("one-to-one"));
}

#[test]
fn changing_a_display_name_does_not_change_feature_binding() {
    let mut model = read("linear-hc3");
    let binding = model.specification.inputs[0].binding.clone();
    model.specification.inputs[0].name = "Renamed feature".into();
    assert_eq!(binding, model.specification.inputs[0].binding);
    model.validate().unwrap();
}

#[test]
fn classification_outputs_keep_the_label_mapping() {
    let mut model = read("imported-classifier");
    model.fitted.outputs.push(Output::Probability {
        name: "unexpected".into(),
        label: ClassLabel::String("unknown".into()),
    });
    assert!(model.validate().unwrap_err().contains("class label"));
}

#[test]
fn comparison_records_share_assessment_membership_but_not_fit_identity() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/evaluation-comparison.json"
    );
    let evaluations: Vec<Evaluation> =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(evaluations.len(), 2);
    assert_eq!(evaluations[0].split_id, evaluations[1].split_id);
    assert_eq!(evaluations[0].membership, evaluations[1].membership);
    assert_eq!(
        evaluations[0].source_revision,
        evaluations[1].source_revision
    );
    assert_ne!(
        evaluations[0].fitted_revision_id,
        evaluations[1].fitted_revision_id
    );
}
