//! Constructors return an ordinary scalar. The same parser reads their
//! configuration for the UI and for compilation, so the slider cannot offer
//! values that mean something different to a downstream filter. This first
//! slice deliberately requires literal bounds/options: a changing formula
//! must not silently change the domain of a hand-selected assumption.
use crate::*;
use chrono::NaiveDate;

pub(crate) fn is_control(name: &str) -> bool {
    matches!(name, "slider" | "dropdown" | "date_input")
}

pub(crate) fn read(expression: &Expr) -> Result<Option<ParameterControl>, String> {
    let Expr::PolarsCall {
        name,
        arguments,
        keyword_arguments,
    } = expression
    else {
        return Ok(None);
    };
    if !is_control(name) {
        return Ok(None);
    }
    let parameters: &[&str] = match name.as_str() {
        "slider" => &["start", "stop", "step", "value"],
        "dropdown" => &["options", "value"],
        _ => &["value"],
    };
    if arguments.len() > parameters.len() {
        return Err(format!("{name}: too many arguments"));
    }
    let mut slots = vec![None; parameters.len()];
    for (slot, argument) in slots.iter_mut().zip(arguments) {
        *slot = Some(argument);
    }
    for (key, argument) in keyword_arguments {
        let index = parameters
            .iter()
            .position(|p| *p == key)
            .ok_or_else(|| format!("{name}: unknown argument {key}"))?;
        if slots[index].replace(argument).is_some() {
            return Err(format!("{name}: {key} was supplied twice"));
        }
    }
    let required = |index: usize| {
        slots[index].ok_or_else(|| format!("{name}: {} is required", parameters[index]))
    };
    let control = match name.as_str() {
        "slider" => {
            let start = number(required(0)?)?;
            let stop = number(required(1)?)?;
            let step = number(required(2)?)?;
            let value = slots[3].map(number).transpose()?.unwrap_or(start);
            if start >= stop || step <= 0.0 || !(stop - start).is_finite() {
                return Err("slider: start must be below stop and step must be positive".into());
            }
            ParameterControl::Slider {
                start,
                stop,
                step,
                value,
            }
        }
        "dropdown" => {
            let Expr::List { items } = required(0)? else {
                return Err("dropdown: options must be a literal list".into());
            };
            let options = items.iter().map(literal).collect::<Result<Vec<_>, _>>()?;
            let Some(first) = options.first() else {
                return Err("dropdown: options cannot be empty".into());
            };
            if options
                .iter()
                .any(|v| std::mem::discriminant(v) != std::mem::discriminant(first))
            {
                return Err("dropdown: options must have the same type".into());
            }
            if options
                .iter()
                .enumerate()
                .any(|(i, v)| options[..i].contains(v))
            {
                return Err("dropdown: options must be distinct".into());
            }
            let value = slots[1]
                .map(literal)
                .transpose()?
                .unwrap_or_else(|| first.clone());
            ParameterControl::Dropdown { options, value }
        }
        _ => {
            let ScalarValue::Date(value) = literal(required(0)?)? else {
                return Err("date_input: value must be a literal date".into());
            };
            ParameterControl::DateInput {
                value: value.to_string(),
            }
        }
    };
    check_value(&control, &selected(&control))?;
    Ok(Some(control))
}

pub(crate) fn selected(control: &ParameterControl) -> ScalarValue {
    match control {
        ParameterControl::Slider { value, .. } => ScalarValue::Number(*value),
        ParameterControl::Dropdown { value, .. } => value.clone(),
        ParameterControl::DateInput { value } => {
            ScalarValue::Date(NaiveDate::parse_from_str(value, "%Y-%m-%d").expect("validated date"))
        }
    }
}

pub(crate) fn check_value(control: &ParameterControl, value: &ScalarValue) -> Result<(), String> {
    let valid = match (control, value) {
        (ParameterControl::Slider { start, stop, .. }, ScalarValue::Number(v)) => {
            v.is_finite() && v >= start && v <= stop
        }
        (ParameterControl::Dropdown { options, .. }, value) => options.contains(value),
        (ParameterControl::DateInput { .. }, ScalarValue::Date(_)) => true,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err("Selected value is outside the control's range or options".into())
    }
}

pub(crate) fn expression(value: &ScalarValue) -> Result<Expr, String> {
    Ok(match value {
        ScalarValue::Number(value) if value.is_finite() => Expr::Number { value: *value },
        ScalarValue::String(value) => Expr::String {
            value: value.clone(),
        },
        ScalarValue::Date(value) => Expr::Date { value: *value },
        ScalarValue::Boolean(value) => Expr::Boolean { value: *value },
        _ => return Err("A control needs a finite number, text, date, or Boolean".into()),
    })
}

fn number(expression: &Expr) -> Result<f64, String> {
    match literal(expression)? {
        ScalarValue::Number(value) => Ok(value),
        _ => Err("slider: bounds, step and value must be literal numbers".into()),
    }
}

fn literal(expression: &Expr) -> Result<ScalarValue, String> {
    let value = match expression {
        Expr::Number { value } | Expr::Percentage { value } | Expr::Money { value } => {
            ScalarValue::Number(*value)
        }
        Expr::Integer { value } => ScalarValue::Number(*value as f64),
        Expr::String { value } => ScalarValue::String(value.clone()),
        Expr::Date { value } => ScalarValue::Date(*value),
        Expr::Boolean { value } => ScalarValue::Boolean(*value),
        Expr::Negate { expression } => ScalarValue::Number(-number(expression)?),
        Expr::PolarsCall {
            name,
            arguments,
            keyword_arguments,
        } if name == "date" && arguments.len() == 3 && keyword_arguments.is_empty() => {
            let nums = arguments
                .iter()
                .map(number)
                .collect::<Result<Vec<_>, _>>()?;
            if nums.iter().any(|n| n.fract() != 0.0) {
                return Err("date_input: date components must be integers".into());
            }
            ScalarValue::Date(
                NaiveDate::from_ymd_opt(nums[0] as i32, nums[1] as u32, nums[2] as u32)
                    .ok_or("Invalid date")?,
            )
        }
        _ => return Err("Control configuration must use literals, not computed formulas".into()),
    };
    if matches!(value, ScalarValue::Number(v) if !v.is_finite()) {
        return Err("Control numbers must be finite".into());
    }
    Ok(value)
}
