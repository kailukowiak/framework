//! Discounted cash flows consume a complete series. Their ordering is part of
//! the formula: NPV is periodic in supplied order; XNPV uses the first supplied
//! date as its origin (Excel's convention), not the minimum date in the data.
use polars::prelude as pl;

pub(super) fn compile(name: &str, args: Vec<pl::Expr>) -> Result<pl::Expr, String> {
    let name = name.to_string();
    Ok(pl::apply_multiple(
        move |columns| {
            if name == "mirr" {
                return modified_return(&name, columns);
            }
            let fail =
                |message: &str| pl::PolarsError::ComputeError(format!("{name}: {message}").into());
            if columns[0].len() != 1 {
                return Err(fail("rate must be a scalar"));
            }
            let rate_column = columns[0].strict_cast(&pl::DataType::Float64)?;
            let rate = rate_column
                .f64()?
                .get(0)
                .ok_or_else(|| fail("rate is missing"))?;
            if !rate.is_finite() || rate <= -1.0 {
                return Err(fail("rate must be finite and greater than -1"));
            }
            let values = columns[1].strict_cast(&pl::DataType::Float64)?;
            let values = values.f64()?;
            if values.is_empty() || values.null_count() != 0 {
                return Err(fail("cash flows must be nonempty with no missing values"));
            }
            let dates = if name == "xnpv" {
                if columns[2].dtype() != &pl::DataType::Date {
                    return Err(fail("dates must be a Date column"));
                }
                if columns[2].len() != values.len() || columns[2].null_count() != 0 {
                    return Err(fail("each cash flow needs a date"));
                }
                Some(columns[2].cast(&pl::DataType::Int32)?)
            } else {
                None
            };
            let days = dates.as_ref().map(|c| c.i32()).transpose()?;
            let origin = days.and_then(|d| d.get(0)).unwrap_or(0);
            let mut total = 0.0;
            let mut correction = 0.0;
            for (index, value) in values.into_no_null_iter().enumerate() {
                let period = if let Some(days) = days {
                    let day = days.get(index).unwrap();
                    if day < origin {
                        return Err(fail("a date precedes the first cash-flow date"));
                    }
                    (f64::from(day) - f64::from(origin)) / 365.0
                } else {
                    (index + 1) as f64
                };
                let discounted = value * (-period * rate.ln_1p()).exp();
                if !value.is_finite() || !discounted.is_finite() {
                    return Err(fail("cash flows and discounted results must be finite"));
                }
                // Compensated summation keeps cancellation of large inflows and
                // outflows from unnecessarily erasing the residual being valued.
                let adjusted = discounted - correction;
                let next = total + adjusted;
                correction = (next - total) - adjusted;
                total = next;
            }
            if !total.is_finite() {
                return Err(fail("discounted total is not finite"));
            }
            Ok(pl::Column::new_scalar(
                "npv".into(),
                pl::Scalar::new(pl::DataType::Float64, pl::AnyValue::Float64(total)),
                1,
            ))
        },
        args,
        |_, _| Ok(pl::Field::new("npv".into(), pl::DataType::Float64)),
        true,
    ))
}

fn modified_return(name: &str, columns: &mut [pl::Column]) -> pl::PolarsResult<pl::Column> {
    let fail = |message: &str| pl::PolarsError::ComputeError(format!("{name}: {message}").into());
    let finance = scalar_rate(name, columns, 1, "finance_rate")?;
    let reinvest = scalar_rate(name, columns, 2, "reinvest_rate")?;
    let values = columns[0].strict_cast(&pl::DataType::Float64)?;
    let values = values.f64()?;
    if values.is_empty() || values.null_count() != 0 {
        return Err(fail("cash flows must be nonempty with no missing values"));
    }
    let flows: Vec<f64> = values.into_no_null_iter().collect();
    if flows.iter().any(|value| !value.is_finite()) {
        return Err(fail("cash flows must be finite"));
    }
    if !flows.iter().any(|value| *value > 0.0) || !flows.iter().any(|value| *value < 0.0) {
        return Err(fail(
            "cash flows must contain at least one positive and one negative value",
        ));
    }
    // Negatives discount to time zero at the finance rate; positives
    // compound to the final period at the reinvestment rate. Summation is
    // compensated so a large loan beside small returns keeps its residual.
    let mut present = 0.0;
    let mut present_correction = 0.0;
    let mut future = 0.0;
    let mut future_correction = 0.0;
    let last = (flows.len() - 1) as f64;
    for (index, value) in flows.iter().copied().enumerate() {
        if value < 0.0 {
            let discounted = value * (-(index as f64) * finance.ln_1p()).exp();
            if !discounted.is_finite() {
                return Err(fail("discounted negatives are not finite"));
            }
            let adjusted = discounted - present_correction;
            let next = present + adjusted;
            present_correction = (next - present) - adjusted;
            present = next;
        } else if value > 0.0 {
            let compounded = value * ((last - index as f64) * reinvest.ln_1p()).exp();
            if !compounded.is_finite() {
                return Err(fail("compounded positives are not finite"));
            }
            let adjusted = compounded - future_correction;
            let next = future + adjusted;
            future_correction = (next - future) - adjusted;
            future = next;
        }
    }
    if present >= 0.0 || future <= 0.0 {
        return Err(fail(
            "discounted negatives and compounded positives must be nonzero",
        ));
    }
    let ratio = future / -present;
    let answer = ratio.powf(1.0 / last) - 1.0;
    if !answer.is_finite() {
        return Err(fail("modified return is not finite"));
    }
    Ok(pl::Column::new_scalar(
        "npv".into(),
        pl::Scalar::new(pl::DataType::Float64, pl::AnyValue::Float64(answer)),
        1,
    ))
}

fn scalar_rate(
    name: &str,
    columns: &[pl::Column],
    position: usize,
    label: &str,
) -> pl::PolarsResult<f64> {
    let fail = |message: String| pl::PolarsError::ComputeError(format!("{name}: {message}").into());
    if columns[position].len() != 1 {
        return Err(fail(format!("{label} must be a scalar")));
    }
    let cast = columns[position]
        .strict_cast(&pl::DataType::Float64)
        .map_err(|_| fail(format!("{label} must be numeric")))?;
    let rate = cast
        .f64()
        .map_err(|_| fail(format!("{label} must be numeric")))?
        .get(0)
        .ok_or_else(|| fail(format!("{label} is missing")))?;
    if !rate.is_finite() || rate <= -1.0 {
        return Err(fail(format!("{label} must be finite and greater than -1")));
    }
    Ok(rate)
}
