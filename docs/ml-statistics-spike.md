# Statistical backend spike — measured acceptance and adapter

Status: **the bounded binary-logistic adapter closes the observed separation and
confidence-interval gaps in this spike.** The raw backend still fails the recorded
separation test. This is not production integration or acceptance of the full library.

The [isolated harness](../tools/ml-spikes/statistics/README.md) pins anofox-regression
0.5.13 and faer 0.23.2. The consensus survey's anofox version was available; there was
no discrepancy. Batch 2 adds microlp 0.6.0 and num-rational 0.4.2 for proposed LP
certificates and exact verification. It changes no application dependencies.

## Batch 1: upstream evidence retained

| Case | Measured result |
|---|---|
| Multiscale OLS with intercept and three predictors | Classical estimates, SE, t, p, slope CI, predictions and residual df match statsmodels |
| HC3 on the same fit | SE/p and slope CI match statsmodels with explicit Student-t inference (`use_t=True`) |
| Ordinary unpenalized binary logistic | Estimates, SE, z, p and probabilities match; backend coefficient CI fields are absent |
| Rank-deficient OLS | One explicit aliased NaN; identifiable fitted values match the pseudoinverse reference |
| Logistic limited to one iteration | Returns `ConvergenceFailed { iterations: 1 }` |
| Perfect logistic separation | **Raw backend defect:** success with `converged=true`, slope 39.906382837046316 and intercept -19.928686656388173; statsmodels warns of separation |

The raw-backend separation test remains ignored by default and fails when explicitly
run. It preserves the upstream finding. Nonignored adapter tests now prove that the
same case and additional complete/quasi-separation cases are rejected before fitting.

## Batch 2: adapter contract

[`logistic::fit`](../tools/ml-spikes/statistics/src/logistic.rs) accepts a rectangular,
finite numeric matrix, binary 0/1 labels with both classes, and explicit settings.
The bounded scope is **unpenalized binary logit with an intercept**, at most 256 rows
and 16 predictors, more rows than fitted parameters, no missing values or weights.
These are spike limits, not proposed FrameWork product limits.

`LogisticFit` returns intercept-first coefficient rows in original feature units,
SE, explicitly named z statistics, two-sided normal p-values, normal-Wald intervals,
training-row probabilities for class 1, confidence level, model-based Fisher-information
covariance-method metadata, and iteration count. Feature order is the supplied order.

`FitError` distinguishes malformed shapes, size limits, nonfinite inputs, invalid
labels/settings, rank deficiency, certified separation, inconclusive detection,
LP interruption/failure, backend errors, nonconvergence, and unavailable inference.
There is no successful result carrying an unverified separation diagnostic. The
adapter does not automatically regularize, omit rows, impute, or silently change
estimators. No partial fit or application document is mutated.

Column scaling improves numerical fitting without changing units in the returned
coefficients/SE. The adapter verifies finite outputs and a small normalized score
residual in addition to the backend's convergence flag. Confidence intervals are
calculated with declared standard-normal quantiles, not the backend's missing fields.

Retained marginal inference supports the declared per-coefficient normal-Wald
intervals. It does **not** expose a full covariance matrix, contrasts, robust logistic
inference, or prediction/mean-response intervals. No generic logistic `t_statistics`
name leaks into the typed result. The original upstream OLS `rank` excludes the
separately handled intercept in its tested path; adapters must normalize that meaning.
Aliased OLS NaNs still need typed unavailable values before application serialization.

## Why separation detection is not a coefficient cutoff

For each observation, form the signed design row
`Z_i = (2*y_i - 1) * [1, x_i / scale]`, with positive feature scales.
The implementation preserves each original f64 exactly as a rational number before
scaling. Thus a floating LP solver cannot change the input geometry by rounding tiny
margins or tiny features away.

Two exact certificates decide the outcome:

- **Overlap:** a strictly positive vector `w` with `Z' w = 0`. If a separating
  direction had all nonnegative margins and at least one strictly positive margin,
  their weighted sum would be positive, contradicting `w' Z beta = 0`. Together
  with full column rank, this rules out both complete and quasi-complete separation.
- **Separation:** a direction `beta` with `Z beta >= 0` and at least one strict
  inequality. Every returned witness is checked exactly. Strict margins everywhere
  establish complete separation. Otherwise the error says `CompleteOrQuasi`: a
  witness touching a boundary alone does not prove complete separation is impossible.
  Both cases are refused; the adapter does not pretend to classify the distinction.

microlp proposes witnesses: an overlap LP minimizes the sum of weights subject to
`w >= 1` and `Z' w = 0`; a separation LP maximizes the sum of signed margins subject
to nonnegative margins and coefficients bounded by ±1. These bounds remove arbitrary
scaling without changing the existence of a separating direction.

Exact Gaussian elimination repairs a proposed overlap vector and reconstructs a
proposed separating vertex from active constraints. Proposed active sets may use
floating tolerances, but **only exact final certificate checks can accept them**.
If repair or reconstruction cannot certify either outcome, the adapter returns
`Inconclusive`. LP infeasibility alone is never proof of overlap. Only an optimal
LP termination is considered; interrupted or failed solves are explicit errors.
There are at most two LP solves with separate configured budgets. Dimension limits
also bound exact-arithmetic work; this is not yet a hard end-to-end wall-clock limit
or cancellable production job.

This follows the separation/LP principle in [Konis's original thesis](https://ora.ox.ac.uk/objects/uuid%3A8f9ee0d0-d78e-4101-9ab4-f9cbceed2a2a).
The exact certificate checker is our own implementation, not a claim that we copied
or reproduced the full detectseparation package. It relies on exact arithmetic for
the supplied binary floating values, not hypothetical unrounded source measurements.

## Executed evidence

**15 tests pass; one raw-backend failure remains explicitly ignored.** Adapter and
detector tests cover:

- Classical logistic estimates, SE, z/p, probabilities and 95% intervals against
  statsmodels, plus independently generated 80% and 99% intervals.
- Multivariate complete and quasi separation where neither individual feature
  separates; the original upstream reproducer; and tiny margins of magnitude 1e-14.
- Feature scaling by 1e12 and 1e-12, including a sign reversal: separation decisions
  are retained; overlapping fits retain probabilities and appropriately scaled inference.
- Overlapping/XOR classes, including identical predictors with opposite labels.
- 24 independently generated integer-data classifications from SciPy's HiGHS solver:
  12 separated and 12 overlapping; all match the adapter.
- Rank deficiency, invalid labels including one-class data, nonfinite predictors,
  empty/ragged/mismatched shapes, invalid confidence settings, forced fit nonconvergence,
  disabled detection budget, and a real LP solver interruption.

The ordinary suite's exact runtime is not a benchmark. Isolated Clippy passes with
warnings denied, including `too_many_lines`; formatting passes. Batch 1's root Rust
lint passed with warnings in untouched application code. Batch 2's root owner runs
workspace checks once; this isolated workspace needs its own commands below.

Fixtures are generated with statsmodels 0.14.6, NumPy 2.5.3 and SciPy 1.18.1 on
Python 3.12.14. The generator, exact requirements and data are retained. Regeneration
was byte-identical. SHA-256:

- `reference.json`: `158f9310d58899b352c59774264b516962fe7e26c4284e0d2559fa4eb49660c2`
- `separation_reference.json`: `aaea6351b2f93f664fb5d2a06704f4ccd20ecf3063708961eda9f2497aa4714a`

Numerical inference tolerance is `1e-8 + 1e-6 * abs(reference)` for the recorded f64
cases. It is not an import-wide or float32 promise. Separation certificates use exact
sign/equality checks, not that tolerance. HiGHS fixtures use integer geometry and an
explicit reference objective threshold; they supplement the constructive boundary cases.

## Remaining gates

The exact checker and numerical adapter need production review and larger/adversarial
coverage before being promoted from this isolated spike. Some difficult overlapping
datasets may be refused as inconclusive; this is deliberate and must remain visible.
Do not broaden the input limits without measuring rational-arithmetic cost and adding
cancellation/resource controls. Highly ill-conditioned fitting, rare-event inference,
and unusual confidence levels need more numerical coverage even when overlap is proved.

OLS HC0–HC2, clustered/HAC methods, weights, no-intercept fits, missing-value row
alignment, small/saturated samples, multiclass, calibration, model serialization and
app lifecycle remain outside this adapter. The public HC3 API still supplies marginal
SE/inference rather than full sandwich covariance; claims about contrasts or robust
prediction intervals remain blocked. No UI, Tauri, persistence or engine code changed,
so no native e2e claim applies.

Primary sources inspected: [pinned anofox crate/source](https://docs.rs/crate/anofox-regression/0.5.13),
[pinned microlp solve/status API](https://docs.rs/microlp/0.6.0/microlp/), and
[statsmodels robust covariance API](https://www.statsmodels.org/stable/generated/statsmodels.regression.linear_model.OLSResults.get_robustcov_results.html).
The acceptance claims above come from executable tests, not upstream quality claims.
