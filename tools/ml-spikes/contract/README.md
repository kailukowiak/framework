# ML contract spike

An executable first draft of the model boundary in `MLConsensus.md`. It is an
isolated Rust crate so the import/backend evidence can change it before a new
persisted `.fw` object or public MCP operation is promised.

```sh
cargo test --locked --manifest-path tools/ml-spikes/contract/Cargo.toml
cargo clippy --locked --manifest-path tools/ml-spikes/contract/Cargo.toml --all-targets -- -D warnings
```

## What the examples establish

- `linear-hc3.json`: native source binding, immutable fit identity, and retained
  component/model summary artifact descriptors with explicit covariance method.
- `branched-pipeline.json`: independent numeric imputation/scaling and categorical
  imputation/one-hot branches, followed by an explicit final feature order.
- `imported-classifier.json`: inference without invented training provenance or
  an outcome, and simultaneous class/probability outputs with label mappings.
- `evaluation-comparison.json`: two fits evaluated on one recorded assessment
  membership, separate from either fitted model.

These are **synthetic contract examples**, not fitted statistical fixtures.
Artifact hashes are placeholders and no corresponding payload exists. Learned
numbers in the branching example illustrate shape only. No prediction parity,
statistical correctness, artifact availability, or production persistence is
claimed by these tests. The neighboring backend spikes supply numerical evidence.

Validation currently rejects role leakage into features, forward references,
overwritten slots, inappropriate transform input types, incomplete fitted state,
category/output width mismatch, and inconsistent classification label mappings.

Batch 2 adds shared import semantics used by the ONNX and XGBoost lanes:
explicit float32/float64 precision, numeric-versus-string class labels, missing
sentinel types, scaler multiplication/division, and half-open boosting iteration
ranges. Their serialization/validation tests do not by themselves prove that
an importer executes those semantics correctly; the consuming lanes test parity.

## Deliberate limits before production integration

This draft covers three estimator specifications and a narrow numeric/string
recipe subset. It is not the complete accepted import vocabulary. In particular:

- Learned category-dependent output topology is written explicitly in these
  examples. A production unfitted recipe needs selectors and generated outputs;
  this spike does not yet implement that binding/learning phase.
- The shared precision, missing-sentinel, class-label and prediction-policy types
  now represent the observed import requirements. Execution, dropped one-hot
  levels, numeric categories, and casts still need importer-specific support.
- The validator checks graph shape, not fit/specification fingerprint agreement,
  backend capability, artifact contents, or retraining-job lifecycle.
- Evaluation types demonstrate identity separation; they do not implement split
  generation, tuning, metric calculation, or validation of arbitrary comparisons.
- Statistical payloads, retained covariance matrices, confidence calculations,
  and source-row alignment are not implemented here.

Do not add this crate to the app workspace just to make it look integrated.
Promote the validated contract into framework-core with production validation,
artifact persistence, and public-operation coverage as one coherent slice.
