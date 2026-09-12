# Production ML numerical runtime

`crates/framework-ml` is the application-owned reusable numerical runtime. It has no
document IDs, view state, Python runtime requirement or path dependency on a spike.
The core binds stable feature-column IDs to its ordered numeric matrix; names in the
runtime are display/provenance metadata.

Public functions: `fit`, `predict`, `predict_columns`, `output_names`, `evaluate`,
`validate_fitted`, `import_xgboost`, `import_xgboost_with_range`, `import_onnx`,
`predict_scalars` (also `predict_values`), `input_types`, `output_types`, and
`statistics`. Persisted payloads
are typed Rust enums/structs with serde and exported TypeScript. Original fitted
statistics are saved; evaluation on another labeled dataset is separate. Baselines
are learned from training targets and retained in the fitted model.

## Implemented methods

- OLS with intercept, classical or HC3 covariance, Student-t inference and compact
  coefficient rows. Aliased terms are represented as unavailable (`None`), never
  serialized NaNs; their identifiable predictions remain available.
- Unpenalized binary logit with intercept, finite numeric targets 0/1, model-based
  SE, named z statistics and normal-Wald intervals. Exact rational certificates
  reject complete/quasi separation before calling anofox. Uncertifiable cases,
  numerical failure and exhausted budgets return errors. No coefficient cutoff is
  used. A tested 600-row fit removes the original spike's 256-row cap.
- Numerical XGBoost 3.0.x `gbtree` JSON for squared-error regression, binary logistic,
  and multiclass soft probabilities. Stored `best_iteration` is preserved; a caller
  may explicitly override the iteration range. Categorical splits, other boosters,
  vector leaves and unsupported versions/objectives are rejected. Native fitting
  through `Method::Xgboost` uses pinned xgb 3.0.6 on macOS/Windows, with explicit
  regression/binary/multiclass objectives. It serializes the booster in memory and
  converts it to the same validated typed tree payload used by imported inference.
  Defaults are 100 rounds, depth 6, learning rate 0.1, full row/column sampling,
  L2 1, child weight 1, seed 0, and one thread. Training is bounded to 1,000 rounds,
  depth 16, 16 threads, and 500 million row-by-feature-by-round work units.

- Seeded random forest regression and classification using Smartcore 0.6.14.
  Settings default to 100 trees, depth 8, minimum leaf size 2, seed 0, and
  `floor(sqrt(p))` candidate features per split. Classification requires contiguous
  integer labels `0..K-1`. Probabilities are tree-vote fractions, not calibrated
  probabilities. Private backend state is converted into validated typed trees;
  no opaque Smartcore model blob is persisted.
- Imported ONNX fitted preprocessing plus linear/logistic prediction for the
  checked branched motif: two numeric inputs with imputation/scaling, one string
  input with imputation/one-hot encoding. This is an explicit translator, not an
  arbitrary ONNX runtime. Typed scoring preserves string/integer class labels and
  emits probabilities in the saved label order. Numeric missing values use the
  saved recipe; missing strings are refused by the document boundary.
- Standalone mean Student-t confidence intervals, Pearson correlation with a
  two-sided t test and Fisher-z interval, and Welch two-sample mean differences.
  Inputs are finite numeric samples; paired Pearson inputs must align. Degenerate
  samples produce explicit errors or documented unavailable statistics.

Scoring gives numerical responses or predicted class indices plus probabilities
ordered by class index. Exact binary ties choose class 0. Numeric missing values
are rejected for OLS/logistic/forests; XGBoost treats NaN as missing and follows its saved
missing branch. Infinite or float32-overflowing XGBoost inputs are rejected.

Imported models carry no fabricated training statistics or baseline. Persisted tree
payloads are revalidated before use, including topology and indexes. JSON parsing
uses serde_json's float-roundtrip feature to preserve stored fitted f64 values.

## Resource and inference boundaries

The generic fitting matrix budget is five million numeric cells. Logistic exact
separation additionally checks arithmetic work `n * (p+1)^2 <= 2,000,000` and uses
bounded LP solves. These are explicit resource errors, not silent sampling. They are
conservative protection pending broader measured scaling, not a row-count claim.
Exact certificate reconstruction is dimension-bounded but still needs integration
with a cancellable training job for a hard end-to-end deadline.

Forest training additionally bounds `rows * features * trees <= 50,000,000`,
with 1–1,000 trees and depth 1–24. Standalone statistics accept at most two million
combined sample values. ONNX import is limited to 64 KiB and scoring to 10,000
rows per call. Limits return explicit errors; no rows are silently discarded.

Marginal SE/intervals are retained. Full covariance, contrasts and prediction
intervals are not yet advertised. The current fitting surface requires finite
numeric data and an intercept; missing-value recipes and intercept choices need
additional implementation and acceptance for native training. Imported ONNX
pipelines already own their fitted typed recipe and use `predict_scalars`; the
numeric-only `predict` entry point deliberately refuses those pipelines.
Arbitrary ONNX graphs and arbitrary sklearn pickle/joblib loading
are outside this implementation. Import supported sklearn pipelines through ONNX.

`FitRequest` carries method, ordered feature names, target name, covariance,
confidence level, iteration budget, and defaulted forest/XGBoost settings. `FittedModel`
retains its typed payload, summary, training mean/prevalence and multiclass class
frequencies. `evaluate` accepts a separate numeric assessment dataset and compares
against those saved training baselines. The document layer owns deterministic
splitting and stable column-ID binding; the runtime does not split data implicitly.

## Verification

`cargo test -p framework-ml` runs independent statsmodels/HiGHS/XGBoost reference
fixtures, independent SciPy standalone-statistics references, Smartcore forest
parity, typed ONNX/sklearn/ORT parity, persistence/corruption rejection,
invalid-input/separation checks and TS exports. The settled gate passed 49 unit
and binding tests plus 16 integration tests (65 total, none ignored) using
`cargo test -p framework-ml --offline`. Strict Clippy also passed with
`cargo clippy -p framework-ml --all-targets --offline -- -D warnings`.
`cargo clippy -p framework-ml --all-targets -- -D warnings` and package formatting
are required. Core history/persistence and native UI seam tests are separate lanes;
this crate's tests do not claim that the model card is wired correctly.
