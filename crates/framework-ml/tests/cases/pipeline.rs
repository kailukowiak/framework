use framework_ml::pipeline_types::{Dtype, Scalar};
use framework_ml::*;
use serde_json::Value;

#[test]
fn imported_pipeline_uses_typed_inputs_and_retains_recipe_on_reload() {
    for name in ["linear", "logistic"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/fixtures/branched-{name}"));
        let bytes = std::fs::read(path.with_extension("onnx")).unwrap();
        let fixture: Value =
            serde_json::from_slice(&std::fs::read(path.with_extension("json")).unwrap()).unwrap();
        let model = import_onnx(&bytes).unwrap();
        assert_eq!(
            input_types(&model),
            vec![Dtype::Number, Dtype::Number, Dtype::String]
        );
        let rows: Vec<Vec<Scalar>> = fixture["probes"]
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
            .collect();
        let output = predict_scalars(&model, &rows).unwrap();
        let loaded: FittedModel =
            serde_json::from_str(&serde_json::to_string(&model).unwrap()).unwrap();
        assert_eq!(output, predict_scalars(&loaded, &rows).unwrap());
        assert_eq!(output[0].len(), output_names(&model).len());
        assert_eq!(output[0].len(), output_types(&model).len());
        assert!(model.training_baseline.is_none());
        assert!(
            model
                .summary
                .coefficients
                .iter()
                .all(|c| c.standard_error.is_none())
        );
        if name == "logistic" {
            assert_eq!(output_types(&model)[0], Dtype::String);
            assert!(matches!(&output[0][0], Scalar::String(_)));
        }
    }
}
