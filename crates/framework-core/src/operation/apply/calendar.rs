//! Applying already-resolved document-level fiscal calendar edits.

use crate::*;

impl Document {
    pub(crate) fn apply_add_calendar(&mut self, calendar: Calendar) -> Result<(), CoreError> {
        if self
            .calendars
            .iter()
            .any(|existing| existing.id == calendar.id)
        {
            return Err(CoreError::InvalidOperation(format!(
                "There is already a calendar with id ‘{}’.",
                calendar.id
            )));
        }
        self.calendars.push(calendar);
        Ok(())
    }

    pub(crate) fn apply_update_calendar(&mut self, calendar: Calendar) -> Result<(), CoreError> {
        let slot = self
            .calendars
            .iter_mut()
            .find(|existing| existing.id == calendar.id)
            .ok_or_else(|| {
                CoreError::InvalidOperation(format!(
                    "There is no calendar with id ‘{}’.",
                    calendar.id
                ))
            })?;
        *slot = calendar;
        Ok(())
    }

    pub(crate) fn apply_remove_calendar(&mut self, calendar_id: Id) -> Result<(), CoreError> {
        let before = self.calendars.len();
        self.calendars.retain(|calendar| calendar.id != calendar_id);
        if self.calendars.len() == before {
            return Err(CoreError::InvalidOperation(format!(
                "There is no calendar with id ‘{calendar_id}’."
            )));
        }
        if self.default_calendar_id.as_deref() == Some(calendar_id.as_str()) {
            self.default_calendar_id = None;
        }
        Ok(())
    }

    pub(crate) fn apply_set_default_calendar(
        &mut self,
        calendar_id: Option<Id>,
    ) -> Result<(), CoreError> {
        if let Some(id) = &calendar_id
            && !self.calendars.iter().any(|calendar| &calendar.id == id)
        {
            return Err(CoreError::InvalidOperation(format!(
                "There is no calendar with id ‘{id}’."
            )));
        }
        self.default_calendar_id = calendar_id;
        Ok(())
    }
}
