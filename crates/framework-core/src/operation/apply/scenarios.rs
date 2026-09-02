//! Applying already-resolved scenario operations.

use crate::*;

impl Document {
    pub(crate) fn apply_add_scenario(&mut self, scenario: Scenario) -> Result<(), CoreError> {
        if self.scenario(&scenario.id).is_some() {
            return Err(CoreError::InvalidOperation(
                "A scenario with that ID already exists.".into(),
            ));
        }
        self.scenarios.push(scenario);
        Ok(())
    }

    pub(crate) fn apply_remove_scenario(&mut self, scenario_id: Id) -> Result<(), CoreError> {
        let index = self
            .scenarios
            .iter()
            .position(|scenario| scenario.id == scenario_id)
            .ok_or_else(|| {
                CoreError::InvalidOperation("That scenario is no longer in this document.".into())
            })?;
        self.scenarios.remove(index);
        // An activation pointing at nothing would leave the document reading
        // a scenario the switcher cannot name, so deleting the active one
        // returns it to the base as part of the same edit.
        if self.active_scenario.as_deref() == Some(scenario_id.as_str()) {
            self.active_scenario = None;
        }
        Ok(())
    }

    pub(crate) fn apply_rename_scenario(
        &mut self,
        scenario_id: Id,
        name: String,
    ) -> Result<(), CoreError> {
        let scenario = self.scenario_mut(&scenario_id).ok_or_else(|| {
            CoreError::InvalidOperation("That scenario is no longer in this document.".into())
        })?;
        scenario.name = name;
        Ok(())
    }

    pub(crate) fn apply_set_scenario_value(
        &mut self,
        scenario_id: Id,
        value_id: Id,
        raw: Option<String>,
    ) -> Result<(), CoreError> {
        let scenario = self.scenario_mut(&scenario_id).ok_or_else(|| {
            CoreError::InvalidOperation("That scenario is no longer in this document.".into())
        })?;
        match raw {
            Some(raw) => scenario.values.insert(value_id, raw),
            None => scenario.values.remove(&value_id),
        };
        Ok(())
    }

    pub(crate) fn apply_activate_scenario(
        &mut self,
        scenario_id: Option<Id>,
    ) -> Result<(), CoreError> {
        if let Some(scenario_id) = &scenario_id
            && self.scenario(scenario_id).is_none()
        {
            return Err(CoreError::InvalidOperation(
                "That scenario is no longer in this document.".into(),
            ));
        }
        self.active_scenario = scenario_id;
        Ok(())
    }

    pub(crate) fn apply_restore_scenarios(
        &mut self,
        scenarios: Vec<Scenario>,
        active_scenario: Option<Id>,
    ) {
        self.scenarios = scenarios;
        self.active_scenario = active_scenario;
    }

    /// Every scenario forgets what it said about this value.
    ///
    /// Called when a value object is deleted. An override keyed to an id
    /// nothing answers to would be invisible in every interface and would
    /// come back to life if a value were ever recreated with that id, which
    /// undo does — so the undo of a delete puts the whole scenario list back
    /// rather than relying on these surviving.
    pub(crate) fn drop_scenario_overrides(&mut self, value_id: &str) {
        for scenario in &mut self.scenarios {
            scenario.values.remove(value_id);
        }
    }
}
