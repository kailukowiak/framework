//! Resolving the scenario operations: names checked, ids minted, and every
//! override typed against the value it overrides before anything is applied.

use crate::*;
use std::collections::BTreeMap;

impl Document {
    /// A scenario's name, checked the way a person would check it.
    ///
    /// Case-insensitively unique, because the only place these names are
    /// read is a switcher listing them side by side, where `Upside` and
    /// `upside` are not two things — they are one thing and a typo, and
    /// letting both exist means the switcher can never say which is
    /// selected.
    fn scenario_name(&self, name: &str, keeping: Option<&str>) -> Result<String, CoreError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::InvalidOperation(
                "A scenario needs a name — Base, Upside, Downside.".into(),
            ));
        }
        if self.scenarios.iter().any(|scenario| {
            Some(scenario.id.as_str()) != keeping && scenario.name.eq_ignore_ascii_case(name)
        }) {
            return Err(CoreError::InvalidOperation(format!(
                "This document already has a scenario called ‘{name}’."
            )));
        }
        Ok(name.to_string())
    }

    fn require_scenario(&self, scenario_id: &str) -> Result<&Scenario, CoreError> {
        self.scenario(scenario_id).ok_or_else(|| {
            CoreError::InvalidOperation("That scenario is no longer in this document.".into())
        })
    }

    pub(crate) fn prepare_add_scenario(
        &self,
        scenario_id: Option<Id>,
        name: String,
        copy_from: Option<Id>,
    ) -> Result<ReplicatedOperation, CoreError> {
        let name = self.scenario_name(&name, None)?;
        let values = match &copy_from {
            Some(source_id) => self.require_scenario(source_id)?.values.clone(),
            None => BTreeMap::new(),
        };
        let scenario_id = match scenario_id {
            Some(chosen) if self.scenario(&chosen).is_some() => {
                return Err(CoreError::InvalidOperation(
                    "A scenario with that ID already exists.".into(),
                ));
            }
            Some(chosen) => chosen,
            None => id(),
        };
        Ok(ReplicatedOperation::AddScenario {
            scenario: Scenario {
                id: scenario_id,
                name,
                values,
            },
        })
    }

    pub(crate) fn prepare_remove_scenario(
        &self,
        scenario_id: Id,
    ) -> Result<ReplicatedOperation, CoreError> {
        let scenario = self.require_scenario(&scenario_id)?;
        // The same rule that holds a value in place while a formula reads
        // it, asked of a scenario id: `under(`Upside`, …)` holds Upside the
        // way `` `Price` `` holds Price, and the refusal says who is reading.
        if let Some(reader) = self.read_by(&scenario_id, None) {
            return Err(CoreError::InvalidOperation(format!(
                "{reader} reads ‘{}’ with under(…), so it cannot be removed. Change that formula first.",
                scenario.name
            )));
        }
        Ok(ReplicatedOperation::RemoveScenario { scenario_id })
    }

    pub(crate) fn prepare_rename_scenario(
        &self,
        scenario_id: Id,
        name: String,
    ) -> Result<ReplicatedOperation, CoreError> {
        self.require_scenario(&scenario_id)?;
        let name = self.scenario_name(&name, Some(&scenario_id))?;
        Ok(ReplicatedOperation::RenameScenario { scenario_id, name })
    }

    pub(crate) fn prepare_set_scenario_value(
        &self,
        scenario_id: Id,
        value_id: Id,
        raw: Option<String>,
    ) -> Result<ReplicatedOperation, CoreError> {
        self.require_scenario(&scenario_id)?;
        let DataObject::Value(value) = self.object(&value_id)? else {
            return Err(CoreError::InvalidOperation(
                "Only a value can hold a scenario override.".into(),
            ));
        };
        // Blank is how the interface says "no override here": a cleared
        // field and an explicit clear are the same gesture, so they
        // replicate as the same edit rather than as an override holding an
        // empty string that would evaluate to null.
        let raw = raw.filter(|raw| !raw.trim().is_empty());
        if let Some(raw) = &raw {
            validate_value_raw(value, raw)?;
        }
        Ok(ReplicatedOperation::SetScenarioValue {
            scenario_id,
            value_id,
            raw,
        })
    }

    pub(crate) fn prepare_activate_scenario(
        &self,
        scenario_id: Option<Id>,
    ) -> Result<ReplicatedOperation, CoreError> {
        if let Some(scenario_id) = &scenario_id {
            self.require_scenario(scenario_id)?;
        }
        Ok(ReplicatedOperation::ActivateScenario { scenario_id })
    }
}
