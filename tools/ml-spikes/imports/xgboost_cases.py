"""Check saved-booster prediction policy using synthetic train/validation data."""
import json

import numpy as np
import xgboost as xgb


def generate_xgboost(output):
    rng = np.random.default_rng(19)
    features = rng.normal(size=(120, 3)).astype(np.float32)
    features[::9, 1] = np.nan
    signal = features[:, 0] + np.nan_to_num(features[:, 1]) * 0.5
    report = {}
    cases = [("regression", xgb.XGBRegressor, signal, "reg:squarederror"),
             ("binary", xgb.XGBClassifier, (signal > 0).astype(int), "binary:logistic"),
             ("multiclass", xgb.XGBClassifier, np.digitize(signal, [-0.5, 0.5]), "multi:softprob")]
    for name, constructor, target, objective in cases:
        model = constructor(n_estimators=50, max_depth=2, learning_rate=0.2,
                            n_jobs=1, random_state=7, objective=objective,
                            early_stopping_rounds=3)
        # A separate, deliberately mismatched validation label distribution makes
        # unused trailing trees observable instead of assuming early stopping ran.
        validation = target[80:][::-1]
        model.fit(features[:80], target[:80], eval_set=[(features[80:], validation)], verbose=False)
        path = output / f"xgboost-{name}.model.json"
        model.save_model(path)
        restored = xgb.Booster()
        restored.load_model(path)
        matrix = xgb.DMatrix(features[80:])
        expected = model.predict(features[80:]) if name == "regression" else model.predict_proba(features[80:])
        if name == "binary":
            expected = expected[:, 1]
        actual = restored.predict(matrix, iteration_range=(0, model.best_iteration + 1))
        np.testing.assert_allclose(actual, expected, atol=1e-7, rtol=1e-6)
        all_trees = restored.predict(matrix)
        assert restored.num_boosted_rounds() > model.best_iteration + 1
        assert not np.allclose(all_trees, expected, atol=1e-7, rtol=1e-6)
        info = {"status": "savedBoosterRangeParityPassed", "objective": objective,
                "bestIteration": model.best_iteration,
                "savedRounds": restored.num_boosted_rounds(),
                "iterationRange": [0, model.best_iteration + 1],
                "classes": None if name == "regression" else model.classes_.tolist(),
                "expected": expected.tolist(),
                "probes": [[None if np.isnan(v) else float(v) for v in row] for row in features[80:]],
                "atol": 1e-7, "rtol": 1e-6,
                "allTreesMaxDifference": float(np.max(np.abs(all_trees - expected)))}
        (output / f"xgboost-{name}.json").write_text(json.dumps(info, indent=2, allow_nan=False) + "\n")
        report[f"xgboost-{name}"] = {k: v for k, v in info.items() if k not in ["expected", "probes"]}
    return report
