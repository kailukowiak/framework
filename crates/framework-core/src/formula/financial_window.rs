//! Year-to-date, trailing-twelve-month and same-period-last-year reads.
//!
//! These are the declaration-gated aggregations: `ytd` sums the fiscal year
//! so far, `ttm` sums the twelve indexes ending here, and
//! `same_period_last_year` reads the index twelve back. Like `prior` they
//! compile to self-joins on the period index within the declared
//! partitions — never to a positional shift — so shuffled rows and gaps
//! change which periods exist, never which rows are silently read instead.
//! A window sums the periods present; only a window with no readable value
//! at all reads blank.
//!
//! Only `ytd` takes `fy_start`: "since the year started" has to know when
//! the year starts. Twelve indexes back is the same date last year under
//! any year start — shifting the calendar moves every index by the same
//! constant — so `ttm` and `same_period_last_year` honestly take none.
//! All three take `calendar`, because which periods exist at all is a
//! calendar question: under a retail week pattern twelve indexes back is
//! twelve retail blocks, not twelve calendar months.
use crate::*;
use polars::prelude as pl;

use super::financial_period::{
    PeriodBasis, PriorCall, check_period_columns, is_prior_call, join_prior, require_period,
    substitute,
};

enum WindowKind {
    Ytd,
    Ttm,
    SamePeriodLastYear,
}

/// One window call: the value expression and the numbering its bounds
/// count in — the same `PeriodBasis` both sides of the join read.
struct WindowCall<'a> {
    expression: &'a Expr,
    kind: WindowKind,
    basis: PeriodBasis,
}

pub(super) fn is_window_call(expression: &Expr) -> bool {
    match expression {
        Expr::PolarsCall { name, .. } => is_window_name(name),
        Expr::Method { path, .. } => {
            matches!(path.as_slice(), [namespace, name] if namespace.eq_ignore_ascii_case("finance") && is_window_name(name))
        }
        _ => false,
    }
}

fn is_window_name(name: &str) -> bool {
    matches!(
        name.strip_prefix("finance.")
            .unwrap_or(name)
            .to_ascii_lowercase()
            .as_str(),
        "ytd" | "ttm" | "same_period_last_year"
    )
}

fn window_call<'a>(expression: &'a Expr, document: &Document) -> Result<WindowCall<'a>, String> {
    let (name, inner, arguments, keywords) = match expression {
        Expr::PolarsCall {
            name,
            arguments,
            keyword_arguments,
        } => (
            name.clone(),
            None,
            arguments.as_slice(),
            keyword_arguments.as_slice(),
        ),
        Expr::Method {
            input,
            path,
            arguments,
            keyword_arguments,
        } => match path.as_slice() {
            [namespace, name]
                if namespace.eq_ignore_ascii_case("finance") && is_window_name(name) =>
            {
                (
                    name.clone(),
                    Some(input.as_ref()),
                    arguments.as_slice(),
                    keyword_arguments.as_slice(),
                )
            }
            _ => unreachable!("is_window_call only collects window calls"),
        },
        _ => unreachable!("is_window_call only collects window calls"),
    };
    let name = name
        .strip_prefix("finance.")
        .unwrap_or(name.as_str())
        .to_ascii_lowercase();
    let kind = match name.as_str() {
        "ytd" => WindowKind::Ytd,
        "ttm" => WindowKind::Ttm,
        _ => WindowKind::SamePeriodLastYear,
    };
    // The receiver stands in for `expr`, so the remaining positionals are
    // whatever this window still takes. A trailing window has no year
    // boundary to place, so `calendar` is its second parameter and
    // `fy_start` is not one of its parameters at all.
    let parameters: &[&str] = match kind {
        WindowKind::Ytd => &["expr", "fy_start", "calendar"],
        _ => &["expr", "calendar"],
    };
    let mut slots: Vec<Option<&Expr>> = vec![None; parameters.len()];
    let mut positional = Vec::with_capacity(parameters.len());
    if let Some(input) = inner {
        positional.push(input);
    }
    positional.extend(arguments.iter());
    if positional.len() > parameters.len() {
        return Err(format!(
            "{name} expects at most {} arguments",
            parameters.len()
        ));
    }
    for (slot, argument) in slots.iter_mut().zip(positional) {
        *slot = Some(argument);
    }
    for (key, value) in keywords {
        // Naming the missing parameter beats "no argument fy_start": the
        // reason a trailing window has none is worth saying once.
        if key == "fy_start" && !matches!(kind, WindowKind::Ytd) {
            return Err(format!(
                "{name} counts twelve whole periods, so it takes no fy_start"
            ));
        }
        let index = parameters
            .iter()
            .position(|parameter| parameter == key)
            .ok_or_else(|| format!("{name} has no argument ‘{key}’"))?;
        if slots[index].replace(value).is_some() {
            return Err(format!("{name}: ‘{key}’ was supplied twice"));
        }
    }
    let expression = slots[0].ok_or_else(|| format!("{name} expects the expression to read"))?;
    // The numbering comes from the document default when unwritten: a bare
    // `ytd` in a February-start workbook counts from February, and under a
    // retail calendar every window counts the blocks `fiscal_period` does.
    let fy_start = match kind {
        WindowKind::Ytd => slots[1],
        _ => None,
    };
    let basis = PeriodBasis::resolve(&name, fy_start, slots[parameters.len() - 1], document)?;
    Ok(WindowCall {
        expression,
        kind,
        basis,
    })
}

/// Whether the expression holds a period-relative call the engine lifts
/// into a self-join at plan time — and which therefore cannot be compiled
/// or evaluated without a plan. Stored calculated columns ask this before
/// typing themselves so both authoring surfaces accept the same formulas.
pub(crate) fn lifts_period_join(expression: &Expr) -> bool {
    let mut found = false;
    expression.walk(&mut |inner| {
        if !found && (is_prior_call(inner) || is_window_call(inner)) {
            found = true;
        }
    });
    found
}

/// The function name of the first period-relative call inside the
/// expression, so a refusal can name the call rather than the concept.
pub(crate) fn first_lift_name(expression: &Expr) -> Option<String> {
    let mut found = None;
    expression.walk(&mut |inner| {
        if found.is_some() {
            return;
        }
        if is_prior_call(inner) {
            found = Some("prior".to_string());
        } else if is_window_call(inner) {
            found = Some(match inner {
                Expr::PolarsCall { name, .. } => name
                    .strip_prefix("finance.")
                    .unwrap_or(name.as_str())
                    .to_ascii_lowercase(),
                Expr::Method { path, .. } => match path.as_slice() {
                    [_, name] => name.to_ascii_lowercase(),
                    _ => unreachable!("is_window_call only matches two-segment paths"),
                },
                _ => unreachable!("is_window_call only matches calls"),
            });
        }
    });
    found
}

/// Every distinct window call inside `columns`, in first-seen order. Two
/// identical calls share one join. Collected after the prior pre-pass, so
/// a `prior` inside a window already reads its joined column.
fn window_calls(columns: &[DerivedExpression]) -> Vec<&Expr> {
    let mut calls: Vec<&Expr> = Vec::new();
    for column in columns {
        column.expression.walk(&mut |expression| {
            if is_window_call(expression) && !calls.contains(&expression) {
                calls.push(expression);
            }
        });
    }
    calls
}

/// Joins each window aggregate in `columns` into `plan` and returns the
/// columns rewritten to read the joined answers.
///
/// Like the prior pre-pass, every call derives its two sides from the
/// input snapshot and joins its answer back by the declaration's natural
/// keys, keeping the cost linear in the number of calls. A same-period
/// read is the prior machinery with a fixed offset of twelve.
pub(crate) fn join_window_aggregates(
    document: &Document,
    frame_id: &str,
    mut plan: pl::LazyFrame,
    columns: &[DerivedExpression],
) -> Result<(pl::LazyFrame, Vec<DerivedExpression>, Vec<String>), String> {
    let calls = window_calls(columns);
    if calls.is_empty() {
        return Ok((plan, columns.to_vec(), Vec::new()));
    }
    let frame = document
        .frame(frame_id)
        .map_err(|error| error.to_string())?;
    let schema = plan.collect_schema().map_err(|error| error.to_string())?;
    let base = plan;
    let mut acc = base.clone();
    let mut outputs = Vec::with_capacity(calls.len());
    for (index, call) in calls.iter().enumerate() {
        let window = window_call(call, document)?;
        let name = match window.kind {
            WindowKind::Ytd => "ytd",
            WindowKind::Ttm => "ttm",
            WindowKind::SamePeriodLastYear => "same_period_last_year",
        };
        let period = require_period(frame, name)?;
        check_period_columns(frame, period, &schema)?;
        let output = format!("__framework_window_{index}");
        acc = match window.kind {
            WindowKind::SamePeriodLastYear => join_prior(
                document,
                period,
                &base,
                acc,
                &PriorCall {
                    expression: window.expression,
                    n: 12,
                    basis: window.basis.clone(),
                },
                &output,
            )?,
            _ => join_window(document, period, &base, acc, &window, &output)?,
        };
        outputs.push(output);
    }
    let rewritten = columns
        .iter()
        .map(|column| DerivedExpression {
            output_column_id: column.output_column_id.clone(),
            expression: substitute(&column.expression, &calls, &outputs),
        })
        .collect();
    Ok((acc, rewritten, outputs))
}

/// One `ytd` or `ttm` as a range self-join: each row pairs with the rows
/// in its window, which are then summed per row. Both sides derive from
/// the pre-pass snapshot; the answer joins back onto the accumulation by
/// the partition columns and the period column the declaration guarantees
/// unique, so no synthetic row key is needed and row order never enters
/// the answer. The range is index arithmetic throughout.
fn join_window(
    document: &Document,
    period: &FramePeriod,
    base: &pl::LazyFrame,
    acc: pl::LazyFrame,
    window: &WindowCall<'_>,
    output: &str,
) -> Result<pl::LazyFrame, String> {
    let low = format!("{output}_low");
    let high = format!("{output}_high");
    let have = format!("{output}_have");
    let value = format!("{output}_value");
    let count = format!("{output}_count");
    // Both bounds count in the call's own basis. A trailing window would
    // answer the same under any year start — shifting the year start moves
    // every index by one constant — but it must still count the same
    // *periods* the other side of the join counts, which under a retail
    // pattern are blocks of weeks rather than months.
    let index = window.basis.index(pl::col(&period.column_id));
    let high_expr = index.clone();
    let low_expr = match window.kind {
        WindowKind::Ytd => {
            // The index is twelve per fiscal year from a zero-based month,
            // so stripping the within-year remainder finds the year's first
            // index. Modulo stays in integer arithmetic throughout.
            index.clone() - (index.clone() % pl::lit(12))
        }
        _ => index.clone() - pl::lit(11),
    };
    let left = base
        .clone()
        .with_columns([high_expr.alias(high.as_str()), low_expr.alias(low.as_str())]);
    let right = base.clone().select(
        period
            .partition_column_ids
            .iter()
            .map(pl::col)
            .chain([
                pl::col(&period.column_id),
                index.alias(have.as_str()),
                window.expression.to_polars(document)?.alias(value.as_str()),
            ])
            .collect::<Vec<_>>(),
    );
    let partitions: Vec<pl::Expr> = period.partition_column_ids.iter().map(pl::col).collect();
    // With no partitions every row pairs with every row before the range
    // filter; Polars wants that spelled as a cross join, not an equi-join
    // with empty keys.
    let paired = if partitions.is_empty() {
        left.join(
            right,
            Vec::<pl::Expr>::new(),
            Vec::<pl::Expr>::new(),
            pl::JoinArgs::new(pl::JoinType::Cross),
        )
    } else {
        left.join(
            right,
            partitions.clone(),
            partitions,
            pl::JoinArgs::new(pl::JoinType::Inner),
        )
    };
    let paired = paired.filter(
        pl::col(have.as_str())
            .gt_eq(pl::col(low.as_str()))
            .and(pl::col(have.as_str()).lt_eq(pl::col(high.as_str()))),
    );
    // The declaration's natural keys identify each row: the period column
    // is unique within each partition. Nulls are skipped the way a frame
    // `sum` skips them; only a window with no readable value at all reads
    // blank rather than zero.
    let mut natural: Vec<pl::Expr> = period.partition_column_ids.iter().map(pl::col).collect();
    natural.push(pl::col(&period.column_id));
    let summed = paired
        .group_by(natural.clone())
        .agg([
            pl::col(value.as_str()).sum().alias(output),
            pl::col(value.as_str()).count().alias(count.as_str()),
        ])
        .with_columns([pl::when(pl::col(count.as_str()).eq(pl::lit(0)))
            .then(pl::lit(pl::NULL))
            .otherwise(pl::col(output))
            .alias(output)])
        .select(
            natural
                .iter()
                .cloned()
                .chain([pl::col(output)])
                .collect::<Vec<_>>(),
        );
    // The grouped answer holds one row per natural key by construction;
    // the many-to-one validation states that shape out loud, so a future
    // change that could multiply rows fails here instead of duplicating
    // answers. Duplicate periods themselves stay refused by the
    // declaration check, the way they are for `prior`.
    let mut arguments = pl::JoinArgs::new(pl::JoinType::Left);
    arguments.validation = pl::JoinValidation::ManyToOne;
    arguments.maintain_order = pl::MaintainOrderJoin::Left;
    Ok(acc.join(summed, natural.clone(), natural, arguments))
}
