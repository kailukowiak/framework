//! Rate conversions stay Polars expressions; the annuity rate needs a
//! solver per row, so it maps over broadcast inputs with the shared bounded
//! root finder rather than Newton iteration from a guess.
use polars::prelude as pl;
use polars::prelude::NamedFrom;

pub(super) fn compile(name: &str, args: Vec<pl::Expr>) -> Result<pl::Expr, String> {
    match name {
        "effect" | "nominal" => conversion(name, args),
        "rate" => annuity_rate(args),
        _ => Err(format!("Unknown financial function ‘{name}’")),
    }
}

fn conversion(name: &str, args: Vec<pl::Expr>) -> Result<pl::Expr, String> {
    let rate = args[0].clone().strict_cast(pl::DataType::Float64);
    let npery = args[1].clone().strict_cast(pl::DataType::Float64);
    let truncated = npery.clone().floor();
    let valid = rate
        .clone()
        .is_finite()
        .and(npery.clone().is_finite())
        .and(rate.clone().gt(pl::lit(0.0)))
        .and(truncated.clone().gt_eq(pl::lit(1.0)));
    let answer = if name == "effect" {
        (pl::lit(1.0) + rate.clone() / truncated.clone()).pow(truncated.clone()) - pl::lit(1.0)
    } else {
        truncated.clone()
            * ((pl::lit(1.0) + rate.clone()).pow(pl::lit(1.0) / truncated.clone()) - pl::lit(1.0))
    };
    Ok(super::financial::checked(name, answer, valid))
}

fn annuity_rate(args: Vec<pl::Expr>) -> Result<pl::Expr, String> {
    Ok(pl::apply_multiple(
        solve_rows,
        args,
        |_, _| Ok(pl::Field::new("rate".into(), pl::DataType::Float64)),
        true,
    ))
}

fn solve_rows(columns: &mut [pl::Column]) -> pl::PolarsResult<pl::Column> {
    let fail = |message: &str| pl::PolarsError::ComputeError(format!("rate: {message}").into());
    let width = columns.iter().map(pl::Column::len).max().unwrap_or(0);
    let mut out: Vec<Option<f64>> = Vec::with_capacity(width.max(1));
    for index in 0..width.max(1) {
        match solve_row(columns, index) {
            Err(message) => return Err(fail(message)),
            Ok(value) => out.push(value),
        }
    }
    Ok(pl::Series::new("rate".into(), out).into())
}

fn solve_row(columns: &[pl::Column], index: usize) -> Result<Option<f64>, &'static str> {
    let mut inputs = Vec::with_capacity(columns.len());
    for column in columns {
        let cast = column
            .strict_cast(&pl::DataType::Float64)
            .map_err(|_| "inputs must be numeric")?;
        let floats = cast.f64().map_err(|_| "inputs must be numeric")?;
        match floats.get(if floats.len() == 1 { 0 } else { index }) {
            None => return Ok(None),
            Some(value) => inputs.push(value),
        }
    }
    let (nper, pmt, pv, fv, timing, guess) = (
        inputs[0], inputs[1], inputs[2], inputs[3], inputs[4], inputs[5],
    );
    if ![nper, pmt, pv, fv, timing, guess]
        .iter()
        .all(|value| value.is_finite())
    {
        return Err("nper, pmt, pv, fv, type and guess must be finite");
    }
    if nper <= 0.0 {
        return Err("nper must be positive");
    }
    if timing != 0.0 && timing != 1.0 {
        return Err("type must be 0 or 1");
    }
    if guess <= -1.0 {
        return Err("guess must be greater than -1");
    }
    let objective = |log_rate: f64| annuity_residual(nper, pmt, pv, fv, timing, log_rate);
    let outcome =
        super::financial_root::solve(guess, objective).map_err(|_| "no rate root was found")?;
    if !outcome.root.is_finite() || outcome.root <= -1.0 {
        return Err("solved rate is not finite");
    }
    Ok(Some(outcome.root))
}

fn annuity_residual(
    nper: f64,
    pmt: f64,
    pv: f64,
    fv: f64,
    timing: f64,
    log_rate: f64,
) -> Option<f64> {
    if log_rate == 0.0 {
        return Some(fv + pv + pmt * nper);
    }
    let rate = log_rate.exp_m1();
    if !rate.is_finite() || rate <= -1.0 {
        return None;
    }
    // x is n * ln(1 + rate) by construction, so growth avoids a second log.
    let x = nper * log_rate;
    if !x.is_finite() {
        return None;
    }
    let growth = x.exp();
    if !growth.is_finite() {
        return None;
    }
    let excess = x.exp_m1();
    if !excess.is_finite() {
        return None;
    }
    let total = fv + pv * growth + pmt * (1.0 + rate * timing) * excess / rate;
    total.is_finite().then_some(total)
}
