use framework_ml_contract_spike::semantics::IterationRange;
use framework_xgboost_import_spike::Model;
use serde_json::Value;
use std::path::PathBuf;

fn fixture(name: &str, model: bool) -> Value {
    let suffix = if model { ".model.json" } else { ".json" };
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../imports/fixtures/xgboost-{name}{suffix}"));
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn parse(value: &Value) -> Model {
    Model::from_json(&serde_json::to_vec(value).unwrap()).unwrap()
}
#[test]
fn native_json_predictions_match_python_and_preserve_early_stopping() {
    for name in ["regression", "binary", "multiclass"] {
        let model = parse(&fixture(name, true));
        let case = fixture(name, false);
        let range = IterationRange {
            start: case["iterationRange"][0].as_u64().unwrap() as u32,
            end: case["iterationRange"][1].as_u64().unwrap() as u32,
        };
        let atol = case["atol"].as_f64().unwrap();
        let rtol = case["rtol"].as_f64().unwrap();
        let mut all_round_difference = 0_f64;
        let mut max_error = 0_f64;
        for (index, probe) in case["probes"].as_array().unwrap().iter().enumerate() {
            let row: Vec<Option<f64>> = serde_json::from_value(probe.clone()).unwrap();
            let actual = model.predict(&row, Some(range)).unwrap();
            let all = model.predict(&row, None).unwrap();
            let expected = &case["expected"][index];
            let expected: Vec<f64> = if expected.is_array() {
                serde_json::from_value(expected.clone()).unwrap()
            } else {
                vec![expected.as_f64().unwrap()]
            };
            assert_eq!(actual.len(), expected.len());
            for ((actual, expected), all) in actual.iter().zip(expected).zip(all) {
                let error = (f64::from(*actual) - expected).abs();
                max_error = max_error.max(error);
                assert!(
                    error <= atol + rtol * expected.abs(),
                    "{name} row {index}: {actual} vs {expected}"
                );
                all_round_difference = all_round_difference.max(f64::from((all - actual).abs()));
            }
        }
        assert!(
            all_round_difference > 0.01,
            "all rounds must differ from stopped policy"
        );
        println!(
            "{name}: max_abs_error={max_error:e}, all_rounds_difference={all_round_difference:e}"
        );
    }
}

fn stump() -> Value {
    let mut model = fixture("regression", true);
    model["learner"]["learner_model_param"]["base_score"] = "0".into();
    let ensemble = &mut model["learner"]["gradient_booster"]["model"];
    ensemble["gbtree_model_param"]["num_trees"] = "1".into();
    ensemble["iteration_indptr"] = serde_json::json!([0, 1]);
    ensemble["tree_info"] = serde_json::json!([0]);
    ensemble["trees"] = serde_json::json!([{
        "id":0, "tree_param":{"num_nodes":"3","num_feature":"3","num_deleted":"0","size_leaf_vector":"1"},
        "left_children":[1,-1,-1], "right_children":[2,-1,-1], "parents":[2147483647,0,0],
        "split_indices":[0,0,0], "split_conditions":[1.0,-2.0,3.0], "split_type":[0,0,0],
        "default_left":[1,0,0], "base_weights":[0,0,0], "loss_changes":[0,0,0], "sum_hessian":[0,0,0],
        "categories":[],"categories_nodes":[],"categories_segments":[],"categories_sizes":[]
    }]);
    model
}
#[test]
fn split_uses_float32_strict_less_than_and_explicit_missing_branch() {
    let model = parse(&stump());
    let score = |x| model.predict(&[x, Some(0.0), Some(0.0)], None).unwrap()[0];
    assert_eq!(
        score(Some(f64::from(f32::from_bits(1_f32.to_bits() - 1)))),
        -2.0
    );
    assert_eq!(score(Some(1.0)), 3.0);
    assert_eq!(
        score(Some(1.0 - 1e-10)),
        3.0,
        "round input to float32 before comparison"
    );
    assert_eq!(score(None), -2.0);
    assert_eq!(score(Some(f64::NAN)), -2.0);
    let mut value = stump();
    value["learner"]["gradient_booster"]["model"]["trees"][0]["default_left"][0] = 0.into();
    assert_eq!(
        parse(&value).predict(&[None, None, None], None).unwrap(),
        [3.0]
    );
}
#[test]
fn objective_base_scores_and_explicit_half_open_ranges() {
    let mut value = stump();
    value["learner"]["learner_model_param"]["base_score"] = "0.8".into();
    value["learner"]["objective"]["name"] = "binary:logistic".into();
    value["learner"]["gradient_booster"]["model"]["trees"][0]["split_conditions"] =
        serde_json::json!([1, 0, 0]);
    assert!((parse(&value).predict(&[None, None, None], None).unwrap()[0] - 0.8).abs() < 1e-7);
    let mut value = stump();
    let ensemble = &mut value["learner"]["gradient_booster"]["model"];
    let mut second = ensemble["trees"][0].clone();
    second["id"] = 1.into();
    second["split_conditions"] = serde_json::json!([1, 7, 9]);
    ensemble["trees"].as_array_mut().unwrap().push(second);
    ensemble["gbtree_model_param"]["num_trees"] = "2".into();
    ensemble["iteration_indptr"] = serde_json::json!([0, 1, 2]);
    ensemble["tree_info"] = serde_json::json!([0, 0]);
    let model = parse(&value);
    assert_eq!(
        model
            .predict(
                &[None, None, None],
                Some(IterationRange { start: 1, end: 2 })
            )
            .unwrap(),
        [7.0]
    );
    assert_eq!(model.predict(&[None, None, None], None).unwrap(), [5.0]);
    for (start, end) in [(0, 0), (2, 1), (0, 3)] {
        assert!(
            model
                .predict(&[None, None, None], Some(IterationRange { start, end }))
                .is_err()
        );
    }
    assert!(model.predict(&[None], None).is_err());
    assert!(model.predict(&[Some(f64::MAX), None, None], None).is_err());
}
#[test]
fn malformed_and_unsupported_models_are_rejected_before_scoring() {
    let mutations = [
        ("/learner/gradient_booster/model/trees/0/id", 1.into()),
        (
            "/learner/gradient_booster/model/gbtree_model_param/num_trees",
            "2".into(),
        ),
        ("/version", serde_json::json!([3, 1, 0])),
        ("/learner/gradient_booster/name", "dart".into()),
        ("/learner/gradient_booster/name", "gblinear".into()),
        ("/learner/objective/name", "count:poisson".into()),
        ("/learner/learner_model_param/num_feature", "100001".into()),
        ("/learner/learner_model_param/base_score", "NaN".into()),
        (
            "/learner/gradient_booster/model/tree_info",
            serde_json::json!([1]),
        ),
        (
            "/learner/gradient_booster/model/iteration_indptr",
            serde_json::json!([0, 2]),
        ),
        (
            "/learner/gradient_booster/model/trees/0/left_children",
            serde_json::json!([1, -1]),
        ),
        (
            "/learner/gradient_booster/model/trees/0/left_children/0",
            999.into(),
        ),
        (
            "/learner/gradient_booster/model/trees/0/left_children/0",
            0.into(),
        ),
        (
            "/learner/gradient_booster/model/trees/0/right_children/0",
            1.into(),
        ),
        (
            "/learner/gradient_booster/model/trees/0/parents/1",
            2.into(),
        ),
        (
            "/learner/gradient_booster/model/trees/0/split_indices/0",
            3.into(),
        ),
        (
            "/learner/gradient_booster/model/trees/0/default_left/0",
            2.into(),
        ),
        (
            "/learner/gradient_booster/model/trees/0/split_type/0",
            1.into(),
        ),
        (
            "/learner/gradient_booster/model/trees/0/categories",
            serde_json::json!([1]),
        ),
        (
            "/learner/gradient_booster/model/trees/0/tree_param/num_nodes",
            "100001".into(),
        ),
        (
            "/learner/gradient_booster/model/trees/0/tree_param/num_deleted",
            "1".into(),
        ),
    ];
    for (pointer, replacement) in mutations {
        let mut value = stump();
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert!(
            Model::from_json(&serde_json::to_vec(&value).unwrap()).is_err(),
            "accepted {pointer}"
        );
    }
    assert!(
        Model::from_json(&vec![b' '; 16 * 1024 * 1024 + 1])
            .err()
            .unwrap()
            .contains("resource")
    );
}

#[test]
fn disconnected_nodes_and_model_resource_counts_are_rejected() {
    let mut disconnected = stump();
    let tree = &mut disconnected["learner"]["gradient_booster"]["model"]["trees"][0];
    tree["left_children"][0] = (-1).into();
    tree["right_children"][0] = (-1).into();
    let error = Model::from_json(&serde_json::to_vec(&disconnected).unwrap())
        .err()
        .unwrap();
    assert!(error.contains("unreachable"));
    let mut huge = stump();
    let ensemble = &mut huge["learner"]["gradient_booster"]["model"];
    let tree = ensemble["trees"][0].clone();
    ensemble["trees"] = Value::Array(vec![tree; 10_001]);
    let error = Model::from_json(&serde_json::to_vec(&huge).unwrap())
        .err()
        .unwrap();
    assert!(error.contains("tree resource"));
    let mut invalid_base = stump();
    invalid_base["learner"]["objective"]["name"] = "binary:logistic".into();
    let error = Model::from_json(&serde_json::to_vec(&invalid_base).unwrap())
        .err()
        .unwrap();
    assert!(error.contains("base_score"));
}
