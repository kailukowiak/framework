use crate::{Metrics, MlError, PredictionOutput, Result};

pub(crate) fn finite(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}
pub(crate) fn mean(values: &[f64]) -> f64 {
    values.iter().map(|v| v / values.len() as f64).sum()
}

pub(crate) fn calculate(
    predictions: &PredictionOutput,
    targets: &[f64],
    baseline: Option<f64>,
    class_baseline: Option<&[f64]>,
) -> Result<Metrics> {
    let mut result = Metrics {
        rows: targets.len(),
        ..Metrics::default()
    };
    if let Some(probabilities) = &predictions.probabilities {
        classification(
            &mut result,
            predictions,
            probabilities,
            targets,
            baseline,
            class_baseline,
        )?;
    } else {
        let residual_norm = predictions
            .values
            .iter()
            .zip(targets)
            .fold(0_f64, |norm, (p, y)| norm.hypot(p - y));
        result.rmse = finite(residual_norm / (targets.len() as f64).sqrt());
        result.mae = finite(
            predictions
                .values
                .iter()
                .zip(targets)
                .map(|(p, y)| (p - y).abs() / targets.len() as f64)
                .sum(),
        );
        let target_mean = mean(targets);
        let total_norm = targets
            .iter()
            .fold(0_f64, |norm, y| norm.hypot(y - target_mean));
        result.r_squared = if total_norm > 0. {
            finite(1. - (residual_norm / total_norm).powi(2))
        } else {
            None
        };
        result.baseline_rmse = baseline.and_then(|base| {
            finite(
                targets.iter().fold(0_f64, |norm, y| norm.hypot(y - base))
                    / (targets.len() as f64).sqrt(),
            )
        });
    }
    Ok(result)
}

fn classification(
    result: &mut Metrics,
    predictions: &PredictionOutput,
    probabilities: &[Vec<f64>],
    targets: &[f64],
    baseline: Option<f64>,
    class_baseline: Option<&[f64]>,
) -> Result<()> {
    let classes = probabilities[0].len();
    if targets
        .iter()
        .any(|v| *v < 0. || v.fract() != 0. || *v >= classes as f64)
    {
        return Err(MlError::InvalidInput(
            "Classification targets must be class indices in the fitted model's class order".into(),
        ));
    }
    let n = targets.len() as f64;
    result.accuracy = Some(
        predictions
            .values
            .iter()
            .zip(targets)
            .filter(|(a, b)| a == b)
            .count() as f64
            / n,
    );
    result.log_loss = finite(
        probabilities
            .iter()
            .zip(targets)
            .map(|(p, y)| -p[*y as usize].clamp(1e-15, 1. - 1e-15).ln() / n)
            .sum(),
    );
    let fallback = baseline.filter(|_| classes == 2).map(|p| vec![1. - p, p]);
    if let Some(frequencies) = class_baseline.or(fallback.as_deref()) {
        if frequencies.len() != classes
            || frequencies
                .iter()
                .any(|v| !v.is_finite() || *v < 0. || *v > 1.)
        {
            return Err(MlError::InvalidModel("Invalid saved class baseline".into()));
        }
        let class = frequencies
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1).then_with(|| b.0.cmp(&a.0)))
            .map(|(i, _)| i)
            .unwrap_or(0);
        result.baseline_accuracy =
            Some(targets.iter().filter(|y| **y == class as f64).count() as f64 / n);
        result.baseline_log_loss = finite(
            targets
                .iter()
                .map(|y| -frequencies[*y as usize].clamp(1e-15, 1. - 1e-15).ln() / n)
                .sum(),
        );
    }
    Ok(())
}
