//! IRR and XIRR reduce complete cash-flow columns to a scalar. They share the
//! bounded log-rate solver, while this module owns column validation and each
//! function's time convention.
use polars::prelude as pl;

pub(super) fn compile(name: &str, args: Vec<pl::Expr>) -> Result<pl::Expr, String> {
    let name = name.to_string();
    Ok(pl::apply_multiple(
        move |columns| calculate(&name, columns),
        args,
        |_, _| Ok(pl::Field::new("return".into(), pl::DataType::Float64)),
        true,
    ))
}

fn calculate(name: &str, columns: &mut [pl::Column]) -> pl::PolarsResult<pl::Column> {
    let fail = |message: &str| pl::PolarsError::ComputeError(format!("{name}: {message}").into());
    let values = columns[0].strict_cast(&pl::DataType::Float64)?;
    let values = values.f64()?;
    if values.is_empty() || values.null_count() != 0 {
        return Err(fail("cash flows must be nonempty with no missing values"));
    }
    let values: Vec<_> = values.into_no_null_iter().collect();
    if values.iter().any(|value| !value.is_finite()) {
        return Err(fail("cash flows must be finite"));
    }
    let (periods, guess_index) = if name == "xirr" {
        if columns[1].dtype() != &pl::DataType::Date {
            return Err(fail("dates must be a Date column"));
        }
        if columns[1].len() != values.len() || columns[1].null_count() != 0 {
            return Err(fail("each cash flow needs a date"));
        }
        let dates = columns[1].cast(&pl::DataType::Int32)?;
        let dates = dates.i32()?;
        let origin = dates.get(0).ok_or_else(|| fail("dates must be nonempty"))?;
        let mut periods = Vec::with_capacity(values.len());
        for date in dates.into_no_null_iter() {
            if date < origin {
                return Err(fail("a date precedes the first cash-flow date"));
            }
            periods.push((f64::from(date) - f64::from(origin)) / 365.0);
        }
        (periods, 2)
    } else {
        ((0..values.len()).map(|index| index as f64).collect(), 1)
    };
    if columns[guess_index].len() != 1 {
        return Err(fail("guess must be a scalar"));
    }
    let guess = columns[guess_index].strict_cast(&pl::DataType::Float64)?;
    let guess = guess
        .f64()?
        .get(0)
        .ok_or_else(|| fail("guess is missing"))?;
    let terms = grouped_terms(&values, &periods).map_err(fail)?;
    if !terms.iter().any(|term| term.sign < 0.0) || !terms.iter().any(|term| term.sign > 0.0) {
        return Err(fail(
            "cash flows must contain at least one positive and one negative value after combining equal dates",
        ));
    }
    let objective = |log_rate: f64| discounted_total(&terms, log_rate);
    let outcome = super::financial_root::solve(guess, objective).map_err(fail)?;
    debug_assert!(outcome.iterations <= 160);
    debug_assert!(outcome.bracket.0 <= outcome.root && outcome.root <= outcome.bracket.1);
    debug_assert!(outcome.residual.is_finite());
    Ok(pl::Column::new_scalar(
        "return".into(),
        pl::Scalar::new(pl::DataType::Float64, pl::AnyValue::Float64(outcome.root)),
        1,
    ))
}

struct ReturnTerm {
    value: f64,
    sign: f64,
    log_magnitude: f64,
    period: f64,
}

fn grouped_terms(values: &[f64], periods: &[f64]) -> Result<Vec<ReturnTerm>, &'static str> {
    let mut pairs: Vec<_> = periods
        .iter()
        .copied()
        .zip(values.iter().copied())
        .collect();
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut grouped: Vec<(f64, f64, f64)> = Vec::new();
    for (period, value) in pairs {
        if grouped
            .last()
            .is_some_and(|(last_period, _, _)| *last_period == period)
        {
            let (_, total, correction) = grouped.last_mut().unwrap();
            let adjusted = value - *correction;
            let next = *total + adjusted;
            *correction = (next - *total) - adjusted;
            *total = next;
            if !total.is_finite() {
                return Err("cash flows at the same date sum to a non-finite value");
            }
        } else {
            grouped.push((period, value, 0.0));
        }
    }
    let terms: Vec<_> = grouped
        .into_iter()
        .filter(|(_, value, _)| *value != 0.0)
        .map(|(period, value, _)| ReturnTerm {
            value,
            sign: value.signum(),
            log_magnitude: value.abs().ln(),
            period,
        })
        .collect();
    if terms.is_empty() {
        return Err("cash flows make the return indeterminate");
    }
    Ok(terms)
}

fn discounted_total(terms: &[ReturnTerm], log_rate: f64) -> Option<f64> {
    if log_rate == 0.0 {
        let scale = terms
            .iter()
            .map(|term| term.value.abs())
            .max_by(f64::total_cmp)?;
        let mut total = 0.0;
        let mut correction = 0.0;
        for term in terms {
            let adjusted = term.value / scale - correction;
            let next = total + adjusted;
            correction = (next - total) - adjusted;
            total = next;
        }
        return total.is_finite().then_some(total);
    }
    // Normalize every evaluation by its largest term in log space. The scale
    // is positive, so roots and signs are unchanged, while long dated tails
    // cannot overflow near rate=-1 or underflow into a false all-zero curve.
    let largest = terms
        .iter()
        .map(|term| term.log_magnitude - term.period * log_rate)
        .max_by(f64::total_cmp)?;
    let mut total = 0.0;
    let mut correction = 0.0;
    for term in terms {
        let discounted = term.sign * (term.log_magnitude - term.period * log_rate - largest).exp();
        if !discounted.is_finite() {
            return None;
        }
        let adjusted = discounted - correction;
        let next = total + adjusted;
        correction = (next - total) - adjusted;
        total = next;
    }
    total.is_finite().then_some(total)
}
