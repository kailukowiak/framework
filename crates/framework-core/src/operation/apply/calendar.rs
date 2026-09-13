//! Applying already-resolved document-level fiscal calendar edits.
//!
//! "Already resolved" is not the same as "already checked". A replicated
//! operation arrives from another replica, from undo, or from a `.fw`
//! someone edited by hand, and none of those ran `prepare`. A calendar
//! whose holiday list does not parse installs silently on this path and
//! then quietly turns every bare fiscal call into January arithmetic, so
//! the two writing arms re-run the same validation `prepare` does. It is
//! a handful of string parses against an operation that already touches
//! the whole document — the cost is not the consideration.

use crate::model::calendar::validate_calendar;
use crate::*;

impl Document {
    /// The checks `prepare` makes, re-made against an incoming calendar:
    /// its own fields parse, and its name is still unique.
    fn check_calendar(&self, calendar: &Calendar) -> Result<(), CoreError> {
        validate_calendar(
            &calendar.name,
            calendar.fy_start,
            calendar.pattern,
            &calendar.year_end,
            &calendar.weekend,
            &calendar.holidays,
        )
        .map_err(CoreError::InvalidOperation)?;
        if self.calendars.iter().any(|existing| {
            existing.id != calendar.id && existing.name.eq_ignore_ascii_case(&calendar.name)
        }) {
            return Err(CoreError::InvalidOperation(format!(
                "There is already a calendar named ‘{}’. Calendar names are unique because formulas resolve them by name.",
                calendar.name
            )));
        }
        Ok(())
    }

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
        self.check_calendar(&calendar)?;
        self.calendars.push(calendar);
        Ok(())
    }

    pub(crate) fn apply_update_calendar(&mut self, calendar: Calendar) -> Result<(), CoreError> {
        self.check_calendar(&calendar)?;
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
