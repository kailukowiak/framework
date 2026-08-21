//! Scalar formatting, parsing, comparison, and the Polars<->framework type
//! mapping — the leaf helpers every other engine module reaches for.

use crate::*;
use chrono::{Duration, NaiveDate};
use polars::prelude as pl;
use polars::prelude::NamedFrom;

pub(crate) fn polars_value_at(series: &pl::Series, index: usize) -> Result<ScalarValue, String> {
    let value = series.get(index).map_err(|error| error.to_string())?;
    Ok(match value {
        pl::AnyValue::Null => ScalarValue::Null,
        pl::AnyValue::Boolean(value) => ScalarValue::Boolean(value),
        pl::AnyValue::String(value) => ScalarValue::String(value.to_string()),
        pl::AnyValue::StringOwned(value) => ScalarValue::String(value.to_string()),
        pl::AnyValue::UInt8(value) => ScalarValue::Number(value as f64),
        pl::AnyValue::UInt16(value) => ScalarValue::Number(value as f64),
        pl::AnyValue::UInt32(value) => ScalarValue::Number(value as f64),
        pl::AnyValue::UInt64(value) => ScalarValue::Number(value as f64),
        pl::AnyValue::Int8(value) => ScalarValue::Number(value as f64),
        pl::AnyValue::Int16(value) => ScalarValue::Number(value as f64),
        pl::AnyValue::Int32(value) => ScalarValue::Number(value as f64),
        pl::AnyValue::Int64(value) => ScalarValue::Number(value as f64),
        pl::AnyValue::Float32(value) => ScalarValue::Number(value as f64),
        pl::AnyValue::Float64(value) => ScalarValue::Number(value),
        pl::AnyValue::Date(days) => ScalarValue::Date(
            NaiveDate::from_ymd_opt(1970, 1, 1)
                .expect("Unix epoch is a valid date")
                .checked_add_signed(Duration::days(days as i64))
                .ok_or_else(|| "Polars produced an out-of-range date".to_string())?,
        ),
        pl::AnyValue::Datetime(value, tu, _) => {
            let ns = match tu {
                pl::TimeUnit::Nanoseconds => value,
                pl::TimeUnit::Microseconds => value * 1_000,
                pl::TimeUnit::Milliseconds => value * 1_000_000,
            };
            let secs = ns.div_euclid(1_000_000_000);
            let nsecs = ns.rem_euclid(1_000_000_000) as u32;
            let dt = chrono::DateTime::from_timestamp(secs, nsecs)
                .ok_or_else(|| "Polars produced an out-of-range datetime".to_string())?;
            ScalarValue::Date(dt.date_naive())
        }
        // A category is stored as a number pointing into its list. Ask for
        // the label rather than formatting the value, which quotes it.
        pl::AnyValue::Categorical(..)
        | pl::AnyValue::CategoricalOwned(..)
        | pl::AnyValue::Enum(..)
        | pl::AnyValue::EnumOwned(..) => ScalarValue::String(
            value
                .get_str()
                .ok_or_else(|| "Polars returned a category with no label".to_string())?
                .to_string(),
        ),
        other => {
            return Err(format!(
                "Polars returned unsupported cell type {}",
                other.dtype()
            ));
        }
    })
}

pub(crate) fn framework_type_from_polars(data_type: &pl::DataType) -> Result<DataType, String> {
    Ok(match data_type {
        pl::DataType::String => DataType::String,
        pl::DataType::Categorical(_, _) | pl::DataType::Enum(_, _) => DataType::Categorical,
        pl::DataType::Boolean => DataType::Boolean,
        pl::DataType::Date | pl::DataType::Datetime(_, _) => DataType::Date,
        pl::DataType::Int8
        | pl::DataType::Int16
        | pl::DataType::Int32
        | pl::DataType::Int64
        | pl::DataType::Int128
        | pl::DataType::UInt8
        | pl::DataType::UInt16
        | pl::DataType::UInt32
        | pl::DataType::UInt64
        | pl::DataType::UInt128 => DataType::Integer,
        pl::DataType::Float16
        | pl::DataType::Float32
        | pl::DataType::Float64
        | pl::DataType::Decimal(_, _) => DataType::Number,
        pl::DataType::Null => DataType::String,
        other => {
            return Err(format!(
                "Polars output type {other} is not yet displayable as a frame column"
            ));
        }
    })
}

pub(crate) fn format_scalar_value(value: &ScalarValue, data_type: DataType) -> String {
    match value {
        ScalarValue::Null => "—".into(),
        ScalarValue::Number(value) => match data_type {
            DataType::Integer => format!("{value:.0}"),
            DataType::Number => format_float(*value),
            DataType::Currency => format!("${value:.2}"),
            // As many figures as the rate has, rather than one: this used to
            // be `{:.1}`, which showed `4.25%` as `4.2%` — a digit somebody
            // typed, dropped on the way to the screen.
            DataType::Percentage => format!("{}%", format_number(value * 100.0)),
            _ => format_number(*value),
        },
        ScalarValue::String(value) => value.clone(),
        ScalarValue::Boolean(value) => value.to_string(),
        ScalarValue::Date(value) => value.format("%Y-%m-%d").to_string(),
    }
}

pub(crate) fn parse_scalar_value(raw: &str, data_type: DataType) -> Result<ScalarValue, String> {
    if raw.trim().is_empty() {
        return Ok(ScalarValue::Null);
    }
    match data_type {
        DataType::String | DataType::Categorical => Ok(ScalarValue::String(raw.to_string())),
        DataType::Integer => parse_integer(raw)
            .map(|value| ScalarValue::Number(value as f64))
            .ok_or_else(|| "Invalid integer value".into()),
        DataType::Number | DataType::Currency | DataType::Percentage => parse_number(raw)
            .map(ScalarValue::Number)
            .ok_or_else(|| format!("Invalid {} value", data_type_name(data_type))),
        DataType::Boolean => parse_boolean(raw)
            .map(ScalarValue::Boolean)
            .ok_or_else(|| "Invalid boolean; use true or false".into()),
        DataType::Date => parse_date(raw)
            .map(ScalarValue::Date)
            .ok_or_else(|| "Invalid date; use YYYY-MM-DD".into()),
    }
}

pub(crate) fn scalar_value_to_raw(value: ScalarValue) -> String {
    match value {
        ScalarValue::Null => String::new(),
        ScalarValue::Number(value) => value.to_string(),
        ScalarValue::String(value) => value,
        ScalarValue::Boolean(value) => value.to_string(),
        ScalarValue::Date(value) => value.format("%Y-%m-%d").to_string(),
    }
}

pub(crate) fn parse_number(raw: &str) -> Option<f64> {
    let trimmed = raw.trim();
    let percentage = trimmed.ends_with('%');
    let normalized = trimmed
        .trim_start_matches('$')
        .trim_end_matches('%')
        .replace([',', ' '], "");
    normalized
        .parse::<f64>()
        .ok()
        .map(|value| if percentage { value / 100.0 } else { value })
}

pub(crate) fn parse_integer(raw: &str) -> Option<i64> {
    raw.trim().replace([',', ' '], "").parse::<i64>().ok()
}

/// A floating-point value as a frame cell: at least two decimal places to
/// distinguish it from an integer, while retaining meaningful precision up
/// to the same practical display ceiling used before integer was a type.
pub(crate) fn format_float(value: f64) -> String {
    let text = format!("{value:.4}");
    let trimmed = text.trim_end_matches('0');
    let decimals = trimmed
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len());
    match decimals {
        0 => format!("{value:.2}"),
        1 => format!("{value:.2}"),
        _ => trimmed.to_string(),
    }
}

pub(crate) fn format_number(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.4}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

pub(crate) fn infer_column_type(grid: &[Vec<String>], index: usize) -> DataType {
    let values: Vec<&str> = grid
        .iter()
        .skip(1)
        .filter_map(|row| row.get(index).map(String::as_str))
        .filter(|value| !value.trim().is_empty())
        .collect();
    if values.iter().all(|value| value.trim().starts_with('$')) && !values.is_empty() {
        DataType::Currency
    } else if values.iter().all(|value| value.trim().ends_with('%')) && !values.is_empty() {
        DataType::Percentage
    } else if values.iter().all(|value| parse_date(value).is_some()) && !values.is_empty() {
        DataType::Date
    } else if values.iter().all(|value| parse_boolean(value).is_some()) && !values.is_empty() {
        DataType::Boolean
    } else if values.iter().all(|value| parse_integer(value).is_some()) && !values.is_empty() {
        DataType::Integer
    } else if values.iter().all(|value| parse_number(value).is_some()) && !values.is_empty() {
        DataType::Number
    } else {
        DataType::String
    }
}

pub(crate) fn infer_data_type(raw: &str) -> DataType {
    if raw.trim().ends_with('%') {
        DataType::Percentage
    } else if raw.trim().starts_with('$') {
        DataType::Currency
    } else if parse_date(raw).is_some() {
        DataType::Date
    } else if parse_boolean(raw).is_some() {
        DataType::Boolean
    } else if parse_integer(raw).is_some() {
        DataType::Integer
    } else if parse_number(raw).is_some() {
        DataType::Number
    } else {
        DataType::String
    }
}

pub(crate) fn data_type_name(data_type: DataType) -> &'static str {
    match data_type {
        DataType::String => "string",
        DataType::Categorical => "categorical",
        DataType::Integer => "integer",
        DataType::Number => "number",
        DataType::Currency => "currency",
        DataType::Percentage => "percentage",
        DataType::Boolean => "boolean",
        DataType::Date => "date",
    }
}

pub(crate) fn normalized_categories(categories: Vec<String>) -> Result<Vec<String>, CoreError> {
    let mut normalized = Vec::new();
    for category in categories {
        let category = category.trim().to_string();
        if category.is_empty() {
            continue;
        }
        if normalized.contains(&category) {
            return Err(CoreError::InvalidOperation(format!(
                "category '{category}' is listed more than once"
            )));
        }
        normalized.push(category);
    }
    if normalized.is_empty() {
        return Err(CoreError::InvalidOperation(
            "categorical columns need at least one category".into(),
        ));
    }
    Ok(normalized)
}

/// A canvas list as the Polars series a formula carries it in.
///
/// Typed by the list's own declared type rather than re-guessed here, so a
/// list of postcodes that happen to be digits stays text once it has been
/// called text.
pub(crate) fn series_to_polars(series: &SeriesObject) -> Result<pl::Series, String> {
    let name = series.name.clone().into();
    let parsed = series
        .values
        .iter()
        .map(|raw| parse_scalar_value(raw, series.data_type))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(match series.data_type {
        DataType::Integer => pl::Series::new(
            name,
            parsed
                .iter()
                .map(|value| match value {
                    ScalarValue::Number(value) => Some(*value as i64),
                    _ => None,
                })
                .collect::<Vec<_>>(),
        ),
        DataType::Number | DataType::Currency | DataType::Percentage => pl::Series::new(
            name,
            parsed
                .iter()
                .map(|value| match value {
                    ScalarValue::Number(value) => Some(*value),
                    _ => None,
                })
                .collect::<Vec<_>>(),
        ),
        DataType::Boolean => pl::Series::new(
            name,
            parsed
                .iter()
                .map(|value| match value {
                    ScalarValue::Boolean(value) => Some(*value),
                    _ => None,
                })
                .collect::<Vec<_>>(),
        ),
        DataType::Date => pl::Series::new(
            name,
            parsed
                .iter()
                .map(|value| match value {
                    ScalarValue::Date(value) => Some(*value),
                    _ => None,
                })
                .collect::<Vec<_>>(),
        ),
        DataType::String | DataType::Categorical => pl::Series::new(
            name,
            parsed
                .iter()
                .map(|value| match value {
                    ScalarValue::String(value) => Some(value.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
        ),
    })
}

/// Reads a list out of whatever text somebody had it in.
///
/// A list is almost never typed from nothing — it is copied out of a
/// spreadsheet column, a Python session, an R session, or a message
/// somebody sent. Those all look different and all mean the same thing, so
/// they are all read rather than one being declared correct and the rest
/// being the user's problem:
///
/// ```text
/// [1, 2, 3]        array([1, 2, 3])      c(1, 2, 3)
/// 1, 2, 3          USD⏎CAD⏎EUR           "a, b", "c"
/// ```
///
/// Newlines win over commas where both appear, because a pasted column is a
/// column and a value in it may well contain a comma. Splitting is
/// quote-aware for the same reason: `"a, b"` is one value, and silently
/// making it two would be a wrong answer rather than a refusal.
pub(crate) fn parse_list_text(text: &str) -> Vec<String> {
    let trimmed = text.trim();
    // A call wrapper first — `array(...)`, `c(...)`, `Series(...)` — then
    // whatever brackets are left. Both are stripped, so `array([1, 2])`
    // needs no special case of its own.
    let inner = trimmed
        .split_once('(')
        .filter(|(head, _)| {
            head.chars()
                .all(|c| c.is_alphanumeric() || c == '.' || c == '_')
        })
        .and_then(|(_, rest)| rest.strip_suffix(')'))
        .unwrap_or(trimmed)
        .trim();
    let inner = inner
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .or_else(|| {
            inner
                .strip_prefix('{')
                .and_then(|rest| rest.strip_suffix('}'))
        })
        .unwrap_or(inner);
    let separator = if inner.contains('\n') { '\n' } else { ',' };
    split_outside_quotes(inner, separator)
        .into_iter()
        .map(|item| unquote(item.trim()))
        .filter(|item| !item.is_empty())
        .collect()
}

/// Splits on `separator`, ignoring separators inside single or double
/// quotes. A backslash escapes the next character.
fn split_outside_quotes(text: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' if quote.is_some() => escaped = true,
            '"' | '\'' if quote == Some(character) => quote = None,
            '"' | '\'' if quote.is_none() => quote = Some(character),
            // A pasted spreadsheet column arrives tab-separated when it was
            // a row, so a tab ends a value whichever separator is in force.
            _ if quote.is_none() && (character == separator || character == '\t') => {
                parts.push(&text[start..index]);
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
}

fn unquote(item: &str) -> String {
    for quote in ['"', '\''] {
        if let Some(inner) = item
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner.replace(&format!("\\{quote}"), &quote.to_string());
        }
    }
    item.to_string()
}

/// The one type that fits every value in a list, by the same reading a
/// column of a pasted grid gets.
pub(crate) fn infer_list_type(values: &[String]) -> DataType {
    let grid: Vec<Vec<String>> = std::iter::once(vec!["list".to_string()])
        .chain(values.iter().map(|value| vec![value.clone()]))
        .collect();
    infer_column_type(&grid, 0)
}

/// The Polars type for a column whose values come from a declared list.
///
/// Polars calls this an `Enum`, and its defining property is that the
/// declared order *is* the order: "Low, Medium, High" sorts the way it
/// reads instead of the way the alphabet falls, `min` is Low rather than
/// High, and `>= "Medium"` means what a person means by it.
///
/// Two columns that declare the same values in the same order get the same
/// type back, so they compare and join without a cast. Two that declare
/// different lists do not — Polars refuses to guess how one order maps onto
/// another, which is the right answer.
pub(crate) fn category_dtype(categories: &[String]) -> Result<pl::DataType, String> {
    let frozen = pl::FrozenCategories::new(categories.iter().map(String::as_str))
        .map_err(|error| error.to_string())?;
    Ok(pl::DataType::from_frozen_categories(frozen))
}

/// The values a Polars type declares, in the order it declares them.
///
/// Empty for every type that does not declare a list — including a plain
/// categorical, which accumulates its values as it meets them and so has no
/// order worth carrying into the document.
pub(crate) fn declared_categories(data_type: &pl::DataType) -> Vec<String> {
    match data_type {
        pl::DataType::Enum(frozen, _) => frozen
            .categories()
            .values_iter()
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

/// The list to start from when a column is first called categorical:
/// everything already in it, alphabetically.
///
/// Alphabetically, because a declared list decides how the column sorts, and
/// calling a column categorical should not quietly rearrange it. Sorted, it
/// keeps sorting the way text did, and only starts meaning something once
/// someone puts the list in an order on purpose.
pub(crate) fn distinct_category_values(column: &Column, rows: &[Row]) -> Vec<String> {
    let mut categories = Vec::new();
    for raw in rows
        .iter()
        .filter_map(|row| row.cells.get(&column.id).map(|cell| &cell.raw))
    {
        if !raw.trim().is_empty() && !categories.contains(raw) {
            categories.push(raw.clone());
        }
    }
    categories.sort();
    categories
}

pub(crate) fn computed_cell(
    result: Result<ScalarValue, String>,
    data_type: DataType,
    is_override: bool,
) -> ComputedCell {
    match result {
        Ok(typed_value) => {
            let value = match typed_value {
                ScalarValue::Number(value) => Some(value),
                _ => None,
            };
            ComputedCell {
                value,
                display: format_scalar_value(&typed_value, data_type),
                typed_value,
                error: None,
                is_override,
            }
        }
        Err(error) => ComputedCell {
            value: None,
            typed_value: ScalarValue::Null,
            display: "—".into(),
            error: Some(error),
            is_override,
        },
    }
}

/// The Polars type a declared column type stores as — the same widths the
/// base scan normalizes to, so a column built from raw text joins cleanly
/// against one read from a file.
pub(crate) fn polars_type_for(data_type: DataType) -> pl::DataType {
    match data_type {
        DataType::Integer => pl::DataType::Int64,
        DataType::Number | DataType::Currency | DataType::Percentage => pl::DataType::Float64,
        DataType::Boolean => pl::DataType::Boolean,
        DataType::Date => pl::DataType::Date,
        DataType::String | DataType::Categorical => pl::DataType::String,
    }
}

/// A typed series built from raw text, one value per raw, parsed exactly
/// the way a frame's own cells parse. Unparseable text is a null, not an
/// error: raws arrive from the document, which stores what was typed.
pub(crate) fn typed_series<'a>(
    name: &str,
    data_type: DataType,
    raws: impl Iterator<Item = &'a str>,
) -> Result<pl::Series, String> {
    let name = pl::PlSmallStr::from(name);
    Ok(match data_type {
        DataType::String | DataType::Categorical => pl::Series::new(
            name,
            raws.map(|raw| (!raw.trim().is_empty()).then(|| raw.to_string()))
                .collect::<Vec<_>>(),
        ),
        DataType::Integer => pl::Series::new(name, raws.map(parse_integer).collect::<Vec<_>>()),
        DataType::Number | DataType::Currency | DataType::Percentage => {
            pl::Series::new(name, raws.map(parse_number).collect::<Vec<_>>())
        }
        DataType::Boolean => pl::Series::new(name, raws.map(parse_boolean).collect::<Vec<_>>()),
        DataType::Date => pl::Series::new(name, raws.map(parse_date).collect::<Vec<_>>()),
    })
}

pub(crate) fn frame_rows_from_polars(frame: &FrameObject, data_frame: &pl::DataFrame) -> Vec<Row> {
    (0..data_frame.height())
        .map(|row_index| Row {
            id: format!("derived:{}:{row_index}", frame.id),
            cells: frame
                .columns
                .iter()
                .map(|column| {
                    let raw = data_frame
                        .column(&column.id)
                        .ok()
                        .and_then(|series| {
                            polars_value_at(series.as_materialized_series(), row_index).ok()
                        })
                        .map(scalar_value_to_raw)
                        .unwrap_or_default();
                    (
                        column.id.clone(),
                        Cell {
                            raw,
                            override_formula: None,
                        },
                    )
                })
                .collect(),
        })
        .collect()
}

pub(crate) fn lazy_row_count(plan: pl::LazyFrame) -> Result<usize, CoreError> {
    let mut schema_plan = plan.clone();
    let schema = schema_plan
        .collect_schema()
        .map_err(|error| CoreError::Import(format!("row count schema: {error}")))?;
    let first_column = schema.iter_names().next().cloned().ok_or_else(|| {
        CoreError::Import("Could not count rows in a frame with no columns".into())
    })?;
    let present = pl::col(first_column.clone()).is_not_null();
    let missing = pl::col(first_column).is_null();
    let frame = plan
        // Count an explicit Int64 expression rather than Polars' `len()`,
        // whose UInt32 partial counts are promoted to UInt128 when pushed
        // through a Union. The parquet executor cannot perform that cast.
        // `is null OR is not null` is true once for every row, including a
        // row whose first value is null.
        .cache()
        .select([present
            .or(missing)
            .cast(pl::DataType::Int64)
            .sum()
            .alias("__framework_row_count")])
        .collect()
        .map_err(|error| CoreError::Import(format!("row count: {error}")))?;
    let series = frame
        .column("__framework_row_count")
        .map_err(|error| CoreError::Import(error.to_string()))?
        .as_materialized_series();
    match polars_value_at(series, 0).map_err(CoreError::Import)? {
        ScalarValue::Number(value) if value >= 0.0 => Ok(value as usize),
        _ => Err(CoreError::Import(
            "Could not read artifact row count".into(),
        )),
    }
}

/// Compatibility ceiling for the legacy eager file-import operation.
/// Desktop imports are normalized into paged artifacts and are not subject to
/// this limit.
pub(crate) fn join_types_compatible(left: DataType, right: DataType) -> bool {
    left == right
        || (matches!(left, DataType::String | DataType::Categorical)
            && matches!(right, DataType::String | DataType::Categorical))
        || (matches!(
            left,
            DataType::Integer | DataType::Number | DataType::Currency | DataType::Percentage
        ) && matches!(
            right,
            DataType::Integer | DataType::Number | DataType::Currency | DataType::Percentage
        ))
}

pub(crate) fn is_numeric_type(data_type: DataType) -> bool {
    matches!(
        data_type,
        DataType::Integer | DataType::Number | DataType::Currency | DataType::Percentage
    )
}

pub(crate) fn formula_types_compatible(left: DataType, right: DataType) -> bool {
    left == right
        || (matches!(left, DataType::String | DataType::Categorical)
            && matches!(right, DataType::String | DataType::Categorical))
        || (is_numeric_type(left) && is_numeric_type(right))
}

pub(crate) fn parse_date(raw: &str) -> Option<NaiveDate> {
    let trimmed = raw.trim();
    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(trimmed, "%Y/%m/%d"))
        .ok()
}

pub(crate) fn parse_boolean(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" => Some(true),
        "false" | "no" => Some(false),
        _ => None,
    }
}

pub(crate) fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::test_support::*;
    #[allow(unused_imports)]
    use crate::*;
    #[allow(unused_imports)]
    use std::{fs, path::PathBuf};
    #[allow(unused_imports)]
    use uuid::Uuid;

    #[test]
    pub(crate) fn polars_dtypes_map_onto_framework_column_types() {
        assert_eq!(
            framework_type_from_polars(&pl::DataType::String).unwrap(),
            DataType::String
        );
        assert_eq!(
            framework_type_from_polars(&pl::DataType::Int64).unwrap(),
            DataType::Integer
        );
        assert_eq!(
            framework_type_from_polars(&pl::DataType::Float64).unwrap(),
            DataType::Number
        );
        assert_eq!(
            framework_type_from_polars(&pl::DataType::Boolean).unwrap(),
            DataType::Boolean
        );
        assert_eq!(
            framework_type_from_polars(&pl::DataType::Date).unwrap(),
            DataType::Date
        );
        assert_eq!(
            framework_type_from_polars(&pl::DataType::Datetime(pl::TimeUnit::Microseconds, None))
                .unwrap(),
            DataType::Date
        );
        // Unmappable dtypes have no direct column type; imports fall back to
        // text by casting the series before extraction.
        assert!(framework_type_from_polars(&pl::DataType::Time).is_err());
    }

    #[test]
    fn integers_and_floats_keep_distinct_types_and_default_renderings() {
        let grid = vec![
            vec!["Count".into(), "Rate".into(), "Ready".into()],
            vec!["12".into(), "12.5".into(), "true".into()],
            vec!["13".into(), "13.125".into(), "false".into()],
        ];
        assert_eq!(infer_column_type(&grid, 0), DataType::Integer);
        assert_eq!(infer_column_type(&grid, 1), DataType::Number);
        assert_eq!(infer_column_type(&grid, 2), DataType::Boolean);
        assert_eq!(
            format_scalar_value(&ScalarValue::Number(12.0), DataType::Integer),
            "12"
        );
        assert_eq!(
            format_scalar_value(&ScalarValue::Number(12.0), DataType::Number),
            "12.00"
        );
        assert_eq!(
            format_scalar_value(&ScalarValue::Number(12.5), DataType::Number),
            "12.50"
        );
        assert!(parse_scalar_value("12.5", DataType::Integer).is_err());
    }
}
