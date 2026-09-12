"""Regenerate small, inspectable reference cases with statsmodels (never anofox)."""
import json
from pathlib import Path
import warnings
import numpy as np
import scipy
import statsmodels
import statsmodels.api as sm


def summary(fit):
    return {"coefficients": fit.params.tolist(), "se": fit.bse.tolist(),
            "statistic": fit.tvalues.tolist(), "p": fit.pvalues.tolist(),
            "ci": fit.conf_int().tolist(), "predictions": fit.predict().tolist()}

x = np.array([[i / 3, ((i * 7) % 11) * 20, ((i * 3) % 7) / 100]
              for i in range(24)], dtype=float)
noise = np.array([(-1 if i % 2 else 1) * (0.1 + i / 15) for i in range(24)])
y = 3 + x @ np.array([0.8, -0.02, 15]) + noise
ols = sm.OLS(y, sm.add_constant(x)).fit()
lx = np.array([[i / 5 - 2, ((i * 7) % 9) / 3 - 1] for i in range(30)])
ly = np.array([0, 1, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 1, 0,
               0, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 0, 1, 1])
logistic = sm.GLM(ly, sm.add_constant(lx), family=sm.families.Binomial()).fit(tol=1e-12, maxiter=200)
rx = np.column_stack([x[:, 0], 2 * x[:, 0]])
rank = sm.OLS(y, sm.add_constant(rx)).fit()
sx = np.arange(-5, 6, dtype=float).reshape(-1, 1)
sy = (sx[:, 0] > 0).astype(float)
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    separated = sm.GLM(sy, sm.add_constant(sx), family=sm.families.Binomial()).fit(maxiter=100)
separation = {"x": sx.tolist(), "y": sy.tolist(),
              "warnings": sorted(set(type(w.message).__name__ for w in caught)),
              "converged": separated.converged}
limited = sm.GLM(ly, sm.add_constant(lx), family=sm.families.Binomial()).fit(maxiter=1)
result = {
    "versions": {"statsmodels": statsmodels.__version__, "numpy": np.__version__, "scipy": scipy.__version__},
    "ols": {"x": x.tolist(), "y": y.tolist(), "classical": summary(ols),
            "hc3": summary(ols.get_robustcov_results(cov_type="HC3", use_t=True)), "df": ols.df_resid},
    "logistic": {"x": lx.tolist(), "y": ly.tolist(), "classical": summary(logistic),
                 "limited_iterations_converged": limited.converged,
                 "intervals_by_level": {str(level): logistic.conf_int(alpha=1-level).tolist() for level in [0.8, 0.99]}},
    "rank_deficient": {"x": rx.tolist(), "y": y.tolist(), "rank": int(rank.model.rank),
                       "predictions": rank.predict().tolist()},
    "separation": separation,
}
Path(__file__).with_name("reference.json").write_text(json.dumps(result, indent=2, allow_nan=False) + "\n")

# Independent HiGHS LP references. Integer inputs keep the exact geometry
# unambiguous relative to the numerical reference solver's tolerance.
from scipy.optimize import linprog
rng = np.random.default_rng(1729)
separation_cases = []
for index in range(24):
    design = rng.integers(-5, 6, size=(16, 2))
    if index % 2:
        labels = (design[:, 0] + 2 * design[:, 1] > 0).astype(float)
    else:
        labels = rng.integers(0, 2, size=16).astype(float)
    signed = (2 * labels - 1)[:, None] * sm.add_constant(design)
    lp = linprog(-signed.sum(axis=0), A_ub=-signed,
                 b_ub=np.zeros(16), bounds=[(-1, 1)] * 3, method="highs")
    if not lp.success:
        raise RuntimeError(lp.message)
    objective = -lp.fun
    separation_cases.append({"x": design.tolist(), "y": labels.tolist(),
                             "separated": bool(objective > 1e-7),
                             "reference_objective": float(objective)})
Path(__file__).with_name("separation_reference.json").write_text(
    json.dumps({"scipy": scipy.__version__, "solver": "HiGHS via scipy.optimize.linprog", "seed": 1729,
                "cases": separation_cases}, indent=2) + "\n")
