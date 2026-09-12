use super::*;

pub(super) fn is_financial(path: &[String]) -> bool {
    matches!(path, [namespace, name] if namespace.eq_ignore_ascii_case("finance")
        && crate::formula::financial::is_financial(name))
}

/// The methods that take a whole list and answer with one value. The set a
/// list may be handed to directly, alongside the functions that take one.
const REDUCING_METHODS: &[&str] = &[
    "sum",
    "mean",
    "median",
    "quantile",
    "min",
    "max",
    "count",
    "len",
    "null_count",
    "n_unique",
    "std",
    "var",
    "product",
    "first",
    "last",
    "at",
];

pub(super) fn reduces(path: &[String]) -> bool {
    match path {
        [name] => REDUCING_METHODS.contains(&name.as_str()),
        [namespace, name] if namespace.eq_ignore_ascii_case("finance") => {
            name.eq_ignore_ascii_case("npv") || name.eq_ignore_ascii_case("xnpv")
        }
        _ => false,
    }
}

/// A fold answers in the same kind its values were — the sum of
/// money is money, the earliest of some dates is a date — except
/// for the ones that answer with a tally, which are numbers
/// whatever they counted. Everything else stays unknown: what a
/// Polars method hands back is a fact about Polars, and guessing
/// at it would put this document's writing on a value it does
/// not own.
pub(super) fn declared_type(
    input: &Expr,
    path: &[String],
    arguments: &[Expr],
    document: &Document,
    scope: &[Column],
) -> Option<DataType> {
    match path {
        [name] if name == "format" => Some(DataType::String),
        // The override, and the only thing in the language whose
        // whole job is to answer this question. Said out loud, it
        // beats whatever the arithmetic worked out — and it is
        // where the chain starts again, because everything above
        // reads this node rather than the one under it.
        [name] if name == "show" => match arguments {
            [Expr::String { value }] => shown_as(value),
            _ => None,
        },
        [name] if matches!(name.as_str(), "count" | "len" | "null_count" | "n_unique") => {
            Some(DataType::Integer)
        }
        [name] if matches!(name.as_str(), "mean" | "median" | "quantile") => input
            .declared_type_among(document, scope)
            .map(|data_type| match data_type {
                DataType::Integer | DataType::Number => DataType::Number,
                other => other,
            }),
        [name] if name == "mode" => input.declared_type_among(document, scope),
        // A fraction of the way along a range, whatever went in.
        // Money normalized is not money -- said here so notation
        // stops at this node rather than riding out on a number
        // that no longer means dollars.
        [name] if name == "normalize" => Some(DataType::Number),
        [namespace, name] if namespace == "str" && name == "to_date" => Some(DataType::Date),
        // Date in, date out — not a guess about Polars but the
        // meaning of the operation: moving or snapping a date
        // cannot answer with anything else, and a date has no
        // notation variants for this to get wrong. Known here so
        // `today().dt.month_start() + 1` can read the `1` as days.
        [namespace, name]
            if namespace == "dt"
                && matches!(
                    name.as_str(),
                    "date" | "month_start" | "month_end" | "offset_by"
                ) =>
        {
            Some(DataType::Date)
        }
        // The calendar parts are counts by the same definitional
        // argument as `count` and `len` above. Declaring them is
        // load-bearing, not cosmetic: `offset_by` reads an
        // integer-typed argument as a day count, and an expression
        // built from `.dt.day()` has to *say* it is an integer for
        // that reading to reach it — left unknown, the raw number
        // once flowed into a string slot and took a frame down
        // with it.
        [namespace, name]
            if namespace == "dt"
                && matches!(
                    name.as_str(),
                    "year"
                        | "iso_year"
                        | "quarter"
                        | "month"
                        | "week"
                        | "weekday"
                        | "day"
                        | "ordinal_day"
                        | "days_in_month"
                ) =>
        {
            Some(DataType::Integer)
        }
        [namespace, _] if namespace.eq_ignore_ascii_case("finance") => Some(DataType::Number),
        path if reduces(path) => input.declared_type_among(document, scope),
        _ => None,
    }
}
