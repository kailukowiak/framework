# Models and statistical results

FrameWork keeps a model specification, a fitted model, and its outputs separate.
Creating or editing a specification does not silently retrain it. **Fit** captures
one fitted revision; predictions use that revision until you explicitly refit.
Changing training data marks the fit stale, while changing scoring data updates
predictions. A failed fit leaves the last successful fit available. You can delete
a model input column: the affected model or prediction frame reports the missing
input inline, and the saved fit remains intact. Undo restores the connection.

## Learn by doing

The Data library includes two model tutorials, each with a Start workbook and an
Answer key: [robust OLS on diabetes](../tutorials/robust-linear-regression/README.md)
and [XGBoost on Iris](../tutorials/xgboost/README.md). Create tutorials to add the
new lessons without replacing your existing working copies.

## Start with a regression

Create a model from a frame, choose the target column, and
choose linear regression or binary logistic regression. Native regressions use
numeric predictors and an intercept. Logistic targets must be 0 and 1. Missing
values need explicit preparation before fitting; they are not silently dropped.

The default random holdout reserves data for evaluation. It assumes independent
rows: for a time series or grouped observations, do not treat that random score as
future or out-of-group performance. A zero holdout reports training results only.
Baseline metrics use a mean or class frequency learned from training rows.
Wrangle runs before this split. If preprocessing estimates means, scales or other
parameters from data, learn those parameters from training rows only; a holdout
does not undo leakage from preprocessing already fitted on the full frame.

OLS offers classical or HC3 standard errors. HC3 changes uncertainty estimates,
not the least-squares objective; it is not an outlier-resistant regression.
Logistic inference uses model-based standard errors. Separation and unsupported
or numerically unstable fits produce inline errors.

The model's coefficient and metric tables are compact summaries. Create a summary
frame to use these numbers in normal FrameWork formulas. Create predictions to
score rows with the saved fit. These outputs are ordinary frames and can be
transformed through Wrangle.

## Random forests and standalone statistics

Random forests support regression and classification with numeric features.
Set the tree count, depth, minimum leaf size, feature sampling and seed in the
same model dialog. Classification targets use consecutive class indices starting
at zero. The reported probability columns are fractions of tree votes; they are
not a calibration guarantee. Holdout metrics and training-derived baselines use
the same evaluation path as regressions.

The Statistics action produces a mean confidence interval, Pearson correlation,
or Welch difference of means from selected numeric columns. These are explicit
snapshots, with source, workbook revision and confidence level recorded in the
frame comment. They remain ordinary editable frames. Handle missing values in
Wrangle first; statistical rows are not silently removed. Welch's calculation
assumes independent samples, whereas Pearson uses aligned pairs. Statistical
uncertainty depends on those assumptions; the output is not a causal conclusion.

## Train XGBoost

Create a model from a frame and choose XGBoost for a number, a binary category
(0/1), or multiple categories (consecutive class indices starting at zero).
Choose the target column, then create and explicitly fit the model.
The default uses 100 boosting rounds, depth 6 and a 0.1 learning rate. Advanced
settings are optional. Holdout evaluation and live prediction frames work just
like regression; editing data never silently retrains the saved booster.

Native training is available on supported macOS and Windows builds. Predictor
columns currently need finite numeric values; prepare missing values and category
encodings before fitting, observing the training-only preprocessing rule above.
The saved model contains typed trees and can score without a native training
library. See [packaging verification](ml-xgboost-packaging.md) for remaining
clean-machine release checks.

## Import XGBoost

Export a fitted booster using `booster.save_model("model.json")` (or the equivalent
method on a fitted sklearn XGBoost estimator). Select that JSON file in the model
import flow and map each feature to a frame column in the exported order.

The importer supports numerical `gbtree` models in XGBoost 3.0.x JSON with
`reg:squarederror`, `binary:logistic`, or `multi:softprob` objectives. It preserves
saved early-stopping `best_iteration`; probabilities follow class-index order.
NaN follows the saved missing branch. Unsupported formats, categorical splits,
objectives and versions are errors, not approximate conversions. An exported
booster does not contain preprocessing outside that booster: apply the same
external transformations before scoring it.

Import and scoring run locally without Python or a native XGBoost library.

## Import a fitted sklearn pipeline

Pick an ONNX file. Import translates a supported graph into an ordered, typed
fitted recipe; the saved means, scales, categories, feature order, numeric
precision and class labels travel with the model. Scoring never re-estimates
these values from the new dataset. Python pickle and joblib files are unsupported.

The first supported graph is deliberately narrow: two float32 numeric inputs
through imputation and standard scaling, one string input through string-sentinel
imputation and one-hot encoding, followed by linear regression or binary logistic
regression with string labels. Unseen categories map to the saved all-zero policy.
Numeric blanks map to NaN; string nulls are not interchangeable with a trained
string missing sentinel. Up to 32 categories and 10,000 scoring rows are supported.
The bounded importer accepts files up to 64 KiB. Graphs outside this contract are
rejected even if a general ONNX runtime could execute them.

The reproducible export recipe is in
[`tools/ml-spikes/imports/generate.py`](../tools/ml-spikes/imports/generate.py),
with its exact tested package versions in
[`requirements.txt`](../tools/ml-spikes/imports/requirements.txt). It uses
`skl2onnx.convert_sklearn` with separate `[None, 1]` raw inputs, main opset 18,
`ai.onnx.ml` opset 3, and `zipmap=False` for logistic output. This is a compatibility
example, not a promise that arbitrary `ColumnTransformer` graphs will import.

Imported models have no invented training statistics or evaluation results.
Coefficient inference is available only where an estimator actually computed and
retained it. Saved model payloads are currently bounded and stored in the workbook;
large external artifact storage and unrestricted model catalogs remain future work.

For a fitted pipeline matching that recipe, the export call is:

```python
from pathlib import Path
from skl2onnx import convert_sklearn
from skl2onnx.common.data_types import FloatTensorType, StringTensorType

# Use the names and order of your fitted raw columns here.
# `pipeline` is already fitted; no fitting happens during export.
estimator = pipeline.steps[-1][1]
is_classifier = hasattr(estimator, "classes_")
model = convert_sklearn(
    pipeline,
    initial_types=[
        ("age", FloatTensorType([None, 1])),
        ("income", FloatTensorType([None, 1])),
        ("city", StringTensorType([None, 1])),
    ],
    target_opset={"": 18, "ai.onnx.ml": 3},
    options={id(estimator): {"zipmap": False}} if is_classifier else None,
)
Path("pipeline.onnx").write_bytes(model.SerializeToString())
```

Conversion succeeding is not itself a compatibility guarantee. FrameWork checks
the actual operators, wiring, types and fitted values at import, then keeps the
accepted recipe with the model.

Native model fitting uses every source-frame column except the target, in frame order. To use fewer predictors, select the desired columns in a derived frame through Wrangle/formulas, then model that frame. Unsupported column types produce a fit error rather than being silently omitted. Imported models keep their explicit ordered input mapping.
