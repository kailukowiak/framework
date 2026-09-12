//! Standalone statistical summaries, independent of predictive models.
use crate::{MlError, Result};
use anofox_regression::distributions::{ContinuousCDF, Normal, StudentsT};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum StatsMethod {
    MeanConfidence,
    PearsonCorrelation,
    WelchDifference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StatsRequest {
    pub method: StatsMethod,
    pub confidence_level: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StatsResult {
    pub method: StatsMethod,
    pub estimate: Option<f64>,
    pub standard_error: Option<f64>,
    pub statistic: Option<f64>,
    pub degrees_of_freedom: Option<f64>,
    pub p_value: Option<f64>,
    pub confidence_lower: Option<f64>,
    pub confidence_upper: Option<f64>,
    pub n_x: usize,
    pub n_y: Option<usize>,
    pub confidence_level: f64,
    pub warnings: Vec<String>,
}

pub fn statistics(request: &StatsRequest, x: &[f64], y: Option<&[f64]>) -> Result<StatsResult> {
    if !request.confidence_level.is_finite()
        || !(0.0..1.0).contains(&request.confidence_level)
        || request.confidence_level == 0.
    {
        return Err(MlError::InvalidInput(
            "Confidence must be strictly between 0 and 1".into(),
        ));
    }
    if x.len() + y.map_or(0, <[f64]>::len) > 2_000_000 {
        return Err(MlError::ResourceLimit(
            "The statistical summary exceeds the two-million-value budget".into(),
        ));
    }
    let mut result = StatsResult {
        method: request.method,
        estimate: None,
        standard_error: None,
        statistic: None,
        degrees_of_freedom: None,
        p_value: None,
        confidence_lower: None,
        confidence_upper: None,
        n_x: x.len(),
        n_y: y.map(<[f64]>::len),
        confidence_level: request.confidence_level,
        warnings: vec![],
    };
    match request.method {
        StatsMethod::MeanConfidence => {
            if y.is_some() {
                return Err(MlError::InvalidInput(
                    "Mean confidence takes one sample".into(),
                ));
            }
            let (mean, variance) = moments(x)?;
            interval(
                &mut result,
                mean,
                (variance / x.len() as f64).sqrt(),
                (x.len() - 1) as f64,
            )?;
            if variance == 0. {
                result.warnings.push(
                    "Every observed value is equal; the sample-based interval has zero width"
                        .into(),
                );
            }
        }
        StatsMethod::PearsonCorrelation => correlation(
            &mut result,
            x,
            y.ok_or_else(|| MlError::InvalidInput("Choose two columns for correlation".into()))?,
        )?,
        StatsMethod::WelchDifference => welch(
            &mut result,
            x,
            y.ok_or_else(|| {
                MlError::InvalidInput("Choose two samples for Welch's difference of means".into())
            })?,
        )?,
    }
    Ok(result)
}

fn moments(values: &[f64]) -> Result<(f64, f64)> {
    if values.len() < 2 || values.iter().any(|v| !v.is_finite()) {
        return Err(MlError::InvalidInput("Statistics require at least two finite observations per sample; handle missing values upstream".into()));
    }
    let mean = crate::metrics::mean(values);
    let variance = values
        .iter()
        .map(|v| (v - mean).powi(2) / (values.len() - 1) as f64)
        .sum::<f64>();
    if !mean.is_finite() || !variance.is_finite() {
        return Err(MlError::Numerical(
            "The sample's mean or variance overflowed".into(),
        ));
    }
    Ok((mean, variance))
}

fn interval(result: &mut StatsResult, estimate: f64, se: f64, df: f64) -> Result<()> {
    let distribution = StudentsT::new(0., 1., df).map_err(|e| MlError::Numerical(e.to_string()))?;
    let critical = distribution.inverse_cdf((1. + result.confidence_level) / 2.);
    let lower = estimate - critical * se;
    let upper = estimate + critical * se;
    if !critical.is_finite() || critical <= 0. || !lower.is_finite() || !upper.is_finite() {
        return Err(MlError::Numerical(
            "The requested confidence interval is numerically unavailable".into(),
        ));
    }
    result.estimate = Some(estimate);
    result.standard_error = Some(se);
    result.degrees_of_freedom = Some(df);
    result.confidence_lower = Some(lower);
    result.confidence_upper = Some(upper);
    Ok(())
}

fn welch(result: &mut StatsResult, x: &[f64], y: &[f64]) -> Result<()> {
    let (mx, vx) = moments(x)?;
    let (my, vy) = moments(y)?;
    let a = vx / x.len() as f64;
    let b = vy / y.len() as f64;
    if a + b <= 0. {
        return Err(MlError::InvalidInput(
            "Welch inference needs observed variation in at least one sample".into(),
        ));
    }
    let df = (a + b).powi(2) / (a * a / (x.len() - 1) as f64 + b * b / (y.len() - 1) as f64);
    interval(result, mx - my, (a + b).sqrt(), df)?;
    let statistic = (mx - my) / (a + b).sqrt();
    result.statistic = crate::metrics::finite(statistic);
    result.p_value = Some(
        2. * StudentsT::new(0., 1., df)
            .map_err(|e| MlError::Numerical(e.to_string()))?
            .sf(statistic.abs()),
    );
    Ok(())
}

fn correlation(result: &mut StatsResult, x: &[f64], y: &[f64]) -> Result<()> {
    if x.len() != y.len() || x.len() < 3 {
        return Err(MlError::InvalidInput(
            "Correlation needs at least three aligned pairs".into(),
        ));
    }
    let (mx, vx) = moments(x)?;
    let (my, vy) = moments(y)?;
    if vx <= 0. || vy <= 0. {
        return Err(MlError::InvalidInput(
            "Correlation is undefined for a constant column".into(),
        ));
    }
    let denominator = (vx * vy).sqrt();
    if !denominator.is_finite() || denominator == 0. {
        return Err(MlError::Numerical(
            "Correlation scale is numerically unavailable".into(),
        ));
    }
    let r = (x
        .iter()
        .zip(y)
        .map(|(a, b)| ((a - mx) * (b - my) / denominator) / (x.len() - 1) as f64)
        .sum::<f64>())
    .clamp(-1., 1.);
    let df = (x.len() - 2) as f64;
    let statistic = r * (df / (1. - r * r)).sqrt();
    result.estimate = Some(r);
    result.statistic = crate::metrics::finite(statistic);
    result.degrees_of_freedom = Some(df);
    result.p_value = Some(if r.abs() == 1. {
        0.
    } else {
        2. * StudentsT::new(0., 1., df)
            .map_err(|e| MlError::Numerical(e.to_string()))?
            .sf(statistic.abs())
    });
    let (lower, upper) = if r.abs() == 1. {
        (r, r)
    } else if x.len() == 3 {
        (-1., 1.)
    } else {
        let critical = Normal::new(0., 1.)
            .map_err(|e| MlError::Numerical(e.to_string()))?
            .inverse_cdf((1. + result.confidence_level) / 2.);
        if !critical.is_finite() || critical <= 0. {
            return Err(MlError::InvalidInput(
                "Confidence level is too close to the floating-point limit".into(),
            ));
        }
        let width = critical / ((x.len() - 3) as f64).sqrt();
        ((r.atanh() - width).tanh(), (r.atanh() + width).tanh())
    };
    result.confidence_lower = Some(lower);
    result.confidence_upper = Some(upper);
    Ok(())
}
