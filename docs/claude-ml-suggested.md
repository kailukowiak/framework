# Tabular ML in FrameWork — suggested plan

Drafted 2026-09-11 from a design conversation. This refines [ProjectSpec §34 "Machine learning on tabular data"](../ProjectSpec.md) (2026-08-13) in light of a crate survey done against source on that date. Where this document and §34 disagree, the disagreement is called out explicitly under [Deviations](#deviations-from-projectspec-34); nothing here is decided until it lands in the spec.

## Scope

The bar is *above Excel, below R/Python*. Novice users should get real answers with real diagnostics; data scientists should be able to bring their trained models in for inference. What's needed:

1. Half-decent linear and logistic regression with proper output — coefficient table, standard errors, p-values, CIs, fit metrics.
2. A short dropdown of general ML models.
3. Gradient boosting that is actually good.
4. Some non-predictive stats (correlation, t-test, chi-square, ANOVA).
5. Import of trained sklearn pipelines and XGBoost models for inference.

The deliverable is a **model card** on the canvas: R's `summary()` for someone who has never seen R, with significance stars.

## Crate survey (verified 2026-09-11)

All read from source on `main`, not from memory.

| crate | version | verdict |
|---|---|---|
| **anofox-regression** | 0.5.13, MIT, single author, created 2025-12, pushed 2026-08-01 | **Stats tier.** OLS/WLS/ridge/elastic net/Huber/Theil-Sen/RANSAC/quantile/LOWESS/isotonic/LARS; GLMs (logistic, Poisson, NB, binomial logit/probit/cloglog, Gamma, Tweedie); GLMM. Full inference: SE/t/p/CI/prediction intervals/HC covariance/F/AIC/BIC, plus VIF/leverage/Cook's. `ols.rs`: column-pivoted QR, aliased coefficients → NaN (R's `lm` behaviour), no `unsafe`. Deps: faer 0.23, statrs 0.18, argmin — pure Rust, no BLAS. No serde (irrelevant; we extract coefficients). Bus factor 1: pin, acceptance-test against R/statsmodels, vendor if it goes stale. |
| **perpetual** | 3.0.0-rc.2 (2026-04); 2.1.0 stable (2026-03) | **GBM slot.** Regression and classification (`predict_proba`, calibrated), conformal `predict_intervals`, `calculate_feature_importance`, `predict_contributions` (SHAP-style), partial dependence, monotone constraints, native categoricals and missing values, `set_seed`, serde + `from_json`, `ColumnarMatrix`/`fit_columnar` (maps straight onto Polars columns). Deps: rayon, serde, rand, sysinfo. One tuning knob (`budget`). |
| **smartcore** | 0.6.14 (2026-08-26, active daily) | **RF / trees.** Random forest regressor+classifier, extra trees, decision tree, kNN, SVM, PCA, LDA, k-means, StandardScaler/one-hot, serde on every model. Its `xgboost` module is a 33 KB regressor-only file — not the GBM slot. |
| **linfa** | 0.8.1 (2025-12) | **Rejected as foundation.** BLAS-free since 0.6 (the earlier LAPACK worry is stale); 0.8 added RF/AdaBoost/LARS. But: no inference, no robust/quantile, no GBM, pulls ndarray in as a second array type. À la carte later if GMM/DBSCAN/OPTICS are wanted. |
| **robust-rs** | 0.1.0 (2026-07) | Tail only. M/S/MM/LTS estimators with breakdown theory. Pull if someone asks for `rlm(method="MM")`. |
| **tract** | main | **Not in v1.** `onnx/src/ops/ml/mod.rs` registers exactly five `ai.onnx.ml` ops: CategoryMapper, LinearClassifier, LinearRegressor, Normalizer, TreeEnsembleClassifier. **Missing: Scaler, TreeEnsembleRegressor, opset-5 TreeEnsemble, ZipMap, OneHotEncoder, Imputer.** A skl2onnx `Pipeline(StandardScaler, …)` or any tree *regressor* will not run. |
| **ort** (ONNX Runtime) | — | Not needed. Full op coverage but a prebuilt native library per platform — a packaging cost we just paid for Windows and shouldn't pay again for ops we can parse. |
| **faer** | 0.24.4 (2026-06) | Pure Rust. Breaks API every few months; anofox pins 0.23, expect two faer versions in the lockfile. |

XGBoost's native `save_model("m.json")` format is a stable documented schema (`learner.gradient_booster.model.trees[]`). Parse it directly; users never touch onnxmltools.

## Deviations from ProjectSpec §34

Three, each forced by the survey rather than preferred.

1. **Parse ONNX, don't execute it.** §34 says inference runs via tract. tract can't run the ops the sklearn family emits. Instead: walk the small graph skl2onnx produces, recognise the pattern, and lift it into the same typed fitted parameters native training produces. Imported models become native models on import. tract stays as a *later* fallback for graphs that don't decompose (a real neural net dragged in), not a v1 dependency.
2. **Fitted preprocessing lives in the model, not the chain.** §34 says "preprocessing is the wrangle chain, not a pipeline object." Right for one-hot design, imputation formulas, feature engineering, train/test split — those are formulas. Wrong for a fitted scaler: `(x − μ) / σ` where μ, σ came from the *training* rows. As a chain step `mean(x)` floats with the data, which the codebase already bans for pivot outputs ("a schema must not float with the data", `derivation.rs:93`). Rule: **estimated from data → in the model; a formula → in the chain.** This is also what skl2onnx's `Scaler` node *is*, so imported and native models get one contract: raw features in.
3. **Native models persist as typed params, not ONNX.** §34 says the weights live as an ONNX file. There is no mature Rust ONNX *writer*; hand-building protobufs would sit on the critical path of training. Persist native models as serde JSON on the object (weights, intercept, transforms, tree nodes, or a perpetual booster blob); persist *imported* models as their parsed params too, keeping the source bytes as an artifact for provenance. Both are data-not-code and both recompute on a recipient's machine, so both §34 motivations hold. ONNX *export* becomes a later feature. Bonus: a linear model's `predict()` compiles to a Polars expression and renders as a readable formula in the chain.

Crate substitutions follow from the survey: anofox-regression replaces linfa for the linear/GLM family; perpetual takes the GBM slot (as §34's decision log already anticipated); smartcore supplies RF and trees.

## The object

Every model — trained here or imported — lands in one shape. This is what makes the card uniform.

```
ModelObject {
  id, name,
  kind:   Linear | Logistic | Huber | Quantile | Poisson
        | GradientBoosting | RandomForest | DecisionTree,
  task:   Regression | Classification,
  features: [{ column_id, name,
               transform: None | Scale { offset, scale } | OneHot { levels } }],
  target: column_id,
  params: Linear { coef, intercept }
        | Glm { coef, intercept, link }
        | TreeEnsemble { trees, base_score, link }     // imported XGBoost / sklearn trees
        | Perpetual { booster_json },                  // native GBM, kept native for intervals/contributions
  inference: Option<[{ term, estimate, se, t, p }]>,   // linear family
  importance: Option<[{ column_id, score }]>,          // tree family
  metrics: { n_train, n_holdout,
             holdout: { r2, rmse, mae } | { accuracy, auc, confusion } },
  seed, holdout_fraction, trained_at,
  lineage_fingerprint,                                 // existing staleness machinery badges it
  source: Native | Imported { artifact, format }
}
```

- Feature columns are recorded **by stable ID**; renames stay safe; lineage cords render from the card to every input (§34, unchanged).
- Trees skip scaling entirely (scale-invariant). Categoricals are auto one-hot for the linear family and passed native to perpetual.
- `predict(model)` is a `WithColumns` step. Linear family → Polars expression, rendered as a readable formula. `TreeEnsemble` → engine tree-walk. `Perpetual` → `predict_columnar`.
- Imported models are **predict-only**; the card says so. Native models get **Retrain** as the adjacent action on the staleness badge — deliberate, never automatic (§34, unchanged).
- Metrics and inference are **fitted values**: computed once on the holdout at training time and stored, like weights. Never recomputed.
- `DataObject` gains a `Model` variant; box it for the `large_enum_variant` lint (`document.rs:126`). `ReplicatedOperation` replicates the fitted result, not the training command — training is nondeterministic across machines.

## The card

- Header: kind · target · n · holdout metrics · staleness badge ("trained on data 3 refreshes old") · **Retrain** (native) or source file + "predict only" (imported).
- **Coefficient table with significance stars** for the linear family: `term | estimate | SE | t | p | ***`.
- Feature-importance bars for the tree family. Per-row contributions on hover later (perpetual has them).
- Lineage cords to every feature column.
- Excel-dense, per AGENTS.md. This is a table, not a dashboard.

## Dropdown v1

In this order — the order is the recommendation:

1. Linear regression
2. Logistic regression
3. **Gradient boosting** — the "just predict it" default
4. Random forest
5. Decision tree — for the explainability
6. Robust linear (Huber)
7. Quantile regression
8. Poisson regression

Train dialog: pick target → features default to every other numeric/categorical column → kind → holdout 20 %, seeded → Train.

**Out of v1:** k-means/PCA (unsupervised, different card shape), SVM/kNN (no novice asks), forecasting (series-shaped, own feature), ONNX export, LightGBM text import, tract fallback.

## Imports

- **ONNX from skl2onnx.** Walk the graph and match
  `[Concat] → [Scaler | OneHotEncoder | Imputer] → LinearRegressor | LinearClassifier | TreeEnsembleRegressor | TreeEnsembleClassifier | TreeEnsemble → [ZipMap | Cast]`.
  Anything else → reject, naming the op. Users export with `skl2onnx.to_onnx(pipe, initial_types=[...])`; feature names come from `initial_types`. Protobuf via `prost` + the ONNX `.proto`.
- **XGBoost `save_model("m.json")`.** Parse `learner.gradient_booster.model.trees[]`, `base_score`, `objective`, `feature_names`. Regressor, `binary:logistic` (sigmoid), `multi:softprob` (per-class trees).
- **Import dialog** maps file feature names → columns: exact, then fuzzy (`fuzzy-matcher` is already a dependency), user confirms. Same UI shape as `RenameColumnsUsingMapping`.
- Pickles are never accepted (§34, unchanged).

## Non-predictive stats

Analysis-ToolPak-shaped, from statrs distributions plus Polars `cov`/`moment` (both already enabled):

- correlation matrix (Pearson/Spearman) with p-values
- t-test: one-sample, two-sample, Welch, paired
- chi-square independence
- one-way ANOVA
- CI on a mean

Each is a few dozen lines. Whether these are a "Stats" card or functions in the one language returning a small frame is open; the spec's scratchpad decision leans toward functions, and a card can be a display of that frame.

## Milestones — one PR each

1. **Import parsers + predict, core crate only, tests only.** Fixtures: `Pipeline(StandardScaler, LinearRegression)`, `Pipeline(OneHotEncoder, GradientBoostingRegressor)`, an XGBoost binary classifier. Our predictions match sklearn/xgboost to 1e-6. De-risks (5) and builds the tree-walk and linear predict everything else reuses. ~3–4 days.
2. **`ModelObject` + `ImportModel` op + `predict()` step + minimal card.** Import-for-inference ships end to end.
3. **Native linear/logistic via anofox + train dialog + full card with stars.** Plus the **trend-line plot layer** (lm / loess / quantile, with SE and prediction bands) — same crate, and trend lines were the original "critical." A trend line is a *view* (recomputes freely, no identity); a model is a *fitted artifact* with identity and staleness. Same math, different lifecycle — don't unify them.
4. **perpetual + smartcore RF/tree + importance on the card.**
5. **Huber / quantile / Poisson** — cheap once (3) exists. **Stats** — correlation, t-test, chi-square, ANOVA.

Later: ONNX export, LightGBM text import, tract for opaque graphs, k-means/PCA, forecasting.

## Housekeeping when the time comes

- New workspace deps: `anofox-regression`, `perpetual`, `smartcore`, `prost` + ONNX proto. That's the whole footprint; no Python, no C++, no BLAS.
- Acceptance tests for anofox against known R/statsmodels outputs (mtcars OLS, a logistic on a standard dataset).
- Re-verify tract's `ai.onnx.ml` registry before ever adding it — the gap may close.
- `to_ndarray` on the Polars feature list is *not* needed under this plan; anofox takes faer matrices, perpetual takes columnar slices.

## Open questions

- Approve the dependency footprint above?
- Amend ProjectSpec §34 now (superseding linfa, "run via tract", "preprocessing is the chain"), or after milestone 1 proves the parse-don't-run approach?
- Stats as a card, or as formula functions?
