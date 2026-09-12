//! Bounded unpenalized, binary logistic adapter; intercept always included.
use crate::separation::{detect, signed_design};
pub use crate::separation::{DetectionError, Separation};
use anofox_regression::distributions::{ContinuousCDF, Normal};
use anofox_regression::solvers::{FittedRegressor, LogisticRegression};
use faer::{Col, Mat};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum FitError {
    InvalidShape,
    ResourceLimit,
    NonFiniteInput,
    InvalidLabels,
    InvalidSettings,
    Detection(DetectionError),
    Separated(Separation),
    NonConvergence,
    Backend(String),
    UnavailableInference,
}

#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub confidence_level: f64,
    pub max_iterations: usize,
    /// Per LP solve; there are at most two solves. Zero refuses the fit.
    pub detection_budget: Duration,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            confidence_level: 0.95,
            max_iterations: 200,
            detection_budget: Duration::from_secs(2),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalMethod {
    NormalWald,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CovarianceMethod {
    ModelBasedFisherInformation,
}

#[derive(Debug, Clone)]
pub struct Coefficient {
    pub estimate: f64,
    pub standard_error: f64,
    pub z_statistic: f64,
    pub p_value: f64,
    pub confidence_interval: [f64; 2],
}

#[derive(Debug, Clone)]
pub struct LogisticFit {
    /// Intercept first, followed by inputs in supplied order, in original units.
    pub coefficients: Vec<Coefficient>,
    pub probabilities: Vec<f64>,
    pub confidence_level: f64,
    pub interval_method: IntervalMethod,
    pub covariance_method: CovarianceMethod,
    pub iterations: usize,
}

fn validate(x: &[Vec<f64>], y: &[f64], settings: Settings) -> Result<Vec<f64>, FitError> {
    if x.is_empty()
        || x.len() != y.len()
        || x[0].is_empty()
        || x.iter().any(|row| row.len() != x[0].len())
    {
        return Err(FitError::InvalidShape);
    }
    if x.len() > 256 || x[0].len() > 16 {
        return Err(FitError::ResourceLimit);
    }
    if x.iter().flatten().chain(y).any(|v| !v.is_finite()) {
        return Err(FitError::NonFiniteInput);
    }
    if y.iter().any(|&v| v != 0.0 && v != 1.0) || !y.contains(&0.0) || !y.contains(&1.0) {
        return Err(FitError::InvalidLabels);
    }
    if !settings.confidence_level.is_finite()
        || !(0.0..1.0).contains(&settings.confidence_level)
        || settings.confidence_level == 0.0
        || settings.max_iterations == 0
        || settings.max_iterations > 10_000
    {
        return Err(FitError::InvalidSettings);
    }
    if x.len() <= x[0].len() + 1 {
        return Err(FitError::InvalidShape);
    }
    Ok((0..x[0].len())
        .map(|j| {
            x.iter()
                .map(|row| row[j].abs())
                .fold(0.0, f64::max)
                .max(f64::MIN_POSITIVE)
        })
        .collect())
}

pub fn fit(x: &[Vec<f64>], y: &[f64], settings: Settings) -> Result<LogisticFit, FitError> {
    let scales = validate(x, y, settings)?;
    let exact_design = signed_design(x, y, &scales);
    if let Some(kind) =
        detect(&exact_design, settings.detection_budget).map_err(FitError::Detection)?
    {
        return Err(FitError::Separated(kind));
    }
    // Scaling is a change of units only. The exact detector above certifies the
    // supplied f64 observations; it does not trust rounding in this fit matrix.
    let matrix = Mat::from_fn(x.len(), scales.len(), |i, j| x[i][j] / scales[j]);
    let labels = Col::from_fn(y.len(), |i| y[i]);
    let backend = LogisticRegression::builder()
        .tolerance(1e-12)
        .max_iterations(settings.max_iterations)
        .confidence_level(settings.confidence_level)
        .build();
    let fitted = backend.fit(&matrix, &labels).map_err(|e| match e {
        anofox_regression::solvers::RegressionError::ConvergenceFailed { .. } => {
            FitError::NonConvergence
        }
        _ => FitError::Backend(e.to_string()),
    })?;
    if !fitted.inner().converged {
        return Err(FitError::NonConvergence);
    }
    let result = fitted.inner().result();
    if result.aliased.iter().any(|&v| v) {
        return Err(FitError::Detection(DetectionError::RankDeficient));
    }
    let probabilities: Vec<_> = fitted.predict_proba(&matrix).iter().copied().collect();
    if probabilities
        .iter()
        .any(|&p| !p.is_finite() || !(0.0..=1.0).contains(&p))
    {
        return Err(FitError::UnavailableInference);
    }
    // Reject numerical stopping that has not even reached a stationary score.
    for j in 0..=scales.len() {
        let score: f64 = (0..y.len())
            .map(|i| (y[i] - probabilities[i]) * if j == 0 { 1.0 } else { matrix[(i, j - 1)] })
            .sum();
        if score.abs() > 1e-8 * y.len() as f64 {
            return Err(FitError::NonConvergence);
        }
    }
    let coefficients = coefficient_inference(result, &scales, settings.confidence_level)?;
    Ok(LogisticFit {
        coefficients,
        probabilities,
        confidence_level: settings.confidence_level,
        interval_method: IntervalMethod::NormalWald,
        covariance_method: CovarianceMethod::ModelBasedFisherInformation,
        iterations: fitted.n_iter(),
    })
}

fn coefficient_inference(
    result: &anofox_regression::core::RegressionResult,
    scales: &[f64],
    confidence_level: f64,
) -> Result<Vec<Coefficient>, FitError> {
    let standard_errors = result
        .std_errors
        .as_ref()
        .ok_or(FitError::UnavailableInference)?;
    let mut estimates = vec![(
        result.intercept.ok_or(FitError::UnavailableInference)?,
        result
            .intercept_std_error
            .ok_or(FitError::UnavailableInference)?,
    )];
    estimates.extend(
        scales
            .iter()
            .enumerate()
            .map(|(j, scale)| (result.coefficients[j] / scale, standard_errors[j] / scale)),
    );
    let normal = Normal::new(0.0, 1.0).map_err(|_| FitError::UnavailableInference)?;
    let critical = normal.inverse_cdf((1.0 + confidence_level) / 2.0);
    if !critical.is_finite() || critical <= 0.0 {
        return Err(FitError::InvalidSettings);
    }
    estimates
        .into_iter()
        .map(|(estimate, standard_error)| {
            let z_statistic = estimate / standard_error;
            let p_value = 2.0 * normal.sf(z_statistic.abs());
            let confidence_interval = [
                estimate - critical * standard_error,
                estimate + critical * standard_error,
            ];
            if standard_error <= 0.0
                || [
                    estimate,
                    standard_error,
                    z_statistic,
                    p_value,
                    confidence_interval[0],
                    confidence_interval[1],
                ]
                .iter()
                .any(|v| !v.is_finite())
            {
                return Err(FitError::UnavailableInference);
            }
            Ok(Coefficient {
                estimate,
                standard_error,
                z_statistic,
                p_value,
                confidence_interval,
            })
        })
        .collect::<Result<Vec<_>, _>>()
}
