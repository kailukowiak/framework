use crate::{Prediction, import, proto, wire};
use framework_ml_contract_spike::{
    recipe::{Scalar, ScaleOperation, Step},
    semantics::{ClassLabel, MissingValue, NumericPrecision},
};
use prost::Message;
use serde_json::Value;

const LINEAR: &[u8] = include_bytes!("../../imports/fixtures/branched-linear.onnx");
const LOGISTIC: &[u8] = include_bytes!("../../imports/fixtures/branched-logistic.onnx");

fn fixture(classification: bool) -> Value {
    serde_json::from_str(if classification {
        include_str!("../../imports/fixtures/branched-logistic.json")
    } else {
        include_str!("../../imports/fixtures/branched-linear.json")
    })
    .unwrap()
}

fn rows(sidecar: &Value) -> Vec<Vec<Scalar>> {
    sidecar["probes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            vec![
                Scalar::Number(row["age"].as_f64().unwrap_or(f64::NAN)),
                Scalar::Number(row["income"].as_f64().unwrap_or(f64::NAN)),
                Scalar::String(row["city"].as_str().unwrap().into()),
            ]
        })
        .collect()
}

fn close(actual: f32, expected: f64) {
    assert!(
        (f64::from(actual) - expected).abs() <= 2e-5 + 2e-5 * expected.abs(),
        "{actual} != {expected}"
    );
}

#[test]
fn real_protobuf_predictions_match_sklearn_and_ort() {
    for (bytes, classification) in [(LINEAR, false), (LOGISTIC, true)] {
        let model = import(bytes).unwrap();
        let expected = fixture(classification);
        let predictions = model.predict(&rows(&expected)).unwrap();
        for (i, prediction) in predictions.iter().enumerate() {
            match prediction {
                Prediction::Regression(value) => {
                    close(*value, expected["expected"][i].as_f64().unwrap());
                    close(*value, expected["onnxExpected"][0][i][0].as_f64().unwrap());
                }
                Prediction::Classification {
                    label,
                    probabilities,
                } => {
                    assert_eq!(
                        *label,
                        ClassLabel::String(expected["expected"][i].as_str().unwrap().into())
                    );
                    assert_eq!(
                        *label,
                        ClassLabel::String(expected["onnxExpected"][0][i].as_str().unwrap().into())
                    );
                    for (class, probability) in probabilities.iter().enumerate() {
                        close(
                            *probability,
                            expected["probabilities"][i][class].as_f64().unwrap(),
                        );
                        close(
                            *probability,
                            expected["onnxExpected"][1][i][class].as_f64().unwrap(),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn lowering_preserves_shared_recipe_semantics() {
    let model = import(LOGISTIC).unwrap();
    assert_eq!(model.recipe().precision, NumericPrecision::Float32);
    assert_eq!(
        model.recipe().features,
        [
            "scaled:0",
            "scaled:1",
            "encoded:0",
            "encoded:1",
            "encoded:2"
        ]
    );
    assert!(matches!(
        &model.recipe().steps[0],
        Step::Impute {
            missing: MissingValue::NaN,
            ..
        }
    ));
    assert!(
        matches!(&model.recipe().steps[1], Step::Scale { learned: Some(p), .. }
        if p.operation == ScaleOperation::Multiply)
    );
    assert!(
        matches!(&model.recipe().steps[4], Step::Impute { missing: MissingValue::String(s), .. } if s.is_empty())
    );
    assert_eq!(model.inputs()[2].name, "city");
    assert_eq!(
        model.labels(),
        [
            ClassLabel::String("no".into()),
            ClassLabel::String("yes".into())
        ]
    );
}

fn mutate(bytes: &[u8], mutation: impl FnOnce(&mut proto::Model)) -> Vec<u8> {
    let mut model = proto::Model::decode(bytes).unwrap();
    mutation(&mut model);
    model.encode_to_vec()
}

fn rejected(mutation: impl FnOnce(&mut proto::Model), message: &str) {
    let error = import(&mutate(LINEAR, mutation)).unwrap_err();
    assert!(
        error.0.contains(message),
        "{error} did not contain {message}"
    );
}

#[test]
fn rejects_versions_operators_attributes_and_wiring() {
    rejected(|m| m.ir_version = Some(9), "IR version");
    rejected(|m| m.opsets[0].version = Some(19), "opset versions");
    rejected(|m| m.opsets.push(m.opsets[0].clone()), "opset imports");
    rejected(
        |m| m.graph.as_mut().unwrap().nodes[7].op = Some("If".into()),
        "operator",
    );
    rejected(
        |m| m.graph.as_mut().unwrap().nodes[7].inputs.swap(1, 2),
        "motif wiring",
    );
    rejected(
        |m| m.graph.as_mut().unwrap().nodes[0].attributes[0].name = Some("bad".into()),
        "attribute",
    );
    rejected(
        |m| m.graph.as_mut().unwrap().nodes[0].attributes[0].float = Some(1.0),
        "declared value",
    );
    rejected(
        |m| m.graph.as_mut().unwrap().nodes[0].attributes[0].int = Some(0),
        "axis",
    );
    rejected(
        |m| m.graph.as_mut().unwrap().nodes[0].outputs[0] = "age".into(),
        "duplicate",
    );
}

#[test]
fn rejects_wrong_types_shapes_parameters_and_missing_policies() {
    rejected(
        |m| {
            m.graph.as_mut().unwrap().inputs[0]
                .r#type
                .as_mut()
                .unwrap()
                .tensor
                .as_mut()
                .unwrap()
                .elem_type = Some(11)
        },
        "dtype",
    );
    rejected(
        |m| m.graph.as_mut().unwrap().initializers[1].ints[1] = i64::MAX,
        "reshape",
    );
    rejected(
        |m| {
            m.graph.as_mut().unwrap().nodes[13].attributes[0]
                .floats
                .pop()
                .map(|_| ())
                .unwrap()
        },
        "shape",
    );
    rejected(
        |m| m.graph.as_mut().unwrap().nodes[13].attributes[0].floats[0] = f32::NAN,
        "nonfinite",
    );
    rejected(
        |m| m.graph.as_mut().unwrap().nodes[3].attributes[1].float = Some(0.0),
        "missing policy",
    );
    rejected(
        |m| m.graph.as_mut().unwrap().nodes[10].attributes[1].int = Some(0),
        "zeros",
    );
    let changed = mutate(LOGISTIC, |m| {
        m.graph.as_mut().unwrap().nodes[13].attributes[4].string = Some(b"SOFTMAX".to_vec())
    });
    assert!(import(&changed).unwrap_err().0.contains("post-transform"));
}

#[test]
fn bounded_wire_parser_rejects_malformed_unknown_and_duplicate_fields() {
    assert!(
        import(&vec![0; wire::MAX_BYTES + 1])
            .unwrap_err()
            .0
            .contains("byte limit")
    );
    assert!(import(&[0xff; 10]).is_err());
    assert!(import(&LINEAR[..LINEAR.len() - 1]).is_err());
    let mut duplicate = LINEAR.to_vec();
    duplicate.extend_from_slice(&[8, 8]);
    assert!(
        import(&duplicate)
            .unwrap_err()
            .0
            .contains("duplicate protobuf field")
    );
    let mut unknown = LINEAR.to_vec();
    unknown.extend_from_slice(&[0xa2, 1, 0]); // ModelProto.training_info
    assert!(
        import(&unknown)
            .unwrap_err()
            .0
            .contains("unsupported protobuf field")
    );
    let empty_nodes = mutate(LINEAR, |m| {
        m.graph.as_mut().unwrap().nodes = vec![proto::Node::default(); 5000]
    });
    assert!(import(&empty_nodes).unwrap_err().0.contains("field limit"));
    // Every truncation is safe to inspect; malformed input must never panic.
    for length in 0..LINEAR.len() {
        let _ = import(&LINEAR[..length]);
    }
    for i in (0..LINEAR.len()).step_by(7) {
        let mut bytes = LINEAR.to_vec();
        bytes[i] ^= 0xff;
        let _ = import(&bytes);
    }
}

#[test]
fn renamed_graph_names_and_changed_weights_are_read_from_protobuf() {
    let changed = mutate(LINEAR, |m| {
        let g = m.graph.as_mut().unwrap();
        for value in g.inputs.iter_mut().chain(&mut g.outputs) {
            value.name = value.name.as_ref().map(|s| format!("new_{s}"));
        }
        for value in &mut g.initializers {
            value.name = value.name.as_ref().map(|s| format!("new_{s}"));
        }
        for node in &mut g.nodes {
            for name in node.inputs.iter_mut().chain(&mut node.outputs) {
                *name = format!("new_{name}");
            }
        }
        g.nodes[13].attributes[1].floats[0] += 3.0;
    });
    let original = import(LINEAR).unwrap();
    let model = import(&changed).unwrap();
    assert_eq!(model.inputs()[0].name, "new_age");
    for (a, b) in original
        .predict(&rows(&fixture(false)))
        .unwrap()
        .into_iter()
        .zip(model.predict(&rows(&fixture(false))).unwrap())
    {
        let (Prediction::Regression(a), Prediction::Regression(b)) = (a, b) else {
            panic!()
        };
        close(b - a, 3.0);
    }
}

#[test]
fn scoring_rejects_wrong_types_infinities_and_excessive_batches() {
    let model = import(LINEAR).unwrap();
    assert!(
        model
            .predict(&vec![vec![]; 10001])
            .unwrap_err()
            .0
            .contains("row limit")
    );
    assert!(model.predict(&[vec![]]).is_err());
    let mut row = rows(&fixture(false))[0].clone();
    row[0] = Scalar::Number(f64::INFINITY);
    assert!(model.predict(&[row.clone()]).is_err());
    row[0] = Scalar::String("wrong".into());
    assert!(model.predict(&[row]).is_err());
}
