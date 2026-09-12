//! Closed financial formulas remain Polars expressions, so the same function
//! works for one assumption or every row of a loan table. Validation wraps the
//! expression rather than turning invalid arithmetic into a plausible blank.
use crate::{Document, Expr};
use polars::prelude as pl;

pub(crate) fn is_financial(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    matches!(
        name.strip_prefix("finance.").unwrap_or(&name),
        "pv" | "fv"
            | "pmt"
            | "ipmt"
            | "ppmt"
            | "nper"
            | "npv"
            | "xnpv"
            | "irr"
            | "xirr"
            | "effect"
            | "nominal"
            | "sln"
            | "db"
            | "ddb"
            | "rate"
            | "mirr"
            | "period_index"
            | "prior"
            | "ytd"
            | "ttm"
            | "same_period_last_year"
            | "fiscal_year"
            | "fiscal_quarter"
            | "fiscal_period"
            | "period_start"
            | "period_end"
            | "add_periods"
            | "fiscal_week"
            | "workday"
            | "networkdays"
    )
}

pub(crate) fn compile(
    name: &str,
    arguments: &[Expr],
    keywords: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    if let Some(answer) = compile_own_binding(name, arguments, keywords, document)? {
        return Ok(answer);
    }
    let lower = name.to_ascii_lowercase();
    let name = lower.strip_prefix("finance.").unwrap_or(&lower).to_string();
    let args = bind_arguments(&name, arguments, keywords, document)?;
    if matches!(name.as_str(), "irr" | "xirr") {
        return super::financial_return::compile(&name, args);
    }
    if matches!(name.as_str(), "npv" | "xnpv" | "mirr") {
        return super::financial_discount::compile(&name, args);
    }
    if matches!(name.as_str(), "sln" | "db" | "ddb") {
        return super::financial_depreciation::compile(&name, args);
    }
    if matches!(name.as_str(), "effect" | "nominal" | "rate") {
        return super::financial_yield::compile(&name, args);
    }
    if name == "prior" {
        // A `prior` call never compiles to a standalone expression: the
        // engine lifts it into a self-join in `apply_with_columns_step`,
        // where the frame's plan and its declared period column exist.
        // Reaching here means there is no plan to join into — Scratchwork,
        // a filter, a summary — so the error says where it belongs.
        return Err(
            "prior reads a neighbouring period, so it needs a frame with a declared period column. Use it in a calculated column."
                .into(),
        );
    }
    if matches!(name.as_str(), "ytd" | "ttm" | "same_period_last_year") {
        // A window aggregate never compiles to a standalone expression
        // either: the engine lifts it into a self-join in
        // `apply_with_columns_step` alongside `prior`. Reaching here means
        // there is no plan to join into, so the error says where it belongs.
        return Err(format!(
            "{name} reads neighbouring periods, so it needs a frame with a declared period column. Use it in a calculated column."
        ));
    }
    let args: Vec<_> = args
        .into_iter()
        .map(|a| a.strict_cast(pl::DataType::Float64))
        .collect();
    let rate = args[0].clone();
    let timing = args.last().unwrap().clone();
    let finite = args.iter().fold(pl::lit(true), |valid, arg| {
        valid.and(arg.clone().is_finite())
    });
    let valid = finite.and(rate.clone().gt(pl::lit(-1.0))).and(
        timing
            .clone()
            .eq(pl::lit(0.0))
            .or(timing.clone().eq(pl::lit(1.0))),
    );
    let (answer, valid) = if name == "nper" {
        (periods(&args), valid)
    } else {
        let offset = usize::from(matches!(name.as_str(), "ipmt" | "ppmt"));
        let n = args[1 + offset].clone();
        let valid = valid.and(n.clone().gt(pl::lit(0.0)));
        let answer = match name.as_str() {
            "pmt" => payment(&rate, &n, &args[2], &args[3], &timing),
            "fv" => {
                -(args[3].clone() * growth(&rate, &n)
                    + args[2].clone() * annuity(&rate, &n, &timing))
            }
            "pv" => {
                -(args[3].clone() + args[2].clone() * annuity(&rate, &n, &timing))
                    / growth(&rate, &n)
            }
            _ => {
                let per = args[1].clone();
                let pmt = payment(&rate, &n, &args[3], &args[4], &timing);
                let before = per.clone() - pl::lit(1.0);
                let interest = -(args[3].clone() * growth(&rate, &before)
                    + pmt.clone() * annuity(&rate, &before, &timing))
                    * rate.clone();
                let interest = pl::when(timing.clone().eq(pl::lit(1.0)))
                    .then(
                        pl::when(per.clone().eq(pl::lit(1.0)))
                            .then(pl::lit(0.0))
                            .otherwise(interest.clone() / (pl::lit(1.0) + rate.clone())),
                    )
                    .otherwise(interest);
                let answer = if name == "ipmt" {
                    interest
                } else {
                    pmt - interest
                };
                return Ok(checked(
                    &name,
                    answer,
                    valid
                        .and(per.clone().gt_eq(pl::lit(1.0)))
                        .and(per.clone().lt_eq(n))
                        .and(per.clone().eq(per.floor())),
                ));
            }
        };
        (answer, valid)
    };
    Ok(checked(&name, answer, valid))
}

/// The functions that bind their own arguments instead of floating
/// everything to Float64: plan-time month numbers, date-typed inputs, and
/// the native scalars. `None` means the shared binding below applies.
fn compile_own_binding(
    name: &str,
    arguments: &[Expr],
    keywords: &[(String, Expr)],
    document: &Document,
) -> Result<Option<pl::Expr>, String> {
    let lower = name.to_ascii_lowercase();
    let name = lower.strip_prefix("finance.").unwrap_or(&lower);
    if name == "period_index" {
        // Integer literals cannot survive argument binding (it compiles to
        // Polars), so this resolves its month numbers before binding.
        return super::financial_period::compile_period_index(arguments, keywords, document)
            .map(Some);
    }
    if matches!(
        name,
        "fiscal_year"
            | "fiscal_quarter"
            | "fiscal_period"
            | "period_start"
            | "period_end"
            | "add_periods"
    ) {
        // Date-typed arguments cannot survive the Float64 binding, and
        // `fy_start` is a plan-time month number, so these bind their own.
        return super::financial_fiscal::compile(name, arguments, keywords, document).map(Some);
    }
    if matches!(name, "fiscal_week" | "workday" | "networkdays") {
        // Retail weeks and business days evaluate as native scalars over
        // the collected series, the way the return solvers do: 52/53-week
        // rules and holiday skipping are scalar iteration.
        return super::financial_calendar::compile(name, arguments, keywords, document).map(Some);
    }
    Ok(None)
}

fn bind_arguments(
    name: &str,
    arguments: &[Expr],
    keywords: &[(String, Expr)],
    document: &Document,
) -> Result<Vec<pl::Expr>, String> {
    let parameters = parameter_names(name);
    let required = match name {
        "ipmt" | "ppmt" => 4,
        "npv" => 2,
        "irr" => 1,
        "xirr" => 2,
        "effect" | "nominal" => 2,
        "sln" | "mirr" | "rate" => 3,
        "db" | "ddb" => 4,
        "period_index" => 1,
        "prior" => 1,
        "ytd" => 1,
        "ttm" | "same_period_last_year" => 1,
        _ => 3,
    };
    let mut slots = vec![None; parameters.len()];
    if arguments.len() > slots.len() {
        return Err(format!("{name} expects at most {} arguments", slots.len()));
    }
    for (slot, argument) in slots.iter_mut().zip(arguments) {
        *slot = Some(argument);
    }
    for (key, value) in keywords {
        let index = parameters
            .iter()
            .position(|parameter| parameter == key)
            .ok_or_else(|| format!("{name} has no argument ‘{key}’"))?;
        if slots[index].replace(value).is_some() {
            return Err(format!("{name}: ‘{key}’ was supplied twice"));
        }
    }
    slots
        .iter()
        .enumerate()
        .map(|(i, argument)| match argument {
            Some(argument) => argument.to_polars(document),
            None if i < required => Err(format!("{name} expects {}", parameters[i])),
            None => Ok(default_argument(name, parameters[i])),
        })
        .collect::<Result<Vec<_>, _>>()
}

fn default_argument(name: &str, parameter: &str) -> pl::Expr {
    match (name, parameter) {
        ("db", "month") => pl::lit(12.0),
        ("ddb", "factor") => pl::lit(2.0),
        (_, "guess") => pl::lit(0.1),
        (_, "n") => pl::lit(1.0),
        (_, "fy_start") => pl::lit(1.0),
        _ => pl::lit(0.0),
    }
}

pub(super) fn parameter_names(name: &str) -> &'static [&'static str] {
    match name {
        "pv" => &["rate", "nper", "pmt", "fv", "type"],
        "fv" => &["rate", "nper", "pmt", "pv", "type"],
        "pmt" => &["rate", "nper", "pv", "fv", "type"],
        "ipmt" | "ppmt" => &["rate", "per", "nper", "pv", "fv", "type"],
        "nper" => &["rate", "pmt", "pv", "fv", "type"],
        "npv" => &["rate", "values"],
        "xnpv" => &["rate", "values", "dates"],
        "irr" => &["values", "guess"],
        "xirr" => &["values", "dates", "guess"],
        "effect" => &["nominal_rate", "npery"],
        "nominal" => &["effect_rate", "npery"],
        "sln" => &["cost", "salvage", "life"],
        "db" => &["cost", "salvage", "life", "period", "month"],
        "ddb" => &["cost", "salvage", "life", "period", "factor"],
        "rate" => &["nper", "pmt", "pv", "fv", "type", "guess"],
        "mirr" => &["values", "finance_rate", "reinvest_rate"],
        "period_index" => &["date", "fy_start"],
        "prior" => &["expr", "n", "fy_start"],
        "ytd" => &["expr", "fy_start"],
        "ttm" | "same_period_last_year" => &["expr"],
        "fiscal_year" | "fiscal_quarter" | "fiscal_period" => &["date", "fy_start", "calendar"],
        "period_start" | "period_end" => &["date", "calendar"],
        "add_periods" => &["date", "n"],
        "fiscal_week" => &["date", "calendar"],
        "workday" => &["date", "n", "calendar"],
        "networkdays" => &["start_date", "end_date", "calendar"],
        _ => unreachable!(),
    }
}

fn growth(rate: &pl::Expr, n: &pl::Expr) -> pl::Expr {
    (n.clone() * rate.clone().log1p()).exp()
}

fn annuity(rate: &pl::Expr, n: &pl::Expr, timing: &pl::Expr) -> pl::Expr {
    // Near zero, exp(x)-1 loses digits. The series is accurate through x^4
    // here, avoiding cancellation without treating a small rate as zero.
    let x = n.clone() * rate.clone().log1p();
    let excess = pl::when(x.clone().abs().lt(pl::lit(1e-5)))
        .then(
            x.clone()
                * (pl::lit(1.0)
                    + x.clone() / pl::lit(2.0)
                    + x.clone().pow(pl::lit(2.0)) / pl::lit(6.0)
                    + x.clone().pow(pl::lit(3.0)) / pl::lit(24.0)),
        )
        .otherwise(x.exp() - pl::lit(1.0));
    pl::when(rate.clone().eq(pl::lit(0.0)))
        .then(n.clone())
        .otherwise((pl::lit(1.0) + rate.clone() * timing.clone()) * excess / rate.clone())
}

fn payment(
    rate: &pl::Expr,
    n: &pl::Expr,
    pv: &pl::Expr,
    fv: &pl::Expr,
    timing: &pl::Expr,
) -> pl::Expr {
    -(fv.clone() + pv.clone() * growth(rate, n)) / annuity(rate, n, timing)
}

fn periods(args: &[pl::Expr]) -> pl::Expr {
    let (r, p, pv, fv, timing) = (&args[0], &args[1], &args[2], &args[3], &args[4]);
    let adjusted = p.clone() * (pl::lit(1.0) + r.clone() * timing.clone());
    let delta = -(fv.clone() + pv.clone()) * r.clone() / (adjusted + pv.clone() * r.clone());
    pl::when(r.clone().eq(pl::lit(0.0)))
        .then(-(fv.clone() + pv.clone()) / p.clone())
        .otherwise(delta.log1p() / r.clone().log1p())
}

pub(super) fn checked(name: &str, answer: pl::Expr, valid: pl::Expr) -> pl::Expr {
    let name = name.to_string();
    let answer = pl::when(valid.clone().is_null())
        .then(pl::lit(pl::NULL))
        .otherwise(answer);
    pl::map_multiple(
        move |columns| {
            let values = columns[0].f64()?;
            let valid = columns[1].bool()?;
            let length = values.len().max(valid.len());
            for i in 0..length {
                if valid.get(if valid.len() == 1 { 0 } else { i }) == Some(false)
                    || values
                        .get(if values.len() == 1 { 0 } else { i })
                        .is_some_and(|v| !v.is_finite())
                {
                    return Err(pl::PolarsError::ComputeError(format!("{name}: invalid financial arguments or non-finite result; rate must exceed -1, type must be 0 or 1, and periods must be valid").into()));
                }
            }
            Ok(if values.len() == 1 && length > 1 {
                columns[0].new_from_index(0, length)
            } else {
                columns[0].clone()
            })
        },
        [answer, valid],
        |_, fields| {
            Ok(pl::Field::new(
                fields[0].name().clone(),
                pl::DataType::Float64,
            ))
        },
    )
}
