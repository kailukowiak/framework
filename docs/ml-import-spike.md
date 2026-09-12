# ML import spike: first reference fixtures

This implements the fixture/contract portion of MLConsensus milestone 2 and a
bounded native Rust translator/evaluator for the two branched linear fixtures.
It does not connect that translator to the application or decide whether to ship ORT.
Executed locally on macOS arm64 with Python 3.12; reference versions and exact
graph inventories are in `tools/ml-spikes/imports/fixtures/report.json`.

## Evidence obtained

| Synthetic case | Executed evidence | Still unproved |
|---|---|---|
| Branched linear pipeline | Numeric mean imputation → scaling; string most-frequent imputation → one-hot; concatenation; native Rust and ORT predictions match sklearn | Generalized translator and production integration |
| Branched logistic pipeline | Same recipe; unknown category maps to zero one-hot slots; string labels and probability order match in Rust, ORT and sklearn | Multiclass and alternate converter motifs |
| XGBoost numerical regression | Saved JSON reloaded with native Booster; prediction range matches sklearn wrapper | Native FrameWork tree walk |
| XGBoost binary classification | Same; probabilities and numeric class order recorded | Threshold/label behavior in FrameWork |
| XGBoost multiclass | Same; three-class probability matrix and class order recorded | Native multiclass tree grouping |

The ONNX comparisons use float32 inputs with `atol=2e-5, rtol=2e-5`; labels
must match exactly. Native XGBoost round-trip comparisons use `atol=1e-7,
rtol=1e-6`. These are fixture-specific acceptance tolerances, not a universal
promise for all estimators or data scales. Fixtures include missing numerics,
a missing string sentinel, and an unseen string category. Only synthetic rows
are included. There is no sample-data consent question for these fixtures.

## The operator contract needs more than the original short chain

The pinned converter successfully exports string imputation, but not as a
single ONNX-ML Imputer. Its generated subgraph uses `LabelEncoder`,
`ArrayFeatureExtractor`, `Cast`, and `Where`. One-hot processing adds `Gather`
and `Reshape`. Numeric imputation uses `Imputer`, then `Scaler`; `Concat` joins
branches in the learned feature order before `LinearRegressor` or
`LinearClassifier`. The classifier stores `classlabels_strings` and an explicit
post-transform. Export disables ZipMap, making the probability matrix's class
ordering explicit in the sidecar.

This is an executable correction to the historical `[Concat] → transforms →
estimator` sketch. A translator can recognize and lower these *validated
subgraphs* into simple typed imputation and encoding steps. It cannot discard
their semantics merely because the intended sklearn transformer is familiar.
Supporting these motifs does not imply supporting arbitrary `Where` graphs.
The graph carries float32 parameters, while the sidecar records fitted sklearn
parameters at their original precision; native ONNX parity must use the graph's
precision rather than silently substituting sidecar coefficients.

The saved typed plan must preserve raw names/types, explicit missing policies,
branch input selection, ordered fitted transformations, learned statistics,
category order and unknown-category policy, concatenated output slots, and
estimator parameter bindings. It must preserve row alignment. Raw input lineage
and transformed feature slots are separate identities. The sidecars deliberately
record both `numeric__age` and categorical slots such as `categorical__city_a`.

The shared `tools/ml-spikes/contract` vocabulary now carries missing sentinels,
execution precision, typed labels and prediction iteration ranges. Its explicit
scale operation resolves a material arithmetic difference: sklearn's saved
`scale_` divides `(x - mean)`, whereas ONNX Scaler's `scale` multiplies
`(x - offset)`. The native importer preserves the original float32 multiplier
instead of computing a reciprocal. Imported imputation uses `Constant` because
the ONNX graph records the learned replacement without proving whether training
used a mean, median or another strategy.

The [sklearn-ONNX complex pipeline example](https://onnx.ai/sklearn-onnx/auto_examples/plot_complex_pipeline.html)
is useful for input typing and branch structure, but its string-imputation
caveat does not describe the behavior of the pinned converter tested here.
Actual generated graphs are the acceptance evidence.

## Early stopping is observable prediction state

All three XGBoost fixtures save four boosting rounds, while `best_iteration=0`.
The deliberately mismatched validation labels make this edge case deterministic
and visible; it is a semantics test, not an example of useful model training.
Reloading the file and predicting with `iteration_range=(0, 1)` matches the
sklearn estimator. Predicting with all saved rounds differs by approximately
0.6212 for regression, 0.1812 for binary probabilities, and 0.2232 for multiclass
probabilities. Import must preserve the half-open round range or export a
properly sliced booster. It must not confuse boosting rounds with tree count
in multiclass models.

The [XGBoost prediction documentation](https://xgboost.readthedocs.io/en/stable/prediction.html)
describes the different wrapper/native defaults. The generated files retain the
original booster plus a separate prediction policy. They are bare numerical
boosters; external preprocessing or label encoders are not implicitly included.
Non-numeric business labels require a separate recorded mapping even when the
booster predicts numeric class indices.

## What follows before shipping import

1. Extend the now-tested strict graph-to-plan translation beyond the exact
   two-numeric/one-string motif only when additional fixtures earn support.
   The bounded implementation checks actual protobuf attributes, initializers,
   tensor types/shapes, opset/domain, wiring and post-transforms. Sidecars are
   expected results, never replacements for parsing the source model.
2. Add supported ONNX tree regression/classification fixtures with split-boundary
   and missing-value probes. Run the native tree executor and ORT on the *same*
   artifacts. Measure correctness work and packaging cost; these linear fixture
   passes provide no evidence that ORT is necessary for trees.
3. Implement XGBoost gbtree JSON translation with tested base-score/objective,
   default missing branch, class grouping, and iteration-range semantics.
   Explicitly test rejection of dart, gblinear, categorical splits until supported,
   unsupported versions/objectives, and malformed artifacts. None of those
   rejection paths exists or has passed yet.
4. Bound byte size, node/tree counts, depth, tensor allocation and scoring work.
   Prove failures before allocating or executing unbounded work. Add structural
   rejection fixtures rather than trusting only four probe rows.
5. Test import mapping, persistence/reopen and inference without Python through
   the production engine and native e2e lane. Cross-platform bundling remains a
   separate spike; local Python wheel loading does not prove application packaging.

No application dependency, public operation, UI, or release behavior changes
in this spike. The full compatibility gate remains open.

## Native linear translation: executed second slice

`tools/ml-spikes/onnx` is an isolated Rust crate using pinned prost 0.13.5 and
the shared contract crate. It imports the actual ONNX files; a test renames
graph values and changes an intercept to prove predictions follow the artifact,
not reference JSON. Eight tests pass, including native-versus-sklearn and
native-versus-recorded-ORT parity, typed labels/probabilities, explicit missing
sentinels, float32 multiply scaling, strict unsupported/malformed rejection,
all truncations of a fixture, sampled byte mutations, and scoring bounds.
`cargo fmt --check` and strict Clippy pass for the isolated crate.

The parser first runs an allocation-free wire preflight with a 64 KiB byte cap,
4,096-field/packed-element budget, depth 8 and string length 1,024, rejecting
unknown schema fields before prost can silently skip them. Semantic validation
then accepts exactly the observed 14-node motif, at most 32 categories, and
small fixed int64 initializers. Inference is bounded to 10,000 rows per call.
No allocation uses untrusted tensor dimensions. External tensor data, nested
graphs, arbitrary Where branches, trees, alternate operator/opset layouts,
and multiclass classifiers remain unsupported. Full details and reproduction
commands are in the [native spike README](../tools/ml-spikes/onnx/README.md).
