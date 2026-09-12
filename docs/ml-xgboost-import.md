# XGBoost JSON native inference spike

Status: implemented and acceptance-tested as an isolated Rust workspace;
not integrated into FrameWork. This resolves one bounded import question in
`MLConsensus.md`: the selected numerical XGBoost trees can run without loading
XGBoost, Python, or ONNX Runtime.

## What is implemented

[tools/ml-spikes/xgboost-import](../tools/ml-spikes/xgboost-import/) parses the
**actual `.model.json` files** created by the existing Python fixture exporter.
The fixture sidecars supply only probe inputs, expected outputs, tolerances,
and explicit prediction iteration policy. They do not supply tree structures
or fitted parameters to the parser.

The supported contract is deliberately narrow:

- XGBoost `save_model` JSON with version `3.0.x`; the fixtures are from 3.0.2.
- Numerical `gbtree`, one target, one tree per output group per boosting round.
- `reg:squarederror`, `binary:logistic`, and `multi:softprob` objectives.
- Scalar base score, transformed to a logit margin for binary classification.
- Float32 features, split thresholds, leaf values, accumulation, and output
  transformation; split comparison is strictly `<`, so equality goes right.
- Null/NaN numerical input follows the node's saved `default_left` branch.
- Tree output groups and `iteration_indptr` map **boosting rounds**, not tree
  counts, to the selected half-open iteration range.

The scoring interface uses the shared contract's `IterationRange`. An explicit
range must be nonempty and bounded by saved rounds; `None` means all saved
rounds. There is no `(0, 0)` sentinel and no hidden interpretation of the
model's `best_iteration` attribute. The workflow/export adapter must supply
its chosen early-stopping policy. Predictions are numeric outputs in trained
positional column order; multiclass outputs are in group-index order. Class
labels, feature-name mapping, thresholds for label decisions, and sklearn
preprocessing belong to the surrounding workflow contract.

This is a scalar row-scoring interface. It rejects infinite inputs, values
that overflow float32 on conversion, and non-finite accumulated margins.

## Acceptance evidence

On the committed fixtures, all 40 probe rows for each model passed the
sidecar's absolute/relative tolerances, including probes with missing values:

| Model | Maximum absolute error against Python reference | Maximum difference when all saved rounds are used instead |
| --- | --- | --- |
| Regression | 1.39e-17 | 0.621228 |
| Binary classification | 0 | 0.181237 |
| Three-class probabilities | 5.55e-17 | 0.223216 |

The tiny nonzero errors reflect reading reference float32 results through
JSON/f64 comparison. This is evidence for these fixtures and their represented
semantics, not a promise that every XGBoost JSON model has exact parity.
The all-round differences demonstrate why preserving early-stopping policy
matters: scoring every saved tree silently gives a different model result.

Hand-constructed semantic tests additionally prove strict threshold equality,
float64 inputs rounded to float32 before splitting, both missing branches,
binary base-score transformation, and a nonzero-start half-open range.
Malformed-input tests cover unsupported objectives/versions/boosters,
categorical nodes, feature indices, inconsistent parallel arrays, tree IDs
and counts, invalid parents/children, cycles/multiple parents, disconnected
nodes, invalid iteration pointers, and resource limits.

The parser checks a 16 MiB byte limit before JSON decoding, then caps features
at 100,000, classes at 1,000, trees at 10,000, individual trees at 100,000 nodes,
and total nodes at 1,000,000. These are conservative spike limits, not a tuned
production memory budget. JSON decoding still allocates within the byte cap.
Tree topology validation and scoring are iterative rather than recursive;
unreachable or cyclic structures cannot enter the scorer.

## Explicitly unsupported

`dart`, `gblinear`, categorical features/splits, vector leaves, deleted nodes,
multiple targets, parallel forest trees, other objectives/versions, UBJSON,
and arbitrary sklearn pipelines are rejected or outside this interface.
Do not call this a general XGBoost importer yet. Custom Python objectives are
not serialized as executable definitions; a claimed built-in objective in a
model file cannot establish the original training code's identity.

Original feature names are checked for count but are not a feature-binding
API. External class labels and probability display are not inferred from a
bare booster. Persistence and parameter lowering into the shared model object
remain a separate integration step.

## Reproduction and next checks

```sh
cargo test --locked --offline --manifest-path tools/ml-spikes/xgboost-import/Cargo.toml -- --nocapture
cargo fmt --check --manifest-path tools/ml-spikes/xgboost-import/Cargo.toml
cargo clippy --locked --offline --manifest-path tools/ml-spikes/xgboost-import/Cargo.toml --all-targets -- -D warnings
```

The crate has its own lockfile/workspace and only serde, serde_json, and the
shared contract as dependencies. Offline commands assume crates have already
been fetched. The statistical/native-XGBoost packaging spikes are independent.

Before production: fuzz the parser and extend real export fixtures to deeper
and larger trees, extreme margins, float32 boundaries generated by Python,
and each additional supported version/estimator variant. Add cancellation and
batch resource accounting when scoring is exposed as an engine job. None of
these tests proves cross-platform native XGBoost packaging; this scorer has
no native XGBoost runtime to package.

The implemented file structure and bias/objective distinction follow
[XGBoost's model IO documentation](https://xgboost.readthedocs.io/en/release_3.0.0/tutorials/saving_model.html),
with numerical behavior tested against the actual exported models rather
than inferred solely from the schema.
