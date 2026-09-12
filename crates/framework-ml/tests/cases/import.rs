use framework_ml::*;
use serde_json::Value;
fn case(name: &str, suffix: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/fixtures/xgboost-{name}{suffix}.json")),
    )
    .unwrap()
}

#[test]
fn imported_numerical_xgboost_matches_python_saved_early_stopping() {
    for name in ["regression", "binary", "multiclass"] {
        let fixture: Value = serde_json::from_slice(&case(name, "")).unwrap();
        let model = import_xgboost(&case(name, ".model")).unwrap();
        let rows: Vec<Vec<Option<f64>>> =
            serde_json::from_value(fixture["probes"].clone()).unwrap();
        let rows: Vec<Vec<f64>> = rows
            .into_iter()
            .map(|row| row.into_iter().map(|v| v.unwrap_or(f64::NAN)).collect())
            .collect();
        let expected = &fixture["expected"];
        let output = predict(&model, &rows).unwrap();
        for i in 0..rows.len() {
            let actual = match name {
                "regression" => vec![output.values[i]],
                "binary" => vec![output.probabilities.as_ref().unwrap()[i][1]],
                _ => output.probabilities.as_ref().unwrap()[i].clone(),
            };
            let expected: Vec<f64> = if expected[i].is_array() {
                serde_json::from_value(expected[i].clone()).unwrap()
            } else {
                vec![expected[i].as_f64().unwrap()]
            };
            for (a, b) in actual.iter().zip(expected) {
                assert!(
                    (a - b).abs()
                        <= fixture["atol"].as_f64().unwrap()
                            + fixture["rtol"].as_f64().unwrap() * b.abs(),
                    "{name}: {a} vs {b}"
                );
            }
        }
        let serialized = serde_json::to_string(&model).unwrap();
        let loaded: FittedModel = serde_json::from_str(&serialized).unwrap();
        assert_eq!(predict(&loaded, &rows).unwrap(), output);
        let columns = predict_columns(&loaded, &rows).unwrap();
        assert_eq!(columns[0].len(), output_names(&loaded).len());
    }
}

#[test]
fn tampered_persisted_tree_is_rejected_before_scoring() {
    let model = import_xgboost(&case("binary", ".model")).unwrap();
    let mut json = serde_json::to_value(&model).unwrap();
    json["payload"]["model"]["trees"][0][0] = serde_json::json!({"kind":"split","data":{"feature":0,"threshold":0.0,"left":0,"right":1,"default_left":false}});
    let corrupted: FittedModel = serde_json::from_value(json).unwrap();
    assert!(validate_fitted(&corrupted).is_err());
    assert!(predict(&corrupted, &[vec![0.; corrupted.feature_names.len()]]).is_err());
}

#[test]
fn unsupported_booster_and_early_stopping_range_are_explicit() {
    let mut raw: Value = serde_json::from_slice(&case("binary", ".model")).unwrap();
    raw["learner"]["gradient_booster"]["name"] = "dart".into();
    assert!(import_xgboost(&serde_json::to_vec(&raw).unwrap()).is_err());
    assert!(
        import_xgboost_with_range(
            &case("binary", ".model"),
            Some(IterationRange {
                start: 0,
                end: u32::MAX
            })
        )
        .is_err()
    );
}
