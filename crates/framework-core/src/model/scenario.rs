use crate::Id;
use crate::model::document::Document;
use crate::model::value::ValueObject;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// A named set of value-object values the document can be switched into.
///
/// The spreadsheet answer to "what if the rate were 6%" is a second copy of
/// the workbook, and the third copy is where the model and its assumptions
/// stop agreeing. Here the assumptions stay one set of named objects and the
/// scenario carries only what it *disagrees* about: a value not mentioned
/// keeps the number somebody typed on its card, so adding an assumption to
/// the model does not mean revisiting three scenarios to give it a value.
///
/// Stored as `value object id → raw literal`, in the value's own data type
/// and in the same spelling the card would hold, because that is what
/// evaluation reads. Keying by id rather than by name means renaming a value
/// changes nothing here, exactly as it changes nothing in a formula.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Scenario {
    pub id: Id,
    pub name: String,
    /// The values this scenario disagrees with the base about. A value
    /// missing from the map is not "unset" — it is agreed, and reads its
    /// own `raw`.
    #[serde(default)]
    pub values: BTreeMap<Id, String>,
}

impl Document {
    pub fn scenario(&self, scenario_id: &str) -> Option<&Scenario> {
        self.scenarios
            .iter()
            .find(|scenario| scenario.id == scenario_id)
    }

    pub(crate) fn scenario_mut(&mut self, scenario_id: &str) -> Option<&mut Scenario> {
        self.scenarios
            .iter_mut()
            .find(|scenario| scenario.id == scenario_id)
    }

    /// The scenario the document is currently reading through, if any.
    ///
    /// `None` is the base, which is not a scenario object: the base is what
    /// the value cards say, so it needs no storage and can never be deleted
    /// out from under an activation.
    pub fn current_scenario(&self) -> Option<&Scenario> {
        self.active_scenario
            .as_deref()
            .and_then(|id| self.scenario(id))
    }

    /// The scenario overriding `value_id` right now, or `None` when the
    /// value is reading its own number.
    pub(crate) fn overriding_scenario(&self, value_id: &str) -> Option<&Scenario> {
        self.current_scenario()
            .filter(|scenario| scenario.values.contains_key(value_id))
    }

    /// What a value holds right now: the active scenario's override, or the
    /// literal on its own card.
    ///
    /// Every read of a value's contents goes through here — compilation,
    /// export, and the lineage fingerprint alike — so that "which number is
    /// this" has exactly one answer, and switching scenarios cannot leave
    /// one surface showing the base while another shows the override.
    pub(crate) fn effective_value_raw<'a>(&'a self, value: &'a ValueObject) -> &'a str {
        self.overriding_scenario(&value.id)
            .and_then(|scenario| scenario.values.get(&value.id))
            .map_or(value.raw.as_str(), String::as_str)
    }
}
