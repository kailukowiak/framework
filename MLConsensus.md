# ML in FrameWork — consensus

Reconciliation of two proposals for tabular ML and statistics in FrameWork:

- `AstraSuggestedML.md` — product scope: what and why. No crates named, nothing verified against source.
- `docs/claude-ml-suggested.md` — implementation plan: how and with what. Crate survey verified against source on 2026-09-11.

Both refine ProjectSpec §34 (2026-08-13). Each model's position lives under its own subheader so the record shows who said what. The current working agreement is in the joint plan and its decision-status table; earlier positions remain as discussion history. Kai has confirmed those statuses and requested inspiration from R's tidymodels. ProjectSpec.md still needs to be amended before implementation treats this document as its replacement.

Kai subsequently authorized starting the build with subagents. The bounded first
batch, ownership, executable spikes, findings, and remaining integration gates
are tracked in [ML build lanes](docs/ml-build-plan.md). Historical "when Kai says
go" wording below predates that authorization. Spike results are evidence, not
an announcement that model cards or inference have shipped.

## Fable

### Reading of the two proposals

They are complementary, not competing. They agree on every structural point: one model object for native and imported models; fitted state (scalers, encoders, imputers) belongs to the model, not the wrangle chain — being visible in the chain does not prevent leakage; fitting is a deliberate action, inference is live, staleness is visible and never silently repaired; cards are Excel-dense; pickles are never accepted; a prediction artifact cannot carry a complete statistical analysis, so coefficients, covariance and diagnostics need their own persisted representation.

The proposals differ in altitude, and in three places of substance, recorded below.

### Adopted from Astra

Seven points where Astra's document is better than the implementation plan. All are folded into the reconciled plan.

1. **Fit settings are separate from inference settings.** "Robust" means two things. HC3 covariance changes the uncertainty on the same OLS fit; Huber changes the fit. The UI names the method precisely — never a generic "robust" switch. Covariance method (classical, HC0–HC3; clustered and HAC/Newey–West as follow-ons with explicit group/order/lag settings) is an inference setting on the linear card. anofox-regression already exposes `HcType` and a robust-covariance module, so this is nearly free.
2. **Outputs vary by model.** The contract carries `Predictions | Probabilities | Assignments | Components | TransformedFrame` from day one, even though v1 ships only the first two. k-means and PCA are then additions, not retrofits.
3. **Evaluation rigor.** A random 20 % holdout leaks on time-ordered data. The train dialog offers split by fraction, by time column, or by group column. Early-stopping validation is not the untouched test set. Every predictive model reports against a **baseline** (predict-the-mean / majority class): "beats always-guessing-the-average by X" is the novice's first real diagnostic.
4. **Task-first entry points.** "Predict a number", "predict a category", "find groups", "compare groups" — method shown alongside the task, sensible defaults, deeper settings on demand, failures as ordinary inline text.
5. **Compatibility contract and export helper for imports.** A published matrix of supported transformers, estimators, versions, input types and outputs; a small Python helper `framework_export(pipe, X_sample)` that writes the artifact plus a sidecar (feature names, versions, sample inputs and expected outputs for parity checks) and fails early, by name, on unsupported steps. skl2onnx's `initial_types` is where novices get stuck; the helper removes it.
6. **Explicit intercept and missing-value policies** on the linear family. anofox has R-style `na_action`.
7. **Stats outputs are frames.** Coefficient tables and test results are referenceable from Scratchwork and usable as frames downstream. Detailed diagnostics and plots open from the summary.

### Where Fable differs

**1. "Actual XGBoost" — right principle, wrong to pre-commit.** Astra is correct that a different booster must never be *presented* as XGBoost. But "investigate and accept the native dependency" reopens a question the decision log closed, without costing it. Costed on 2026-09-11:

- The maintained binding is `xgb` 3.0.6 (2026-08-17, tracks XGBoost 3.0). It ships **prebuilt dynamic libraries** for macOS arm64, Linux x64 and Windows x64 — no Intel Mac — and on macOS needs `libomp` at runtime. Local compilation needs CMake + Ninja and is "possible but manual" on Windows.
- The bill: bundle `libxgboost` and `libomp` in the .app with rpath fixup, sign and notarize both; ship the DLL plus the MSVC OpenMP redistributable on Windows; add the `.so` and libgomp to the Flatpak manifest. Roughly 2–4 days across three platforms plus permanent CI surface.
- That is materially cheaper than the decision log assumed (a C++ toolchain on every build machine). It is a legitimate **spike**, not a rejection — but it is still days of packaging for a name.

Position: v1 ships **perpetual, labelled "Gradient boosting"** — one tuning knob, calibrated probabilities, conformal prediction intervals, per-row contributions, native categoricals and missing values; for novices this is a better GBM than forty hyperparameters — plus **real XGBoost import from `model.json`** with exact parity, so the model trained in Python runs in the document as itself. A time-boxed `xgb` packaging spike follows *if* in-app real-XGBoost training is demanded. Astra's naming principle is satisfied; the packaging cost is deferred until earned.

**2. "Prove ONNX before committing" — already proved, in the negative.** tract's `ai.onnx.ml` registry (read from source) has exactly CategoryMapper, LinearClassifier, LinearRegressor, Normalizer, TreeEnsembleClassifier. No `Scaler`, no `TreeEnsembleRegressor`, no opset-5 `TreeEnsemble`, no `ZipMap`. A scaled sklearn pipeline or any tree regressor will not run. Resolution: **parse the skl2onnx graph into typed fitted parameters; never execute it.** ONNX remains the sklearn interchange *format*; tract is a later fallback for graphs that don't decompose, not a v1 dependency; `ort` is not needed.

**3. Sequencing.** Astra: linear/logistic + cards → XGBoost → import. Fable: parser spike → object + import → linear → GBM. Astra's order matches Kai's stated priorities; Fable's front-loads the only genuine unknown, and Astra's own closing paragraph — design the artifact and preprocessing contracts in the first slice even though import ships later — is the argument for it: the contract can't be designed right without having lifted a real skl2onnx graph into it. Not a hill to die on. The parser spike is three or four days of tests-only work in the core crate and can run in parallel with the linear card.

### Missing from both, now included

- **Trend lines on plots** — Kai's original "critical". A trend line is a *view* (recomputes freely, no identity); a model is a *fitted artifact* (identity, staleness). Same math, different lifecycle; never unify them. Ships with the linear milestone since anofox supplies lm / LOWESS / quantile with SE and prediction bands.
- **Classification column shape**: a probability column and a predicted-class column; threshold is an inference setting.
- **Replication under collaboration**: the fitted result replicates, never the training command. Training is nondeterministic across machines.

### Reconciled plan

> **Superseded.** This was Fable's first synthesis, written before Astra's response. The object sketch below is kept for the record but is wrong in the ways conceded in [Fable — response to Astra](#fable--response-to-astra): the per-column `transform` is too thin (it must be an ordered fitted plan), `outputs` is an exclusive enum (it must be a collection), `threshold` sits under `inference` (it belongs under `prediction`), and the milestones are PR-counted rather than evidence-gated. The working plan is the [Joint implementation plan](#joint-implementation-plan--draft-for-reconciliation) with that response folded in.

#### The object

```
ModelObject {
  id, name,
  spec: {
    source_frame_id,
    features: [{ column_id, name,
                 transform: None | Scale { offset, scale } | OneHot { levels } | Impute { value } }],
    target: Option<column_id>,
    weights: Option<column_id>, groups: Option<column_id>, time: Option<column_id>,
    missing_policy: Omit | Fail | Impute,
    intercept: bool
  },
  fit: {
    estimator: Linear | Logistic | Ridge | Lasso | Huber | Quantile | Poisson
             | GradientBoosting | RandomForest | DecisionTree | KMeans | Pca,
    settings: estimator-specific (regularization, quantile, budget, depth, k, …),
    split: Fraction { holdout, seed } | ByTime { column, cutoff } | ByGroup { column, holdout_groups },
    seed
  },
  inference: {                                  // only what the estimator supports
    covariance: Classical | HC0 | HC1 | HC2 | HC3 | Clustered {…} | HAC {…},
    confidence_level, threshold
  },
  params: Linear { coef, intercept }
        | Glm { coef, intercept, link }
        | TreeEnsemble { trees, base_score, link }     // imported XGBoost / sklearn trees
        | Perpetual { booster_json }                   // native GBM, kept native for intervals/contributions
        | Forest { … } | Tree { … } | Centroids { … } | Components { … },
  outputs: Predictions | Probabilities | Assignments | Components | TransformedFrame,
  results: {
    coefficients: Option<[{ term, estimate, se, t, p, ci_lo, ci_hi }]>,
    importance:   Option<[{ column_id, score }]>,
    metrics: { n_train, n_holdout, baseline: {…},
               holdout: { r2, rmse, mae } | { accuracy, auc, confusion } },
    diagnostics: Option<{ vif, leverage, cooks, condition_number }>
  },
  provenance: { trained_at, lineage_fingerprint, backend, backend_version,
                source: Native | Imported { artifact, format } },
  status: Fitted | Stale | Failed { message }
}
```

- Feature columns by stable ID; lineage cords from the card to every input.
- Results are **fitted values** — computed once at training time and stored, like weights. Never recomputed.
- `predict(model)` is a `WithColumns` step. Linear family compiles to a Polars expression and renders as a readable formula; `TreeEnsemble` is an engine tree-walk; `Perpetual` calls `predict_columnar`.
- Imported models are predict-only and the card says so. Native models get **Retrain** beside the staleness badge — deliberate, never automatic.
- `DataObject::Model`, boxed for the `large_enum_variant` lint. `ReplicatedOperation` carries the fitted result.

#### Cards

**Model card.** Header: task · estimator · target · n · baseline vs holdout metrics · staleness badge · Retrain (native) or source file + "predict only" (imported). Body by output type: coefficient table with significance stars (`term | estimate | SE | t | p | ***`) for the linear family, with the covariance method named; feature-importance bars for trees and forests; centroid table for k-means; explained-variance table for PCA. Twenty coefficients look like twenty spreadsheet rows.

**Stats output card.** A coefficient table, a test result, or an evaluation summary. Numbers referenceable from Scratchwork; tabular outputs usable as frames; diagnostics and plots open from it.

#### Entry points

Task first, method alongside:

| task | v1 methods | default |
|---|---|---|
| Predict a number | Linear · Ridge · Lasso · Huber · Quantile · Poisson · Gradient boosting · Random forest · Decision tree | Linear if ≤ a few features, else Gradient boosting |
| Predict a category | Logistic · Gradient boosting · Random forest · Decision tree | Logistic / Gradient boosting, same rule |
| Find groups | k-means | k chosen by elbow, shown |
| Reduce dimensions | PCA | components to 90 % variance |
| Compare groups | t-test (one-sample, two-sample, Welch, paired) · chi-square · one-way ANOVA | — |
| Measure association | correlation matrix (Pearson / Spearman) with p-values · CI on a mean | — |

Deferred: SVM, kNN, forecasting (series-shaped; its own feature), ONNX export, LightGBM text import, tract fallback, in-app real XGBoost (see open decision).

#### Imports

- **ONNX from skl2onnx.** Match `[Concat] → [Scaler | OneHotEncoder | Imputer] → LinearRegressor | LinearClassifier | TreeEnsembleRegressor | TreeEnsembleClassifier | TreeEnsemble → [ZipMap | Cast]`. Anything else is rejected naming the op. Protobuf via `prost` + the ONNX `.proto`.
- **XGBoost `save_model("m.json")`.** `learner.gradient_booster.model.trees[]`, `base_score`, `objective`, `feature_names`. Regressor, `binary:logistic`, `multi:softprob`.
- **Export helper** `framework_export(pipe, X_sample)` in Python: writes artifact + sidecar (feature names, library versions, sample inputs, expected outputs); fails by name on unsupported steps. The sidecar drives the parity test on import.
- **Compatibility matrix** published in docs, tested in CI against fixtures.
- Import dialog maps file feature names → columns: exact, then fuzzy, user confirms.

#### Milestones — one PR each

1. **Parsers + predict, core crate, tests only.** Fixtures: `Pipeline(StandardScaler, LinearRegression)`, `Pipeline(OneHotEncoder, GradientBoostingRegressor)`, an XGBoost binary classifier. Parity to 1e-6. Builds the tree-walk and linear predict everything reuses. Can run in parallel with (2).
2. **Linear + logistic via anofox, the object contract, both cards, live downstream inference.** Coefficient table with stars, covariance setting, missing policy, split spec, baseline. **Trend-line plot layer** (lm / LOWESS / quantile with bands) rides here.
3. **`ImportModel` op + import dialog + export helper + compatibility matrix.**
4. **Gradient boosting (perpetual) + random forest / decision tree (smartcore)** with importance, contributions, calibrated probabilities, intervals; time/group splits; early-stopping validation distinct from test.
5. **Ridge / Lasso / Huber / Quantile / Poisson** (cheap once 2 exists) · **k-means / PCA** · **Compare-groups and association stats.**

Later, on demand: `xgb` packaging spike, ONNX export, LightGBM, tract, forecasting.

#### Dependency footprint

`anofox-regression` (pinned; acceptance-tested against R/statsmodels; vendored if it goes stale), `perpetual`, `smartcore`, `prost` + ONNX proto. No Python, no C++, no BLAS. `to_ndarray` is not needed.

### Open decision for Kai

**Is in-app real-XGBoost training worth the packaging spike in v1, or does honest "Gradient boosting" (perpetual) plus real-XGBoost import cover it?** Everything else above is reconciled.

### Verified facts this rests on

Read from source on 2026-09-11; re-verify before adding any of these as dependencies.

- tract `onnx/src/ops/ml/mod.rs` registers five ops; no Scaler / TreeEnsembleRegressor / TreeEnsemble / ZipMap.
- anofox-regression 0.5.13 (MIT, one author, pushed 2026-08-01): OLS via column-pivoted QR, aliased → NaN, HC covariance, prediction intervals, GLMs, Huber/quantile/LOWESS, VIF/leverage/Cook's; faer + statrs + argmin; no BLAS; no serde.
- perpetual 3.0.0-rc.2 (2.1.0 stable): regression + classification, `predict_proba`, `predict_intervals`, `predict_contributions`, `calculate_feature_importance`, `ColumnarMatrix`, serde, `set_seed`; deps rayon/serde/rand/sysinfo.
- smartcore 0.6.14 (2026-08-26): RF, extra trees, decision tree, k-means, PCA, LDA, scalers, serde; its `xgboost` module is a regressor-only file.
- linfa 0.8.1: BLAS-free, has RF/AdaBoost/LARS; no inference, no robust/quantile, no GBM. Not the foundation.
- `xgb` 3.0.6 (2026-08-17): prebuilt dynamic libs for macOS arm64 / Linux x64 / Windows x64; needs libomp on macOS; no Intel Mac.
- XGBoost's `save_model` JSON is a stable documented schema.

## Astra

### Position

The product direction is substantially shared. The implementation is not yet
consensus. In particular, "everything else above is reconciled" overstates the
agreement: rejecting tract for a tested subset does not settle the runtime
choice, substituting Perpetual does not settle XGBoost, and the proposed object
still cannot express several ordinary fitted pipelines. This section is my
response to Fable's position, not an assertion that Fable has accepted it.

I accept Fable's stronger sequencing argument: prove representative imports
before locking the model contract. I also accept typed native fitted state,
replicating fitted results rather than training commands, and using ordinary
frames for statistical outputs. My original implementation order put the
import uncertainty too late.

I do not independently endorse the crate versions, feature claims, or packaging
estimates above. They are Fable's reported survey results, useful leads to
re-verify against pinned releases and executable acceptance cases.

### Where I push back

**1. XGBoost is not merely packaging "for a name."** Kai specifically singled
out XGBoost. Its implementation, behavior, ecosystem, and compatibility are
part of that request; an alternative with a convenient tuning parameter is not
automatically equivalent. Conversely, I should not have treated accepting an
unknown packaging bill as already decided. Keep actual XGBoost training in the
target scope and investigate its cost early. If we defer it, call that a scope
decision explicitly. Perpetual can earn a place as another estimator, but should
not win by treating native packaging cost as disqualifying before measuring it.
Nor does exposing XGBoost require exposing forty parameters: novice defaults
and progressive disclosure are interface choices available to either backend.

**2. A tract limitation is not proof that a custom importer is the best runtime.**
The proposed translation still implements execution semantics for every accepted
operator and model. We must own shape, dtype, category, missing-value, class-label,
and post-transform behavior, not just protobuf decoding. sklearn pipelines can
branch through ColumnTransformer and compose multiple transforms. Compare a
strict native translator with ONNX Runtime on the same fixtures and packaging
targets. Neither has won yet. "Parse, don't execute" is a description of an
implementation technique, not a reduction of the correctness obligation.

**3. A model format being documented does not make its inference trivial.**
XGBoost import needs a versioned support contract for booster type, objective,
base score, missing branches, categorical splits, class ordering, and prediction
iteration range. In particular, an early-stopped estimator's usual predictions
may use fewer trees than the saved booster contains. Either preserve that
prediction policy or have the export helper produce the intended sliced model.
Unsupported cases must be rejected, not approximated as ordinary numerical
trees. A raw XGBoost model file also does not contain an external sklearn
preprocessing pipeline; distinguish booster import from whole-pipeline import.

**4. The feature-transform enum is too small.** A feature can be imputed and
scaled, or imputed and one-hot encoded. Column groups may take separate branches
before concatenation. We need an ordered, typed fitted transformation plan with
explicit input/output mappings, not one optional transform per source column.
Model parameters bind to transformed features; lineage binds back to raw inputs.
Persist category order, unknown-category policy, output order, and dtypes.

**5. Saved fit results and new evaluations have different lifecycles.** Save the
original fit summary immutably. Coefficient inference comes from the fitted
training model, not the holdout. Let users evaluate that saved model against a
new labeled frame without retraining it. Changing confidence level or supported
covariance settings can require a new statistical summary; "never recomputed"
must not become a prohibition on that operation. Declare what retained state
each inference operation needs, and report when it is unavailable. Imported
prediction-only models need not have training counts, targets, or diagnostics.

**6. Statistical correctness is not nearly free.** A covariance API is useful,
but missing observations, rank deficiency, perfect logistic separation, failed
convergence, small groups, and unavailable degrees of freedom still need honest
behavior. Store the statistic's type (for example t or z), covariance method,
confidence interval, and relevant degrees of freedom. Significance stars may
be shown as a secondary convention with a defined legend; they must not replace
effect sizes and uncertainty. Do not produce classical OLS-looking p-values for
regularized estimators merely to fill a common table.

**7. The novice defaults need more thought.** Feature count alone is not a sound
rule for choosing linear models versus boosting. Automatically selecting every
column invites IDs and outcome leakage into the model. Suggest features for
review, show a simple baseline, and make the evaluation split explicit. Keep
training, tuning/early-stopping validation, calibration when used, and final
test roles separate. Regression means and classification baselines must be
learned from training data. A claim about calibrated probabilities or interval
coverage needs an evaluation design, not just a method with that name.

### Additional contract corrections

- A model can expose several outputs simultaneously. Use a collection of typed
  output descriptors, not an exclusive `Predictions | Probabilities` choice.
- Binary thresholds are prediction settings; they are not covariance settings.
  Multiclass outputs also need the class-label-to-probability-column mapping.
- Separate an immutable fitted revision from an attempted retraining job. A
  failed retrain must not destroy the last usable fit. Record the input revision
  used by the job and detect changes while it runs.
- Preserve training provenance separately from the frame currently receiving
  predictions. Imported models do not become stale merely because new scoring
  rows arrive; their original training provenance may be unknown.
- Large fitted payloads belong in versioned, content-addressed artifacts. Small
  metadata and result references belong on the model object. Collaborators need
  artifact availability as well as the replicated reference.
- Plot trend lines are a useful separate lifecycle, but this conversation does
  not establish them as a prerequisite for ML. Track that feature separately
  rather than expanding the first regression milestone automatically.

### Evidence to use in the spikes

The following primary references support compatibility questions, not a claim
that any proposed importer has already passed them:

- [sklearn-ONNX: a pipeline with ColumnTransformer](https://onnx.ai/sklearn-onnx/auto_examples/plot_complex_pipeline.html).
- [XGBoost: model IO and its documented schema](https://xgboost.readthedocs.io/en/stable/tutorials/saving_model.html).
- [XGBoost: estimator interface and early stopping](https://xgboost.readthedocs.io/en/release_3.3.0/python/sklearn_estimator.html).
- [ONNX: TreeEnsembleRegressor semantics](https://onnx.ai/onnx/operators/onnx_aionnxml_TreeEnsembleRegressor.html).
- [Statsmodels: robust covariance results](https://www.statsmodels.org/stable/generated/statsmodels.regression.linear_model.OLSResults.get_robustcov_results.html).

## Joint implementation plan — draft for reconciliation

This working synthesis incorporates both responses and Kai's confirmed decision
statuses. Historical objections above and below are retained for attribution;
the current status table here governs the plan where those passages disagree.
Milestones describe verifiable outcomes; they are not promises of one PR or a
fixed number of days. Amend ProjectSpec after the relevant decisions are proved.

### Design inspiration: tidymodels

Take inspiration from tidymodels' separation of responsibilities and consistent
outputs. FrameWork implements these concepts through its own document, engine,
and compact interface; this does not add an R runtime, R syntax, or R-model import
to the scope. The relationship between recipe, model specification, and workflow
is the particularly useful precedent. [Tidymodels recipes and workflows](https://www.tidymodels.org/start/recipes/)

| Tidymodels concept | FrameWork interpretation |
|---|---|
| parsnip: model specification, mode, engine | Separate method, regression/classification task, and backend. Show actual engine identity; keep settings consistent where semantics agree. |
| recipes: preprocessing specification and learned steps | A saveable ordered recipe inside the model, with column roles, branching, and explicit transformed-feature mappings. |
| workflows: preprocessing plus model | One model object owns the recipe and estimator specification; a fitted revision contains their learned state together. |
| rsample: splits and resamples | A recorded evaluation plan, separate from the fitted model, with explicit training/validation/test membership. |
| yardstick: performance metrics | Typed evaluation frames with consistent metric definitions, dataset roles, and classification event labels. |
| tune: tuning over resamples | A future explicit, bounded search job over whole workflows, producing comparison results before a final fit. |
| broom: tidy, glance, augment | Component tables, model summaries, and observation-level output frames, respectively. |

The specification/engine distinction follows [parsnip](https://parsnip.tidymodels.org/);
the evaluation and tuning divisions follow the [tidymodels package roles](https://www.tidymodels.org/packages/).
An engine adapter declares supported settings and outputs. Changing engines
invalidates a fit and may change available capabilities; a common interface
does not claim identical estimators or silently transfer incompatible settings.

**Recipe before fit, learned recipe after fit.** Keep the editable recipe distinct
from its fitted steps. A fit learns preprocessing from its training partition,
then applies those saved steps to new inputs. That is the useful `prep()`/`bake()`
distinction; users need not learn those verbs. Learned state does not refresh
with the scoring data. [recipes preparation](https://recipes.tidymodels.org/reference/prep.html)

Record column roles such as predictor, outcome, ID, weight, group, and time.
Only predictors enter the feature matrix. A scoring frame needs the predictor
schema, not an outcome column; residuals and supervised evaluation require an
outcome. General source wrangling remains upstream. Model-specific fitted
preprocessing is owned by the model even if its editor reuses Wrangle components.

**Evaluate the entire workflow.** When resampling or tuning is introduced, fit
the recipe independently inside each training fold. Do not fit a scaler,
imputer, encoder, or feature selector on the full dataset before cross-validation.
Compare candidates on the same recorded splits and metric definitions; keep the
final test data out of selection. A selected recipe/model specification still
needs an explicit final fit: a resampling result is not a deployable fitted model.
This follows the workflow-based [resampling](https://tune.tidymodels.org/reference/fit_resamples.html)
and [tuning](https://tune.tidymodels.org/reference/tune_grid.html) interfaces.

**Use tidy outputs as the card contract.** Adopt broom's three result shapes:
component rows (coefficients or loadings), one model-level summary row, and
observation-level additions (predictions, probabilities, available residuals).
These are ordinary referenceable frames. A card displays them rather than
computing a separate private summary. [broom result conventions](https://broom.tidymodels.org/)

Keep fit summaries distinct from evaluation frames. Evaluation rows identify the
fitted revision, dataset/split, metric, estimate, and relevant event class or
averaging method. Unavailable statistics remain explicitly unavailable, never
zero-filled. Preserve source-row alignment through missing-value handling;
observation-level output must not shift predictions onto a neighboring row.
This alignment does not grant positional cell-reference permission on live data.

For a novice, the flow remains compact: choose data and task → review features
and recipe → choose method → fit → inspect summary and use outputs. Recipe and
estimator live in one model card, with deeper settings available when needed.
Standalone statistical tests use the same tidy result frames without requiring
a predictive workflow. Cross-validation and tuning are later capabilities, not
new prerequisites for the first linear/logistic release; their lifecycle must
nevertheless fit the contract from the start.

### 1. Establish the shared product and compatibility contract

Carry forward the agreed purpose: above Excel, below R/Python; compact model
and stats output cards; referenceable results; deliberate fitting; live inference;
and supported Python pipeline import without Python on the recipient's machine.

Write the contract for raw inputs, composed fitted preprocessing, fitted model
revisions, typed outputs, statistical summaries, separate evaluation results,
and provenance. Allow optional targets and multiple outputs even while the first
implementation focuses on supervised models. Keep large payloads in artifacts.

Apply the tidymodels-inspired divisions above: distinguish unfitted recipe and
model specification from a fitted workflow revision, method from engine, and
fit summary from evaluation. Include input roles and the three tidy output shapes.

**Exit evidence:** example serialized contracts for a linear fit with HC3, a
mixed numeric/categorical pipeline, and an imported classifier with no training
data. Each must represent its feature mapping and lifecycle without invented
training metadata or mutually exclusive output restrictions. Include a schema
example for evaluating two fitted revisions on one recorded split, without
implementing a tuning system in this milestone.

### 2. Resolve the risky backend choices with bounded spikes

Use pinned dependencies and committed fixtures generated by the reference
libraries. Keep the fixtures small enough to inspect and regenerate. Python
may generate development fixtures; it is not an application runtime dependency.

The translator into the ordered typed plan is required for the supported import
surface regardless of the tree execution choice. The ONNX Runtime question is
now narrow: does implementing and maintaining the supported ONNX TreeEnsemble
execution semantics justify bundling ORT as an additional execution path? Compare
the native tree-walk and ORT on the same supported ONNX tree fixtures. Keep linear
and preprocessing translation in the acceptance suite either way. This decision
does not expand the accepted operator matrix or route unsupported graphs to ORT.

Use the following compatibility cases:

- Scaled linear regression and logistic regression, including class labels.
- Branched numeric imputation/scaling and categorical imputation/encoding joined
  into one estimator, including unknown-category behavior.
- Tree regression and classification with missing values and boundary cases.
- XGBoost numerical regression, binary and multiclass classification, and an
  early-stopped model. Test categorical splits if claimed supported; otherwise
  prove they fail with an actionable unsupported-case error.

Check feature order, output shape, labels, missing-value behavior, and numerical
predictions using documented dtype-appropriate absolute and relative tolerances.
Do not replace these checks with a universal 1e-6 threshold. Test rejection of
unsupported operators, attributes, versions, and malformed artifacts. Impose
bounded resource use when parsing and scoring imported models.

In the same discovery milestone, measure actual XGBoost integration on supported
shipping platforms: build/bundle, clean-machine load, signing where applicable,
and minimal fit/predict. Use pinned anofox-regression as the agreed statistical
backend, gated by acceptance tests against reference
linear/logistic outputs, robust covariance, and failure cases. Evaluate Perpetual
as a candidate on its own merits, not an assumed replacement.

**Exit evidence:** a compatibility matrix, executable parity tests, a packaging
report, and a short decision record on whether ORT's contribution to supported
tree inference earns its additional dependency. Record anofox acceptance failures
as blockers for the affected capability rather than substituting unverified
statistics. Decide in-app XGBoost explicitly from the evidence; record a deferral
as a scope cut. A pinned backend is a selection, not a claim that its acceptance
suite has already passed.

### 3. Ship one model path end to end

Implement the shared model object and artifact lifecycle with a small proven
linear/logistic surface. Provide explicit feature selection, intercept and
missing-value policies, supported covariance settings, and a documented split
strategy. Do not silently use random splitting for a requested time/group task;
implement the selected strategy or state it is not yet supported.

Render compact coefficients and evaluation summaries. Fit/retrain produces a
new fitted revision; predictions flow into frames and Scratchwork dependencies.
Persist the original fit summary and allow a separate evaluation against a new
labeled frame. Reuse the same result surface for standalone statistics later.

For the first OLS inference surface, retain the supported covariance results
computed at fit time and their method metadata. Confidence levels can be changed
from retained statistics using the declared interval method. Do not promise all
HC variants for every estimator or treat standard errors alone as sufficient for
contrasts or prediction intervals. Retain full covariance where those supported
operations need it; clustered/HAC methods require their own fit-time inputs and
acceptance tests. No training design matrix is retained by default.

**Exit evidence:** native e2e coverage of UI → training → rendered output → save
and reopen, plus changed scoring inputs updating predictions without retraining.
Prove staleness after training-source edits, undo/history behavior, and retention
of the previous fit after a failed retrain. Cover the public operations through
MCP's generated catalog and headless tests. A focused interaction test proves
editor behavior; Rust proves calculations; the e2e path proves their connection.

### 4. Ship supported pipeline and XGBoost import

Use the backend selected by the spike, with the published compatibility matrix
as the boundary. Provide explicit feature mapping and the Python export helper.
Store original source artifacts for provenance and preserve fitted preprocessing
and prediction policy. Imported models use the existing model card and prediction
path, with their actual capabilities shown.

The helper should write schema, library/export versions, and parity information.
Including real sample rows in a distributable artifact must be explicit; use
non-sensitive probes or local-only verification where practical. Sample parity
is a useful import check, not proof of general compatibility.

**Exit evidence:** export in Python, import into FrameWork, map features, and
verify predictions after reopening the document without Python installed. Test
reordered/renamed columns, unknown categories, missing inputs, and unsupported
pipelines. Clearly distinguish importing a bare XGBoost booster from importing
its surrounding fitted preprocessing pipeline.

### 5. Add strong boosting and the small model catalog

Deliver the resolved boosting choice under its actual name. Actual XGBoost
training remains the intended target unless explicitly deferred; Perpetual may
be offered as Gradient boosting if selected. Add random forest and decision
tree only after the shared evaluation and output paths are solid.

Preserve independent validation and test roles for tuning/early stopping. Show
held-out metrics alongside the baseline. Name the feature-importance method and
map transformed features honestly to their source columns. Treat calibration,
prediction intervals, and contributions as separately validated capabilities.

**Exit evidence:** seeded reference cases, appropriate split behavior, stored
preprocessing reused at inference, and the same persistence/history guarantees
as the first models. If XGBoost training is deferred, record the reason and the
user-visible scope reduction rather than silently checking off that requirement.

### 6. Broaden statistics and exploratory analysis incrementally

Add ridge/lasso, Huber, quantile and Poisson regression according to demonstrated
need and backend acceptance results. Add correlations, mean confidence intervals,
t-tests, chi-square independence, and one-way ANOVA as computations producing
ordinary result frames, displayed by the stats output card. Test their applicable
assumptions and degenerate inputs; do not budget them by formula length.

K-means and PCA follow through the already general output contract. Their cards
show assignments, components, and explained variance as appropriate. Do not
present automatic elbow selection or a variance cutoff as universally reliable
defaults. Keep forecasting and plot trend lines as separately scoped work.

**Exit evidence:** reference-backed numerical cases, meaningful invalid-input
behavior, and downstream use of the results through normal formulas and frames.

### Current decision statuses — confirmed by Kai

| Decision | Status | Remaining evidence or implementation work |
|---|---|---|
| In-app XGBoost | **Agreed:** measure in the spike and decide explicitly | Packaging and fit/predict evidence; record any deferral as a scope cut |
| ONNX inference | **Narrowed:** translator required regardless | Determine whether supported tree-walk semantics justify bundling ORT as an additional execution path |
| Statistical backend | **Agreed:** pinned anofox-regression | Acceptance tests against references, including robust covariance and failure cases |
| Initial model breadth | **Agreed:** shared path first | Linear/logistic, contract, both cards; expand incrementally |
| Fitted preprocessing | **Agreed:** ordered typed plan inside the model | Saveable recipe plus learned steps, proved against branched pipelines |
| Design inspiration | **Requested by Kai:** tidymodels | Specification/engine separation, recipes/workflows, explicit evaluation, and tidy result frames |

The confirmed statuses select a direction; they do not claim that spikes or
acceptance tests have already succeeded. The broad historical ORT disagreement
is narrowed as above. Remaining comparison work concerns its marginal value for
the supported tree executor, including correctness and maintenance cost.

For implementation, follow the repository's normal lint, test, e2e, and changelog
rules. This document itself is a design record, not an implementation or a release
announcement.

## Fable — response to Astra

Astra's response is accepted in substance. The joint implementation plan above is the working plan; what follows records what changed and the one point still held.

### Conceded

1. **Fitted preprocessing is an ordered, typed plan, not one transform per column.** `ColumnTransformer` with `Pipeline(Imputer, Scaler)` on numerics and `Pipeline(Imputer, OneHotEncoder)` on categoricals is the canonical sklearn shape; one-hot changes column count, so a per-column optional transform cannot express the output mapping. The model carries an ordered list of fitted steps, each with input column IDs → output feature slots, category order, unknown-category policy, output dtypes. Parameters bind to output slots; lineage binds to raw inputs. This is the "saveable pipeline" from Kai's first message — it lives *inside* the model object and is never a wrangle step.
2. **XGBoost JSON import needs a versioned support contract**: booster type (`gbtree` only at first; `dart`, `gblinear` rejected by name), objective, `base_score`, `default_left` missing branches, categorical splits (reject unless proved), class ordering, and **`best_iteration`** — an early-stopped model predicts with fewer trees than the file holds; preserve that policy or have the export helper slice. Bare booster import and whole-pipeline import are distinct operations with distinct cards.
3. **Coefficient inference comes from the training fit; holdout gives metrics.** The fit summary is immutable. Resolution for "never recomputed": **covariance method is a fit-time setting** — compute classical and HC0–HC3 at fit and store all of them — and **confidence level is display-time**, needing only stored SE and df. No design matrix is retained; a saved model can be evaluated against a new labelled frame without refitting.
4. **Regularized estimators get no p-value column.** Store statistic type (t or z), df, covariance method. Stars are secondary with a legend. Rank deficiency (aliased → NaN), perfect separation, non-convergence and small groups each have a named behaviour; the anofox acceptance suite covers failures, not just fits.
5. **Feature selection is suggest-and-confirm, never select-all.** Flag `unique_keys` columns, near-perfect target correlation, high-cardinality strings. No feature-count rule for choosing an estimator; the task view lists methods with one-line descriptions and the user picks. Baselines are learned from training rows.
6. **Contract corrections adopted wholesale**: a collection of typed output descriptors (probabilities and class simultaneously; multiclass label → column mapping); `prediction` settings (threshold, iteration range) separate from `inference` settings (covariance, confidence); immutable fitted revision separate from a retrain job that records its input revision and cannot destroy the last fit; training provenance separate from the scoring frame, so imported models do not go stale on new rows; large payloads (perpetual boosters, source ONNX bytes) in content-addressed artifacts with only references on the object; trend lines tracked as their own plot feature.
7. **XGBoost**: "for a name" was dismissive. Packaging cost is *measured* in the spike milestone on all three shipping platforms; perpetual is evaluated on its merits in the same milestone; neither wins before the evidence. If in-app XGBoost training is deferred, that is recorded as a scope decision with its user-visible reduction.
8. **Evidence-gated milestones replace PR-counted ones.** Dtype-appropriate tolerances, resource bounds on untrusted artifacts, explicit consent for sample rows in distributable artifacts.

### Held

**ONNX Runtime is additive, never a replacement.** Even with ORT, the graph must still be parsed to extract coefficients for the card and to render a linear model as a readable formula; ORT can only predict. A native tree-walk is required regardless for XGBoost JSON, which is not ONNX. ORT would therefore be a third inference path beside two that cannot be avoided, and its only marginal value is executing graphs outside the compatibility matrix — which the contract rejects by design. The spike should still put translator and ORT on identical fixtures; the question it answers is the narrow one: whether a tree-walk with documented `TreeEnsemble` semantics is hard enough to justify bundling a native runtime.

**The tract op list is a read of registry source, not a survey claim.** Crate *quality* claims (anofox in particular) need the acceptance tests Astra asks for; the tract registry contents are a different kind of fact and stand until the registry changes.

### Note on cost

The joint plan's first two milestones — contract with three serialized examples, then the four-way spike (translator vs ORT, `xgb` packaging, anofox acceptance, perpetual evaluation) — are the honest price of rigor and are roughly two to three weeks before a card is on the canvas. The spike's four legs are independent, tests-only work and should run in parallel.

### Status of the open decisions after this exchange

| decision | status |
|---|---|
| In-app XGBoost | **Agreed.** Measure packaging in the spike milestone on all three shipping platforms; perpetual evaluated on merits alongside; decide explicitly; a deferral is recorded as a scope cut with its user-visible reduction. |
| ONNX inference | **Narrowed, not closed.** The translator is required regardless (card display, formula rendering, XGBoost JSON). The spike answers only whether a native `TreeEnsemble` tree-walk is hard enough to justify bundling ONNX Runtime as an additional path. |
| Statistical backend | **Agreed.** anofox-regression pinned; acceptance tests against statsmodels/R covering fits, robust covariance, and failure cases (rank deficiency, separation, non-convergence). |
| Initial model breadth | **Agreed.** Shared path first (linear/logistic + contract + both cards); boosting next; catalog expands on demonstrated need. |
| Fitted preprocessing | **Agreed.** An ordered, typed fitted plan inside the model object, proved against branched `ColumnTransformer` pipelines. Never a wrangle step. |

Next action when Kai says go: milestone 1 of the joint plan — the contract, with the three serialized examples as exit evidence. Milestone 2's four spikes are independent and should run as parallel tasks.
