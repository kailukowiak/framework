# Native ONNX branched-linear spike

```sh
cargo test --manifest-path tools/ml-spikes/onnx/Cargo.toml --locked
cargo clippy --manifest-path tools/ml-spikes/onnx/Cargo.toml --all-targets --locked -- -D warnings
cargo fmt --manifest-path tools/ml-spikes/onnx/Cargo.toml --check
```

`import(&[u8])` reads actual ONNX protobuf bytes and returns an `ImportedLinear`.
Its `inputs()` exposes ordered named source inputs; `recipe()` returns the
shared contract's fitted recipe, while coefficients, intercepts and typed class
labels come from the source graph. `predict()` executes that fitted recipe and
the estimator with float32 arithmetic. Sidecars are used only by tests.

The accepted surface is intentionally **one exact 14-node motif**, ending in
either a single-output LinearRegressor or a binary LinearClassifier with two
opposite coefficient rows, string labels, and LOGISTIC post-transform:

- Two dynamic-row `[N, 1]` float32 inputs: concatenate → NaN impute → scale.
- One `[N, 1]` string input: the checked LabelEncoder/ArrayFeatureExtractor/
  Cast/Where imputation motif → one-hot → reshape.
- Concatenate numerical features before category slots, then the estimator.

Graph/value names and learned values can vary. Node order, wiring, type/shape
constraints, attribute sets, initializer layouts, domains and opsets must match
the checked motif. This supports the two real fixtures, not arbitrary Where
graphs, topological reorderings, all sklearn converter variants, ZipMap,
multiclass models or trees. The only accepted ONNX IR is 8, with core opset 18
and `ai.onnx.ml` opset 2. Metadata does not override graph semantics.

## Parser and resource contract

`schema/onnx-v1.18.0.proto` is copied from the pinned ONNX 1.18.0 wheel and
corresponds to [upstream's v1.18.0 schema](https://github.com/onnx/onnx/blob/v1.18.0/onnx/onnx.proto).
Its SHA-256 is `73ae5e7933e65bdb33dcf109161ea5c73f60b1a9ec93e997c7d11b8b13918ffe`.
The Apache-2.0 license is retained alongside it. `src/proto.rs` is a handwritten
prost projection with the upstream field numbers and wire types; it is not an
independent model contract. Shared fitted types come from `../contract`.

An allocation-free recursive wire preflight runs before prost, rejects unknown
fields, duplicate singular fields, incorrect wire types, truncated/overflowed
varints and lengths, and imposes limits: 64 KiB source, 4,096 total fields/packed
elements, nesting depth 8, and 1,024 bytes per string. It rejects all external
tensor storage, functions, subgraphs and other unrepresented schema fields.
This is intentionally stricter than the full ONNX schema.

After bounded decoding, semantic validation requires exactly 14 nodes, 3 inputs,
2 small int64 initializers, and 1–32 unique categories. It never allocates a
tensor from an artifact's declared dimensions. Scoring is limited to 10,000 rows
per call, with fixed-width inputs, bounded strings and finite numeric arithmetic;
NaN is accepted only as a raw numeric missing value. The caller should apply
the byte cap while reading a file as well, since this API receives existing bytes.

The generator in `../imports` records separate sklearn and ORT expected values.
Eight tests exercise both references, lowering semantics, parameter mutation and
renaming, malformed/unsupported artifacts and bounded input. They include all
truncations of a fixture and sampled bit mutations. These are regression tests,
not a substitute for fuzzing or a broad compatibility claim.

No ORT, Python or new dependency is linked into FrameWork. This isolated native
linear/preprocessing result does not settle the separate ONNX tree-runtime choice.
