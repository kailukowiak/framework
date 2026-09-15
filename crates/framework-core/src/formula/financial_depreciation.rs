//! Straight-line and declining-balance depreciation. Each call answers
//! per row, so a Period column broadcasts the way ipmt and ppmt already do.
//! Validation names the offending argument; missing inputs propagate as blank.
use polars::prelude as pl;
use polars::prelude::NamedFrom;

pub(super) fn compile(name: &str, args: Vec<pl::Expr>) -> Result<pl::Expr, String> {
    let name = name.to_string();
    Ok(pl::apply_multiple(
        move |columns| calculate(&name, columns),
        args,
        |_, _| Ok(pl::Field::new("depreciation".into(), pl::DataType::Float64)),
        true,
    ))
}

fn calculate(name: &str, columns: &mut [pl::Column]) -> pl::PolarsResult<pl::Column> {
    let fail = |message: &str| pl::PolarsError::ComputeError(format!("{name}: {message}").into());
    let width = columns.iter().map(pl::Column::len).max().unwrap_or(0);
    let mut out: Vec<Option<f64>> = Vec::with_capacity(width.max(1));
    for index in 0..width.max(1) {
        match row_value(name, columns, index) {
            Err(message) => return Err(fail(message)),
            Ok(value) => out.push(value),
        }
    }
    Ok(pl::Series::new("depreciation".into(), out).into())
}

fn row_value(
    name: &str,
    columns: &[pl::Column],
    index: usize,
) -> Result<Option<f64>, &'static str> {
    let mut inputs = Vec::with_capacity(columns.len());
    for column in columns {
        let cast = column
            .strict_cast(&pl::DataType::Float64)
            .map_err(|_| "inputs must be numeric")?;
        let floats = cast.f64().map_err(|_| "inputs must be numeric")?;
        let value = floats.get(if floats.len() == 1 { 0 } else { index });
        match value {
            None => return Ok(None),
            Some(value) => inputs.push(value),
        }
    }
    match name {
        "sln" => straight_line(&inputs),
        "db" => declining_balance(&inputs),
        "ddb" => double_declining(&inputs),
        _ => Err("unknown depreciation function"),
    }
    .map(Some)
}

fn straight_line(inputs: &[f64]) -> Result<f64, &'static str> {
    let (cost, salvage, life) = (inputs[0], inputs[1], inputs[2]);
    if !cost.is_finite() || !salvage.is_finite() || !life.is_finite() {
        return Err("cost, salvage and life must be finite");
    }
    if life == 0.0 {
        return Err("life must be nonzero");
    }
    let answer = (cost - salvage) / life;
    if !answer.is_finite() {
        return Err("depreciation is not finite");
    }
    Ok(answer)
}

fn declining_balance(inputs: &[f64]) -> Result<f64, &'static str> {
    let (cost, salvage, life, period, month) =
        (inputs[0], inputs[1], inputs[2], inputs[3], inputs[4]);
    if ![cost, salvage, life, period, month]
        .iter()
        .all(|value| value.is_finite())
    {
        return Err("cost, salvage, life, period and month must be finite");
    }
    if cost <= 0.0 {
        return Err("cost must be positive");
    }
    if !(0.0..=cost).contains(&salvage) {
        return Err("salvage must be between 0 and cost");
    }
    let life = life.trunc();
    let period = period.trunc();
    let month = month.trunc();
    if life < 1.0 {
        return Err("life must be at least 1");
    }
    if !(1.0..=12.0).contains(&month) {
        return Err("month must be between 1 and 12");
    }
    let last = if month == 12.0 { life } else { life + 1.0 };
    if period < 1.0 || period > last {
        return Err("period must be within the depreciation schedule");
    }
    // Excel rounds the fixed rate to three decimals; the Microsoft DB
    // example only reconciles with that rounding in place.
    let rate = 1.0 - (salvage / cost).powf(1.0 / life);
    let rate = (rate * 1000.0).round() / 1000.0;
    if !rate.is_finite() {
        return Err("declining rate is not finite");
    }
    let mut total = 0.0;
    let mut current = 0.0;
    let target = period as i64;
    for step in 1..=target {
        current = if step == 1 {
            cost * rate * month / 12.0
        } else if step as f64 == last && month != 12.0 {
            (cost - total) * rate * (12.0 - month) / 12.0
        } else {
            (cost - total) * rate
        };
        total += current;
    }
    if !current.is_finite() {
        return Err("depreciation is not finite");
    }
    Ok(current)
}

fn double_declining(inputs: &[f64]) -> Result<f64, &'static str> {
    let (cost, salvage, life, period, factor) =
        (inputs[0], inputs[1], inputs[2], inputs[3], inputs[4]);
    if ![cost, salvage, life, period, factor]
        .iter()
        .all(|value| value.is_finite())
    {
        return Err("cost, salvage, life, period and factor must be finite");
    }
    if cost < 0.0 {
        return Err("cost must be nonnegative");
    }
    if salvage < 0.0 || salvage > cost {
        return Err("salvage must be between 0 and cost");
    }
    let life = life.trunc();
    let period = period.trunc();
    if life < 1.0 {
        return Err("life must be at least 1");
    }
    if period < 1.0 || period > life {
        return Err("period must be between 1 and life");
    }
    if factor < 0.0 {
        return Err("factor must be nonnegative");
    }
    // Excel caps each period so book value never drops below salvage:
    // min((cost - prior) * factor / life, cost - salvage - prior).
    let mut prior = 0.0;
    let mut current = 0.0;
    for _ in 1..=(period as i64) {
        current = ((cost - prior) * factor / life).min(cost - salvage - prior);
        prior += current;
    }
    if !current.is_finite() {
        return Err("depreciation is not finite");
    }
    Ok(current)
}
