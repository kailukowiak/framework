use crate::{Coefficient, FitRequest, StatisticType};
use anofox_regression::core::RegressionResult;
use anofox_regression::inference::HcInference;

pub(crate) fn coefficient(
    request: &FitRequest,
    result: &RegressionResult,
    robust: Option<&HcInference>,
    index: usize,
) -> Coefficient {
    let mut row = Coefficient {
        term: if index == 0 {
            "Intercept".into()
        } else {
            request.feature_names[index - 1].clone()
        },
        estimate: None,
        standard_error: None,
        statistic: None,
        statistic_type: StatisticType::T,
        p_value: None,
        confidence_lower: None,
        confidence_upper: None,
    };
    if index == 0 {
        row.estimate = result.intercept;
        if let Some(hc) = robust.and_then(|r| r.intercept.as_ref()) {
            row.standard_error = Some(hc.std_error);
            row.statistic = Some(hc.t_statistic);
            row.p_value = Some(hc.p_value);
            row.confidence_lower = Some(hc.conf_interval.0);
            row.confidence_upper = Some(hc.conf_interval.1);
        } else {
            row.standard_error = result.intercept_std_error;
            row.statistic = result.intercept_t_statistic;
            row.p_value = result.intercept_p_value;
            row.confidence_lower = result.intercept_conf_interval.map(|v| v.0);
            row.confidence_upper = result.intercept_conf_interval.map(|v| v.1);
        }
    } else {
        let i = index - 1;
        row.estimate = Some(result.coefficients[i]);
        if let Some(hc) = robust {
            row.standard_error = Some(hc.std_errors[i]);
            row.statistic = Some(hc.t_statistics[i]);
            row.p_value = Some(hc.p_values[i]);
            row.confidence_lower = Some(hc.conf_interval_lower[i]);
            row.confidence_upper = Some(hc.conf_interval_upper[i]);
        } else {
            row.standard_error = result.std_errors.as_ref().map(|v| v[i]);
            row.statistic = result.t_statistics.as_ref().map(|v| v[i]);
            row.p_value = result.p_values.as_ref().map(|v| v[i]);
            row.confidence_lower = result.conf_interval_lower.as_ref().map(|v| v[i]);
            row.confidence_upper = result.conf_interval_upper.as_ref().map(|v| v[i]);
        }
    }
    for value in [
        &mut row.estimate,
        &mut row.standard_error,
        &mut row.statistic,
        &mut row.p_value,
        &mut row.confidence_lower,
        &mut row.confidence_upper,
    ] {
        *value = value.and_then(crate::metrics::finite);
    }
    row
}
