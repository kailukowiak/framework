//! Root-call compilation, separate from expression and method lowering.
use super::*;

pub(super) fn compile_polars_root_call(
    name: &str,
    arguments: &[Expr],
    keyword_arguments: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    if crate::formula::controls::is_control(name) {
        let call = Expr::PolarsCall {
            name: name.into(),
            arguments: arguments.to_vec(),
            keyword_arguments: keyword_arguments.to_vec(),
        };
        let control = crate::formula::controls::read(&call)?.expect("known constructor");
        return crate::formula::controls::expression(&crate::formula::controls::selected(
            &control,
        ))?
        .to_polars(document);
    }
    if matches!(name, "under" | "solve") {
        return crate::formula::overrides::compile(name, arguments, keyword_arguments, document);
    }
    if crate::formula::financial::is_financial(name) {
        return crate::formula::financial::compile(name, arguments, keyword_arguments, document);
    }
    if matches!(name, "lookup" | "map_values") {
        return crate::formula::dictionary::compile_mapping(
            name,
            arguments,
            keyword_arguments,
            document,
        );
    }
    if name == "previous" {
        if !arguments.is_empty() || !keyword_arguments.is_empty() {
            return Err("previous() takes no arguments".into());
        }
        return Ok(pl::col(crate::formula::ast::PREVIOUS_RESULT_COLUMN_ID));
    }
    if name == "recur" {
        return Err(
            "recur is a Calculate down rows transformation, not an ordinary row formula".into(),
        );
    }
    validate_keywords(name, keyword_arguments)?;
    // Before the arguments are flattened: a pattern and its values are a
    // run of operands, and spreading a list across them would fill one hole
    // with three things.
    if name == "format" {
        return compile_format(arguments, keyword_arguments, document);
    }
    if name == "sequence" {
        return compile_sequence(arguments, keyword_arguments, document);
    }
    let args = flatten_polars_arguments(arguments, document)?;
    match name {
        "sum_horizontal" => {
            let ignore_nulls = horizontal_ignore_nulls(keyword_arguments)?;
            polars::lazy::dsl::sum_horizontal(&args, ignore_nulls)
                .map_err(|error| error.to_string())
        }
        "min_horizontal" => {
            polars::lazy::dsl::min_horizontal(&args).map_err(|error| error.to_string())
        }
        "max_horizontal" => {
            polars::lazy::dsl::max_horizontal(&args).map_err(|error| error.to_string())
        }
        "mean_horizontal" => {
            polars::lazy::dsl::mean_horizontal(&args, horizontal_ignore_nulls(keyword_arguments)?)
                .map_err(|error| error.to_string())
        }
        "coalesce" => Ok(pl::coalesce(&args)),
        "date" => {
            if args.len() != 3 {
                return Err("date(...) expects year, month, and day".into());
            }
            Ok(pl::datetime(pl::DatetimeArgs::new(
                args[0].clone(),
                args[1].clone(),
                args[2].clone(),
            ))
            .dt()
            .date())
        }
        // Read here rather than baked into the stored formula, so a saved
        // filter keeps meaning what it says. `to_polars` runs each time the
        // plan is built, so "the last 30 days" is the last 30 days from
        // whenever the frame is read — not from the afternoon it was
        // written. A frozen frame is the exception, and is meant to be:
        // holding still is what freezing one is for.
        "today" => {
            if !args.is_empty() {
                return Err("today() takes no arguments".into());
            }
            Ok(pl::lit(chrono::Local::now().date_naive()))
        }
        "now" => {
            if !args.is_empty() {
                return Err("now() takes no arguments".into());
            }
            Ok(pl::lit(chrono::Local::now().naive_local()))
        }
        "frame_len" => {
            if !args.is_empty() {
                return Err("frame.len() takes no arguments".into());
            }
            Ok(pl::len())
        }
        "when" => Err("when(...) must be followed by .then(...).otherwise(...)".into()),
        _ => crate::formula::generated_bindings::compile_generated_root_call(
            name,
            arguments,
            keyword_arguments,
            document,
        )
        .unwrap_or_else(|| Err(format!("Unsupported Polars function ‘{name}’"))),
    }
}

fn validate_keywords(name: &str, keyword_arguments: &[(String, Expr)]) -> Result<(), String> {
    if !keyword_arguments.is_empty()
        && !matches!(name, "sum_horizontal" | "mean_horizontal" | "sequence")
    {
        return Err(format!("{name} does not accept these keyword arguments"));
    }
    if matches!(name, "sum_horizontal" | "mean_horizontal")
        && keyword_arguments
            .iter()
            .any(|(keyword, _)| keyword != "ignore_nulls")
    {
        return Err(format!(
            "{name} only accepts the keyword argument ignore_nulls"
        ));
    }
    Ok(())
}

/// The result type a root call declares before Polars resolves it, for the
/// few calls whose answer is knowable from their arguments alone.
pub(crate) fn polars_call_declared_type(
    name: &str,
    arguments: &[Expr],
    document: &Document,
    scope: &[crate::Column],
) -> Option<DataType> {
    match name.strip_prefix("finance.").unwrap_or(name) {
        "slider" => Some(DataType::Number),
        "date_input" => Some(DataType::Date),
        // A period-relative read answers whatever it was asked for — the
        // sum of money is money — so it declares its value's type. This is
        // what lets a stored calculated column type itself without
        // evaluating a join that only the plan can answer.
        "prior" | "ytd" | "ttm" | "same_period_last_year" => arguments
            .first()
            .and_then(|value| value.declared_type_among(document, scope)),
        "fiscal_week" | "networkdays" => Some(DataType::Integer),
        "workday" => Some(DataType::Date),
        "recur" => arguments
            .first()
            .and_then(|seed| seed.declared_type_among(document, scope))
            // Integer seeds can grow fractional results. Let the resolved
            // Polars schema report that type rather than forcing integer display.
            .filter(|kind| *kind != DataType::Integer),
        "format" => Some(DataType::String),
        "today" => Some(DataType::Date),
        // An answer under a scenario is the same kind of thing as the
        // answer itself; a solved input is always a number, whatever the
        // card's type, because the search bisects a continuous value.
        "under" => arguments
            .get(1)
            .and_then(|expression| expression.declared_type_among(document, scope)),
        "solve" => Some(DataType::Number),
        _ => None,
    }
}
