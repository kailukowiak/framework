# Astra suggested ML and statistics scope

Working proposal from the product discussion. This records the intended scope;
it does not describe features already implemented or replace ProjectSpec.md.

## Purpose

Make useful statistics and machine learning accessible to novice users inside
FrameWork, and let people bring trained Python pipelines into a workbook for
inference. The statistical surface should be substantially more capable than
Excel's basic tools without trying to match R or Python's breadth.

Models and statistical results belong in the document's dependency graph.
Their outputs should feed ordinary frames, calculated columns, Scratchwork,
and plots, so fitting a model opens up further work in the same document.

## Product scope

### 1. Useful linear and logistic regression

Provide approachable linear and logistic regression with coefficient tables,
confidence intervals, relevant diagnostics, and explicit intercept and
missing-value policies. Support robust inference where the estimator supports
it, rather than offering only fitted values and a score.

Separate fitting from inference settings:

- OLS with classical or heteroskedasticity-robust covariance, initially HC3.
- Clustered and HAC/Newey–West covariance as follow-on capabilities, with
  explicit group columns or ordering and lag settings.
- Robust regression such as Huber as a distinct estimator: it changes the fit,
  whereas robust OLS covariance changes uncertainty estimates without changing
  the fitted coefficients.

Robust standard errors do not repair omitted-variable bias or make the OLS
coefficients resistant to outliers. The interface should name the method
precisely rather than presenting a general-purpose "robust" switch.

### 2. A small collection of general ML and data-science models

Candidate initial additions are ridge/lasso, random forest, k-means clustering,
and PCA. They cover regularized prediction, nonlinear prediction, grouping,
and dimensionality reduction without attempting a comprehensive estimator
catalog. Exact ordering remains a product decision.

Outputs vary by model: probabilities, cluster assignments, components, and
transformed frames are as legitimate as predictions. Do not force every
estimator into a target-and-prediction-only interface.

### 3. Actual XGBoost

XGBoost is a core requirement for regression and classification, with validation
and early stopping. A different gradient-boosting implementation should not be
presented as XGBoost.

This changes the existing pure-Rust boosting direction: investigate and accept
the native dependency and packaging work needed for actual XGBoost. The exact
integration is still to be selected and proved.

### 4. Non-predictive statistics

Include a focused selection of correlations, group comparisons, t-tests,
chi-square tests, and confidence intervals. Present the estimate, uncertainty,
sample size, and relevant method details together. The objective is an
understandable analytical result that remains useful elsewhere in the workbook.

### 5. Imported fitted pipelines

Support importing trained sklearn and XGBoost pipelines for inference without
requiring Python on the recipient's machine. Preserve supported preprocessing
alongside the estimator so inference means the same thing as it did in Python.

Publish an explicit compatibility contract for supported transformers,
estimators, versions, input types, and outputs. Provide a Python export helper
and verify prediction parity on representative inputs. Arbitrary sklearn
pipelines, particularly custom Python transformers, are not a blanket promise.
Unsupported steps should fail with a clear explanation during export or import.

ONNX is a candidate interchange format for supported inference paths. Prove
conversion and runtime compatibility before committing to an ONNX-only design.
A prediction artifact alone cannot represent the complete statistical analysis;
coefficients, covariance, diagnostics, and other fitted state need appropriate
persisted representations too. Do not accept executable pickle imports.

## Two compact card types

### Model card

Holds the model specification and a dense fitted-result summary:

- Source frame and feature columns, referenced by stable IDs.
- Target when applicable, plus weights, groups, or time columns when needed.
- Estimator and its relevant fit settings.
- Supported inference settings, such as covariance method and confidence level.
- Training-data fingerprint, fitted preprocessing, seed where applicable,
  backend/version information, and fit status.
- A compact evaluation summary and access to detailed outputs.

Fit and retrain are explicit actions because they create fitted state. Inference
uses that saved state and updates when its input data changes. Changed training
data should make the fitted model visibly stale; it should not silently refit.

### Stats output card

Shows a coefficient table, statistical test result, or evaluation summary. Its
numbers are referenceable from Scratchwork, and tabular outputs are usable as
frames. More detailed diagnostics and plots can open from the summary.

Follow FrameWork's density rules: twenty coefficients should look like twenty
spreadsheet rows. Use ordinary text sizes, make names at least as prominent as
values, and avoid a control or decorative container for every statistic.

## Shared model contract

Use one model object with capabilities declared by each estimator:

1. **Specification:** data, features, target if applicable, and supporting columns.
2. **Fit settings:** estimator-specific parameters, loss, regularization, and seed.
3. **Inference settings:** only covariance methods or uncertainty procedures that
   the estimator actually supports.
4. **Outputs:** typed tables, columns, diagnostics, probabilities, predictions,
   components, or forecasts as appropriate.

Native-trained and imported models should use the same cards and downstream
calculation paths. An imported artifact can support inference even if its
training backend is unavailable; its retraining capability should be explicit.

## Preprocessing and evaluation

Expose preparation through Wrangle while preserving a fit/apply distinction.
Imputation, scaling, and category encoding must learn from training rows only
and apply their saved state to validation and inference rows. Being visible in
the transformation chain does not by itself prevent data leakage.

Treat evaluation as part of the modeling workflow. Choose splits appropriate to
the data, including time or group separation when needed. Keep training,
validation, and final test roles distinct; an early-stopping validation set is
not an untouched final test set. Report held-out performance clearly and compare
predictive models with a simple baseline.

Persist the input schema, feature order, category handling, and preprocessing
state so renamed columns and new inference data do not silently change meaning.

## Novice entry points

Start from tasks such as "predict a number," "predict a category," "find
groups," and "compare groups." Show the actual method alongside the task,
provide sensible defaults, and reveal deeper settings when needed. Surface
method failures and invalid inputs as ordinary inline text.

The success criterion is that a novice can fit and assess something useful,
while a Python user can hand them a supported trained pipeline that becomes a
working part of their document.

## Suggested implementation sequence

1. Linear and logistic regression, the shared model contract, and both card
   types. Include useful statistical outputs and prove live downstream inference.
2. Actual XGBoost, fitted preprocessing, and a sound evaluation workflow.
3. Pipeline import with an export helper, a tested compatibility matrix, and
   prediction-parity checks.
4. Broader statistical procedures and exploratory models based on demand.

Design the artifact and preprocessing contracts in the first slice, even though
pipeline import ships later. Verify native statistical backends for inference
and diagnostics, not merely their ability to fit coefficients. An explicitly
installed Python plugin remains a possible extension path for specialist methods.

## Decisions to reconcile with ProjectSpec.md

- Replace the proposed native Rust boosting substitute with actual XGBoost.
- Revisit ONNX as the only persisted model representation; distinguish portable
  prediction interchange from complete fitted statistical state.
- Make supported fitted-pipeline import a first-class product capability.
- Preserve deliberate fitting, live inference, visible lineage, and dense cards.
