"""Synthetic reference fixtures, not a production exporter or an ONNX translator."""
import argparse
import importlib.metadata
import json
from pathlib import Path

import numpy as np
import onnx
import onnxruntime as ort
import pandas as pd
from sklearn.compose import ColumnTransformer
from sklearn.impute import SimpleImputer
from sklearn.linear_model import LinearRegression, LogisticRegression
from sklearn.pipeline import Pipeline
from sklearn.preprocessing import OneHotEncoder, StandardScaler
from skl2onnx import convert_sklearn
from skl2onnx.common.data_types import FloatTensorType, StringTensorType


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")


def graph_summary(model):
    """Keep complete attributes in the protobuf; summarize the graph for review."""
    return {
        "opsets": {item.domain: item.version for item in model.opset_import},
        "nodes": [
            {"domain": node.domain, "op": node.op_type,
             "inputs": list(node.input), "outputs": list(node.output),
             "attributes": [attr.name for attr in node.attribute]}
            for node in model.graph.node
        ],
    }


def sklearn_case(output, classification):
    name = "branched-logistic" if classification else "branched-linear"
    train = pd.DataFrame({
        "age": [20, 40, np.nan, 30, 70, 50, 60, 25],
        "income": [1, 3, 2, np.nan, 7, 5, 4, 2],
        "city": ["a", "b", "a", "", "b", "c", "c", "a"],
    })
    probes = pd.DataFrame({
        "age": [np.nan, 30, 80, 20], "income": [2, np.nan, 8, 1],
        "city": ["", "unseen", "c", "a"],
    })
    numeric = Pipeline([("impute", SimpleImputer()), ("scale", StandardScaler())])
    categorical = Pipeline([
        ("impute", SimpleImputer(missing_values="", strategy="most_frequent")),
        ("encode", OneHotEncoder(handle_unknown="ignore", sparse_output=False)),
    ])
    recipe = ColumnTransformer([
        ("numeric", numeric, ["age", "income"]),
        ("categorical", categorical, ["city"]),
    ])
    estimator = LogisticRegression(random_state=7, solver="liblinear") if classification else LinearRegression()
    target = np.array(["no", "yes", "no", "no", "yes", "yes", "yes", "no"]) \
        if classification else np.array([2, 6, 3, 4, 12, 9, 8, 3])
    pipeline = Pipeline([("recipe", recipe), ("model", estimator)]).fit(train, target)
    fitted = pipeline.named_steps["recipe"]
    num = fitted.named_transformers_["numeric"]
    cat = fitted.named_transformers_["categorical"]
    result = {
        "syntheticData": True,
        "rawInputs": [{"name": "age", "dtype": "float32"},
                      {"name": "income", "dtype": "float32"},
                      {"name": "city", "dtype": "string", "missingSentinel": ""}],
        "probes": json.loads(probes.to_json(orient="records")),
        "fittedRecipe": {
            "numeric": {"inputs": ["age", "income"],
                        "impute": num.named_steps["impute"].statistics_.tolist(),
                        "mean": num.named_steps["scale"].mean_.tolist(),
                        "scale": num.named_steps["scale"].scale_.tolist()},
            "categorical": {"inputs": ["city"],
                            "impute": cat.named_steps["impute"].statistics_.tolist(),
                            "categories": [x.tolist() for x in cat.named_steps["encode"].categories_],
                            "unknown": "allZero"},
            "outputSlots": fitted.get_feature_names_out().tolist(),
        },
        "transformedProbes": fitted.transform(probes).tolist(),
        "expected": pipeline.predict(probes).tolist(),
        "coefficients": estimator.coef_.tolist(),
        "intercept": np.asarray(estimator.intercept_).tolist(),
    }
    if classification:
        result["classes"] = estimator.classes_.tolist()
        result["probabilities"] = pipeline.predict_proba(probes).tolist()
    # Float32 export may round coefficients and intermediate results. Both an
    # absolute floor and a relative tolerance are needed near zero and at scale.
    result["tolerance"] = {"atol": 2e-5, "rtol": 2e-5, "dtype": "float32"}
    try:
        model = convert_sklearn(
            pipeline, initial_types=[("age", FloatTensorType([None, 1])),
                                     ("income", FloatTensorType([None, 1])),
                                     ("city", StringTensorType([None, 1]))],
            target_opset={"": 18, "ai.onnx.ml": 3},
            options={id(estimator): {"zipmap": False}} if classification else None,
        )
    except (NotImplementedError, RuntimeError, ValueError) as error:
        result["onnx"] = {"status": "conversionRejected", "error": str(error)}
        write_json(output / f"{name}.json", result)
        return result["onnx"]
    # The converter otherwise mints a random graph name on every export.
    model.graph.name = name
    onnx.checker.check_model(model)
    (output / f"{name}.onnx").write_bytes(model.SerializeToString())
    feed = {column: probes[column].to_numpy(dtype=object if column == "city" else np.float32)
            .reshape(-1, 1) for column in probes}
    actual = ort.InferenceSession(model.SerializeToString(), providers=["CPUExecutionProvider"]).run(None, feed)
    if classification:
        np.testing.assert_array_equal(actual[0], pipeline.predict(probes))
        np.testing.assert_allclose(actual[1], pipeline.predict_proba(probes), atol=2e-5, rtol=2e-5)
    else:
        np.testing.assert_allclose(actual[0].reshape(-1), pipeline.predict(probes), atol=2e-5, rtol=2e-5)
    result["onnxExpected"] = [value.tolist() for value in actual]
    result["onnx"] = {"status": "ortParityPassed", **graph_summary(model)}
    write_json(output / f"{name}.json", result)
    return result["onnx"]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=Path(__file__).parent / "fixtures")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    from xgboost_cases import generate_xgboost
    report = {"versions": {name: importlib.metadata.version(name) for name in
                           ["numpy", "scipy", "pandas", "scikit-learn", "onnx", "skl2onnx", "onnxruntime", "xgboost"]},
              "cases": {}}
    for classification in [False, True]:
        name = "branched-logistic" if classification else "branched-linear"
        report["cases"][name] = sklearn_case(args.output, classification)
    report["cases"].update(generate_xgboost(args.output))
    write_json(args.output / "report.json", report)
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
