//! Dictionaries are keyed tables, not copies hidden inside expressions.
//!
//! Inside a frame's plan a mapping is a left join against the dictionary's
//! own lazy plan: Polars scans the dictionary as part of the query, so a
//! large keyed table is never collected and pasted into the plan as a
//! literal, and the optimizer sees both sides. The join's own many-to-one
//! validation is the backstop behind the unique-key constraint, and a hit
//! flag carried across the join is what tells a missing key apart from an
//! entry whose value is intentionally null.
//!
//! Only Scratchwork's scalar `lookup`, which has no plan to join into, reads
//! the dictionary directly -- and it filters to the one key it wants before
//! collecting anything.
use crate::*;
use polars::prelude as pl;
use std::collections::HashSet;

/// One `lookup(...)` or `map_values(...)` call, validated against the
/// document: both dictionary columns come from the same frame, and that
/// frame's key column carries a unique-key constraint.
struct Mapping<'a> {
    name: &'a str,
    input: &'a Expr,
    frame_id: &'a str,
    frame_name: String,
    key: &'a str,
    value: &'a str,
    fallback: Option<&'a Expr>,
}

fn is_mapping_call(name: &str) -> bool {
    matches!(name, "lookup" | "map_values")
}

fn mapping_call<'a>(
    name: &'a str,
    arguments: &'a [Expr],
    keywords: &[(String, Expr)],
    document: &Document,
) -> Result<Mapping<'a>, String> {
    if !(3..=4).contains(&arguments.len()) || !keywords.is_empty() {
        return Err(format!(
            "{name}(value, dictionary.Key, dictionary.Value, fallback?) expects 3 or 4 positional arguments"
        ));
    }
    let (
        Expr::ForeignColumn {
            frame_id,
            column_id: key,
        },
        Expr::ForeignColumn {
            frame_id: value_frame,
            column_id: value,
        },
    ) = (&arguments[1], &arguments[2])
    else {
        return Err("Choose key and value columns from the same dictionary table".into());
    };
    if frame_id != value_frame || key == value {
        return Err("Choose distinct key and value columns from the same dictionary table".into());
    }
    let frame = document
        .frame(frame_id)
        .map_err(|error| error.to_string())?;
    if !frame
        .unique_keys
        .iter()
        .any(|constraint| constraint.column_ids == [key.clone()])
    {
        return Err(format!(
            "Mark the key column in ‘{}’ as a unique key before using it as a dictionary",
            frame.name
        ));
    }
    Ok(Mapping {
        name,
        input: &arguments[0],
        frame_id,
        frame_name: frame.name.clone(),
        key,
        value,
        fallback: arguments.get(3),
    })
}

/// The dictionary's two columns as a lazy plan over its data layer, so a
/// display filter on the dictionary never removes an entry, and the
/// dictionary's own chain -- a derived or imported one included -- runs as
/// part of whatever plan reads it.
fn dictionary_plan(document: &Document, mapping: &Mapping<'_>) -> Result<pl::LazyFrame, String> {
    Ok(document
        .materialize_frame_lazy(mapping.frame_id, Layer::Data, &mut HashSet::new())?
        .select([pl::col(mapping.key), pl::col(mapping.value)]))
}

fn is_text(data_type: &pl::DataType) -> bool {
    matches!(
        data_type,
        pl::DataType::String | pl::DataType::Categorical(..) | pl::DataType::Enum(..)
    )
}

/// Casts that make a value and a dictionary key comparable. Text and
/// categories compare as labels; numbers of differing widths compare as
/// floats; anything else takes the value's own type, so a mismatch that
/// cannot be reconciled is refused by the cast rather than silently missed.
fn comparable_casts(left: &pl::DataType, right: &pl::DataType) -> (pl::DataType, pl::DataType) {
    if is_text(left) || is_text(right) {
        (pl::DataType::String, pl::DataType::String)
    } else if left != right && left.is_primitive_numeric() && right.is_primitive_numeric() {
        (pl::DataType::Float64, pl::DataType::Float64)
    } else {
        (left.clone(), left.clone())
    }
}

/// Every distinct mapping call inside `columns`, in first-seen order. Two
/// identical calls share one join.
fn mapping_calls(columns: &[DerivedExpression]) -> Vec<&Expr> {
    let mut calls: Vec<&Expr> = Vec::new();
    for column in columns {
        column.expression.walk(&mut |expression| {
            if let Expr::PolarsCall { name, .. } = expression
                && is_mapping_call(name)
                && !calls.contains(&expression)
            {
                calls.push(expression);
            }
        });
    }
    calls
}

/// `expression` with each mapping call replaced by the column its join
/// produced.
fn substitute(expression: &Expr, calls: &[&Expr], outputs: &[String]) -> Expr {
    if let Some(index) = calls.iter().position(|call| *call == expression) {
        return Expr::Column {
            column_id: outputs[index].clone(),
        };
    }
    let boxed = |inner: &Expr| Box::new(substitute(inner, calls, outputs));
    let each = |items: &[Expr]| {
        items
            .iter()
            .map(|item| substitute(item, calls, outputs))
            .collect()
    };
    let keyed = |items: &[(String, Expr)]| {
        items
            .iter()
            .map(|(keyword, item)| (keyword.clone(), substitute(item, calls, outputs)))
            .collect()
    };
    match expression {
        Expr::List { items } => Expr::List { items: each(items) },
        Expr::Negate { expression } => Expr::Negate {
            expression: boxed(expression),
        },
        Expr::Not { expression } => Expr::Not {
            expression: boxed(expression),
        },
        Expr::Binary {
            operator,
            left,
            right,
        } => Expr::Binary {
            operator: *operator,
            left: boxed(left),
            right: boxed(right),
        },
        Expr::PolarsCall {
            name,
            arguments,
            keyword_arguments,
        } => Expr::PolarsCall {
            name: name.clone(),
            arguments: each(arguments),
            keyword_arguments: keyed(keyword_arguments),
        },
        Expr::Method {
            input,
            path,
            arguments,
            keyword_arguments,
        } => Expr::Method {
            input: boxed(input),
            path: path.clone(),
            arguments: each(arguments),
            keyword_arguments: keyed(keyword_arguments),
        },
        other => other.clone(),
    }
}

/// Joins each mapping in `columns` into `plan` and returns the columns
/// rewritten to read the joined answers, plus the answer columns' names so
/// the caller can drop them once the step has used them.
///
/// The step's own expressions still compile through `to_polars`; only the
/// mapping calls inside them are lifted out, because they are the one part
/// of a formula that needs another frame in the plan rather than a literal.
pub(crate) fn join_mappings(
    document: &Document,
    plan: pl::LazyFrame,
    columns: &[DerivedExpression],
) -> Result<(pl::LazyFrame, Vec<DerivedExpression>, Vec<String>), String> {
    let calls = mapping_calls(columns);
    if calls.is_empty() {
        return Ok((plan, columns.to_vec(), Vec::new()));
    }
    let mut plan = plan;
    let mut outputs = Vec::with_capacity(calls.len());
    for (index, call) in calls.iter().enumerate() {
        let Expr::PolarsCall {
            name,
            arguments,
            keyword_arguments,
        } = call
        else {
            unreachable!("mapping_calls only collects calls");
        };
        let mapping = mapping_call(name, arguments, keyword_arguments, document)?;
        let output = format!("__framework_map_{index}");
        plan = join_mapping(document, plan, &mapping, &output)?;
        outputs.push(output);
    }
    let rewritten = columns
        .iter()
        .map(|column| DerivedExpression {
            output_column_id: column.output_column_id.clone(),
            expression: substitute(&column.expression, &calls, &outputs),
        })
        .collect();
    Ok((plan, rewritten, outputs))
}

fn join_mapping(
    document: &Document,
    plan: pl::LazyFrame,
    mapping: &Mapping<'_>,
    output: &str,
) -> Result<pl::LazyFrame, String> {
    let key = format!("{output}_key");
    let value = format!("{output}_value");
    let hit = format!("{output}_hit");
    let mut plan = plan.with_columns([mapping.input.to_polars(document)?.alias(key.as_str())]);
    let mut dictionary = dictionary_plan(document, mapping)?;
    let left_type = plan
        .collect_schema()
        .map_err(|error| error.to_string())?
        .get(key.as_str())
        .cloned()
        .ok_or_else(|| {
            format!(
                "Could not determine the type of the value given to {}",
                mapping.name
            )
        })?;
    let dictionary_schema = dictionary
        .collect_schema()
        .map_err(|error| error.to_string())?;
    let right_type = dictionary_schema
        .get(mapping.key)
        .cloned()
        .ok_or_else(|| format!("Dictionary ‘{}’ has no key column", mapping.frame_name))?;
    let value_type = dictionary_schema
        .get(mapping.value)
        .cloned()
        .ok_or_else(|| format!("Dictionary ‘{}’ has no value column", mapping.frame_name))?;
    let (left_cast, right_cast) = comparable_casts(&left_type, &right_type);
    plan = plan.with_columns([pl::col(key.as_str()).cast(left_cast)]);
    dictionary = dictionary.select([
        pl::col(mapping.key).cast(right_cast).alias(key.as_str()),
        pl::col(mapping.value).alias(value.as_str()),
        pl::lit(true).alias(hit.as_str()),
    ]);
    let mut arguments = pl::JoinArgs::new(pl::JoinType::Left);
    arguments.validation = pl::JoinValidation::ManyToOne;
    arguments.maintain_order = pl::MaintainOrderJoin::Left;
    // A null key is a value like any other, so a dictionary entry for it
    // applies -- the same reading the catalog documents for these calls.
    arguments.nulls_equal = true;
    plan = plan.join(
        dictionary,
        [pl::col(key.as_str())],
        [pl::col(key.as_str())],
        arguments,
    );
    let matched = pl::col(hit.as_str()).is_not_null();
    let miss = match mapping.fallback {
        Some(fallback) => fallback.to_polars(document)?,
        None if mapping.name == "map_values" => mapping.input.to_polars(document)?,
        // A strict lookup: the plan fails, naming the first key it could
        // not find, instead of quietly handing back a null.
        None => strict_miss(&key, &matched, &mapping.frame_name, value_type),
    };
    let answer = pl::when(matched)
        .then(pl::col(value.as_str()))
        .otherwise(miss)
        .alias(output);
    Ok(plan
        .with_columns([answer])
        .drop(pl::cols([key.as_str(), value.as_str(), hit.as_str()])))
}

/// The keys the join did not find, as an expression that errors when there
/// are any and is otherwise an all-null column of the dictionary's value
/// type, so it sits in `otherwise` without changing the answer's type.
fn strict_miss(
    key: &str,
    matched: &pl::Expr,
    frame_name: &str,
    value_type: pl::DataType,
) -> pl::Expr {
    let frame_name = frame_name.to_string();
    let output_type = value_type.clone();
    pl::when(matched.clone())
        .then(pl::lit(pl::NULL))
        .otherwise(pl::col(key))
        .map(
            move |column: pl::Column| {
                let missing = column.as_materialized_series();
                if let Some(index) = missing.iter().position(|value| !value.is_null()) {
                    let value = missing.get(index)?;
                    polars::prelude::polars_bail!(
                        ComputeError: "lookup found no entry for ‘{}’ in ‘{}’. Add it to the dictionary, or give lookup a fallback value.",
                        value.str_value(),
                        frame_name
                    );
                }
                Ok(pl::Column::full_null(
                    column.name().clone(),
                    column.len(),
                    &value_type,
                ))
            },
            move |_: &pl::Schema, field: &pl::Field| {
                Ok(pl::Field::new(field.name().clone(), output_type.clone()))
            },
        )
}

/// The scalar path, for a Scratchwork line or a result with no frame plan
/// to join into. A literal key filters the dictionary before anything is
/// read; a series input still has to see every entry.
pub(crate) fn compile_mapping(
    name: &str,
    arguments: &[Expr],
    keywords: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    let mapping = mapping_call(name, arguments, keywords, document)?;
    let mut plan = dictionary_plan(document, &mapping)?;
    let key_type = plan
        .collect_schema()
        .map_err(|error| error.to_string())?
        .get(mapping.key)
        .cloned()
        .ok_or_else(|| format!("Dictionary ‘{}’ has no key column", mapping.frame_name))?;
    let literal = match mapping.input {
        Expr::String { value } => Some(pl::lit(value.clone())),
        Expr::Integer { value } => Some(pl::lit(*value)),
        Expr::Number { value } | Expr::Percentage { value } | Expr::Money { value } => {
            Some(pl::lit(*value))
        }
        Expr::Boolean { value } => Some(pl::lit(*value)),
        Expr::Date { value } => Some(pl::lit(*value)),
        _ => None,
    };
    if let Some(literal) = literal {
        let (input_cast, key_cast) = comparable_casts(&literal_type(mapping.input), &key_type);
        plan = plan.filter(
            pl::col(mapping.key)
                .cast(key_cast)
                .eq(literal.cast(input_cast)),
        );
    }
    let data = plan.collect().map_err(|error| error.to_string())?;
    let keys = data
        .column(mapping.key)
        .map_err(|error| error.to_string())?
        .as_materialized_series();
    if keys.n_unique().map_err(|error| error.to_string())? != keys.len() {
        return Err(format!(
            "Dictionary ‘{}’ has duplicate keys; fix them before mapping values",
            mapping.frame_name
        ));
    }
    let values = data
        .column(mapping.value)
        .map_err(|error| error.to_string())?
        .as_materialized_series();
    let input = mapping.input.to_polars(document)?;
    let lookup_input = if is_text(&key_type) {
        input.clone().cast(pl::DataType::String)
    } else {
        input.clone()
    };
    let fallback = mapping
        .fallback
        .map(|arg| arg.to_polars(document))
        .transpose()?
        .or_else(|| (name == "map_values").then(|| input.clone()));
    Ok(lookup_input.replace_strict(
        pl::lit(keys.clone()).implode(true),
        pl::lit(values.clone()).implode(true),
        fallback,
        None::<pl::DataType>,
    ))
}

fn literal_type(expression: &Expr) -> pl::DataType {
    match expression {
        Expr::String { .. } => pl::DataType::String,
        Expr::Integer { .. } => pl::DataType::Int64,
        Expr::Boolean { .. } => pl::DataType::Boolean,
        Expr::Date { .. } => pl::DataType::Date,
        _ => pl::DataType::Float64,
    }
}

/// A draft formula must not introduce a back edge through a dictionary.
/// The dictionary's derivation and formula dependencies both matter: a
/// table can read the edited frame without having a direct source relation.
pub(crate) fn check_dictionary_cycle(
    document: &Document,
    start: &str,
    target: &str,
) -> Result<(), CoreError> {
    let mut todo = vec![start.to_string()];
    let mut seen = HashSet::new();
    while let Some(id) = todo.pop() {
        if id == target {
            return Err(CoreError::Formula(
                "That dictionary lookup creates a circular dependency".into(),
            ));
        }
        if !seen.insert(id.clone()) {
            continue;
        }
        let frame = document.frame(&id)?;
        if let Some(derivation) = &frame.derivation {
            todo.push(derivation.source_frame_id.clone());
        }
        todo.extend(frame.lookup_frame_ids());
        todo.extend(frame.foreign_frames().into_iter().map(str::to_string));
    }
    Ok(())
}
