//! A control is the presentation of a constructor expression, never a second
//! store of parameter values. These records are derived for the interface;
//! only the ordinary formula, including its selected value, is persisted.
use crate::{Id, ScalarValue};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum ParameterControl {
    Slider {
        start: f64,
        stop: f64,
        step: f64,
        value: f64,
    },
    Dropdown {
        options: Vec<ScalarValue>,
        value: ScalarValue,
    },
    DateInput {
        value: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ParameterInput {
    pub id: Id,
    pub name: String,
    pub control: ParameterControl,
}
