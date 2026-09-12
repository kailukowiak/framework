//! Isolated statistical-backend acceptance spike; no application dependency.

mod exact;
pub mod logistic;
mod separation;

#[cfg(test)]
mod tests {
    use anofox_regression::core::RegressionResult;
    use anofox_regression::inference::{compute_hc_inference, HcType};
    use anofox_regression::solvers::{
        FittedRegressor, LogisticRegression, OlsRegressor, Regressor,
    };
    use faer::{Col, Mat};
    use serde_json::Value;

    fn references() -> Value {
        serde_json::from_str(include_str!("../reference.json")).unwrap()
    }

    fn data(case: &Value) -> (Mat<f64>, Col<f64>) {
        let rows = case["x"].as_array().unwrap();
        let x = Mat::from_fn(rows.len(), rows[0].as_array().unwrap().len(), |i, j| {
            rows[i][j].as_f64().unwrap()
        });
        let y = Col::from_fn(rows.len(), |i| case["y"][i].as_f64().unwrap());
        (x, y)
    }

    fn close(actual: f64, expected: f64, label: &str) {
        // All fixtures and solvers use f64. Relative tolerance permits iterative
        // GLM termination differences; the floor handles probabilities near zero.
        let limit = 1e-8 + 1e-6 * expected.abs();
        assert!(
            actual.is_finite() && (actual - expected).abs() <= limit,
            "{label}: actual {actual}, expected {expected}, tolerance {limit}"
        );
    }

    fn check_column(actual: &Col<f64>, expected: &Value, label: &str) {
        assert_eq!(actual.nrows(), expected.as_array().unwrap().len());
        for i in 0..actual.nrows() {
            close(actual[i], expected[i].as_f64().unwrap(), label);
        }
    }

    fn check_summary(result: &RegressionResult, reference: &Value, has_intervals: bool) {
        close(
            result.intercept.unwrap(),
            reference["coefficients"][0].as_f64().unwrap(),
            "intercept",
        );
        close(
            result.intercept_std_error.unwrap(),
            reference["se"][0].as_f64().unwrap(),
            "intercept SE",
        );
        close(
            result.intercept_p_value.unwrap(),
            reference["p"][0].as_f64().unwrap(),
            "intercept p",
        );
        for i in 0..result.coefficients.nrows() {
            for (actual, key) in [
                (result.coefficients[i], "coefficients"),
                (result.std_errors.as_ref().unwrap()[i], "se"),
                (result.t_statistics.as_ref().unwrap()[i], "statistic"),
                (result.p_values.as_ref().unwrap()[i], "p"),
            ] {
                close(actual, reference[key][i + 1].as_f64().unwrap(), key);
            }
            if has_intervals {
                close(
                    result.conf_interval_lower.as_ref().unwrap()[i],
                    reference["ci"][i + 1][0].as_f64().unwrap(),
                    "CI lower",
                );
                close(
                    result.conf_interval_upper.as_ref().unwrap()[i],
                    reference["ci"][i + 1][1].as_f64().unwrap(),
                    "CI upper",
                );
            } else {
                assert!(result.conf_interval_lower.is_none());
                assert!(result.conf_interval_upper.is_none());
            }
        }
        check_column(
            &result.fitted_values,
            &reference["predictions"],
            "predictions",
        );
    }

    #[test]
    fn ols_multiscale_classical_and_hc3_match_statsmodels() {
        let references = references();
        let case = &references["ols"];
        let (x, y) = data(case);
        let fit = OlsRegressor::builder()
            .with_intercept(true)
            .build()
            .fit(&x, &y)
            .unwrap();
        let result = fit.result();
        check_summary(result, &case["classical"], true);
        assert_eq!(result.residual_df() as f64, case["df"].as_f64().unwrap());
        let hc = compute_hc_inference(
            &x,
            &result.coefficients,
            result.intercept,
            &result.residuals,
            &result.aliased,
            true,
            HcType::HC3,
            0.95,
        )
        .unwrap();
        let reference = &case["hc3"];
        let intercept = hc.intercept.unwrap();
        close(
            intercept.std_error,
            reference["se"][0].as_f64().unwrap(),
            "HC3 intercept SE",
        );
        close(
            intercept.p_value,
            reference["p"][0].as_f64().unwrap(),
            "HC3 intercept p",
        );
        for i in 0..hc.std_errors.nrows() {
            close(
                hc.std_errors[i],
                reference["se"][i + 1].as_f64().unwrap(),
                "HC3 SE",
            );
            close(
                hc.p_values[i],
                reference["p"][i + 1].as_f64().unwrap(),
                "HC3 p",
            );
            close(
                hc.conf_interval_lower[i],
                reference["ci"][i + 1][0].as_f64().unwrap(),
                "HC3 CI lower",
            );
            close(
                hc.conf_interval_upper[i],
                reference["ci"][i + 1][1].as_f64().unwrap(),
                "HC3 CI upper",
            );
        }
    }

    #[test]
    fn logistic_unpenalized_inference_matches_statsmodels() {
        let references = references();
        let case = &references["logistic"];
        let (x, y) = data(case);
        let fit = LogisticRegression::builder()
            .tolerance(1e-12)
            .max_iterations(200)
            .build()
            .fit(&x, &y)
            .unwrap();
        assert!(fit.inner().converged);
        check_summary(fit.inner().result(), &case["classical"], false);
        check_column(
            &fit.predict_proba(&x),
            &case["classical"]["predictions"],
            "probabilities",
        );
    }

    #[test]
    fn rank_deficiency_is_explicit_and_predictions_match() {
        let references = references();
        let case = &references["rank_deficient"];
        let (x, y) = data(case);
        let fit = OlsRegressor::builder().build().fit(&x, &y).unwrap();
        let result = fit.result();
        // anofox reports predictor rank, excluding the intercept in OLS.
        assert_eq!((result.rank + 1) as u64, case["rank"].as_u64().unwrap());
        assert_eq!(result.aliased.iter().filter(|&&value| value).count(), 1);
        for (i, &aliased) in result.aliased.iter().enumerate() {
            assert_eq!(result.coefficients[i].is_nan(), aliased);
        }
        // A pseudoinverse and pivoted QR choose different aliased coefficients;
        // compare their identifiable fitted values instead of arbitrary slopes.
        check_column(
            &result.fitted_values,
            &case["predictions"],
            "rank-deficient predictions",
        );
    }

    #[test]
    fn iteration_limit_returns_error() {
        let references = references();
        let case = &references["logistic"];
        assert_eq!(case["limited_iterations_converged"], false);
        let (x, y) = data(case);
        let error = LogisticRegression::builder()
            .max_iterations(1)
            .build()
            .fit(&x, &y)
            .unwrap_err();
        println!("iteration limit: {error:?}");
        assert!(format!("{error:?}").contains("Convergence"));
    }

    #[test]
    #[ignore = "known blocker: anofox reports convergence for a perfectly separated fit"]
    fn perfect_separation_must_not_return_success() {
        let references = references();
        let case = &references["separation"];
        assert!(case["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "PerfectSeparationWarning"));
        let (x, y) = data(case);
        let outcome = LogisticRegression::builder().build().fit(&x, &y);
        assert!(
            outcome.is_err(),
            "SEPARATION BLOCKER: backend returned a successful fit: {outcome:?}"
        );
    }
}
