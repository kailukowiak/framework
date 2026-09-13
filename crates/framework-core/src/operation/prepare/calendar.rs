//! Preparing document-level fiscal calendar edits.
//!
//! Calendars are data about time, not time itself: adding one validates
//! its fields and mints its id, so every replica files the same calendar
//! under the same id without re-running validation.

use crate::model::calendar::{Calendar, WeekPattern, YearEndRule, YearLabel, validate_calendar};
use crate::*;

impl Document {
    fn calendar_name_taken(&self, name: &str, except: Option<&str>) -> bool {
        self.calendars.iter().any(|calendar| {
            Some(calendar.id.as_str()) != except && calendar.name.eq_ignore_ascii_case(name)
        })
    }

    // The parameter list mirrors the operation's own fields; collapsing
    // it into a struct would just rename the variant.
    #[allow(clippy::too_many_arguments)]
    fn validated_calendar(
        &self,
        id: Id,
        name: String,
        fy_start: u8,
        pattern: WeekPattern,
        year_end: YearEndRule,
        year_label: YearLabel,
        weekend: Vec<u8>,
        holidays: Vec<String>,
        except: Option<&str>,
    ) -> Result<Calendar, CoreError> {
        validate_calendar(&name, fy_start, pattern, &year_end, &weekend, &holidays)
            .map_err(CoreError::InvalidOperation)?;
        if self.calendar_name_taken(&name, except) {
            return Err(CoreError::InvalidOperation(format!(
                "There is already a calendar named ‘{name}’. Calendar names are unique because formulas resolve them by name."
            )));
        }
        let mut weekend = weekend;
        weekend.sort_unstable();
        weekend.dedup();
        let mut holidays = holidays;
        holidays.sort();
        holidays.dedup();
        Ok(Calendar {
            id,
            name,
            fy_start,
            pattern,
            year_end,
            year_label,
            weekend,
            holidays,
        })
    }

    /// The name of the first object holding a formula that names the
    /// calendar `name`, if any.
    ///
    /// Formulas name calendars by name — the one document reference that
    /// is still a string rather than an id (see `Expr::names_calendar`).
    /// That makes a rename or a removal a rewrite of every formula that
    /// says the old name, which nothing here does; so instead of silently
    /// breaking them, both refuse and say what is in the way.
    fn first_calendar_reference(&self, name: &str) -> Option<String> {
        self.objects.iter().find_map(|object| {
            let reads = match object {
                DataObject::Frame(frame) => {
                    frame
                        .expressions()
                        .any(|expression| expression.names_calendar(name))
                        || frame
                            .display
                            .style_rules
                            .iter()
                            .any(|rule| rule.formula.expression.names_calendar(name))
                }
                DataObject::Result(result) => result.formula.expression.names_calendar(name),
                DataObject::Block(block) => block.lines.iter().any(|line| {
                    line.formula
                        .as_ref()
                        .is_some_and(|formula| formula.expression.names_calendar(name))
                }),
                _ => false,
            };
            reads.then(|| object.name().to_string())
        })
    }

    // The parameter list mirrors the operation's own fields; collapsing
    // it into a struct would just rename the variant.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_add_calendar(
        &self,
        name: String,
        fy_start: u8,
        pattern: WeekPattern,
        year_end: YearEndRule,
        year_label: YearLabel,
        weekend: Vec<u8>,
        holidays: Vec<String>,
    ) -> Result<ReplicatedOperation, CoreError> {
        let calendar = self.validated_calendar(
            id(),
            name,
            fy_start,
            pattern,
            year_end,
            year_label,
            weekend,
            holidays,
            None,
        )?;
        Ok(ReplicatedOperation::AddCalendar { calendar })
    }

    // The parameter list mirrors the operation's own fields; collapsing
    // it into a struct would just rename the variant.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_update_calendar(
        &self,
        calendar_id: Id,
        name: String,
        fy_start: u8,
        pattern: WeekPattern,
        year_end: YearEndRule,
        year_label: YearLabel,
        weekend: Vec<u8>,
        holidays: Vec<String>,
    ) -> Result<ReplicatedOperation, CoreError> {
        let previous_name = self
            .calendars
            .iter()
            .find(|calendar| calendar.id == calendar_id)
            .map(|calendar| calendar.name.clone())
            .ok_or_else(|| {
                CoreError::InvalidOperation(format!(
                    "There is no calendar with id ‘{calendar_id}’."
                ))
            })?;
        // Everything else about a calendar may be edited freely: a formula
        // that reads it goes on reading it and simply gets the new answer,
        // which is the point of holding the rule in one place. The name is
        // the exception, because the name is the reference.
        if !previous_name.eq_ignore_ascii_case(&name)
            && let Some(reader) = self.first_calendar_reference(&previous_name)
        {
            return Err(CoreError::InvalidOperation(format!(
                "‘{previous_name}’ cannot be renamed while ‘{reader}’ has a formula that names it. Change those formulas first, or edit the calendar's rules without renaming it."
            )));
        }
        let calendar = self.validated_calendar(
            calendar_id.clone(),
            name,
            fy_start,
            pattern,
            year_end,
            year_label,
            weekend,
            holidays,
            Some(&calendar_id),
        )?;
        Ok(ReplicatedOperation::UpdateCalendar { calendar })
    }

    pub(crate) fn prepare_remove_calendar(
        &self,
        calendar_id: Id,
    ) -> Result<ReplicatedOperation, CoreError> {
        let calendar = self
            .calendars
            .iter()
            .find(|calendar| calendar.id == calendar_id)
            .ok_or_else(|| {
                CoreError::InvalidOperation(format!(
                    "There is no calendar with id ‘{calendar_id}’."
                ))
            })?;
        if self.default_calendar_id.as_deref() == Some(calendar_id.as_str()) {
            return Err(CoreError::InvalidOperation(format!(
                "‘{}’ is the default calendar. Set another default before removing it.",
                calendar.name
            )));
        }
        let name = calendar.name.clone();
        if let Some(reader) = self.first_calendar_reference(&name) {
            return Err(CoreError::InvalidOperation(format!(
                "‘{name}’ cannot be removed while ‘{reader}’ has a formula that names it. Change those formulas first."
            )));
        }
        Ok(ReplicatedOperation::RemoveCalendar { calendar_id })
    }

    pub(crate) fn prepare_set_default_calendar(
        &self,
        calendar_id: Option<Id>,
    ) -> Result<ReplicatedOperation, CoreError> {
        if let Some(id) = &calendar_id
            && !self.calendars.iter().any(|calendar| &calendar.id == id)
        {
            return Err(CoreError::InvalidOperation(format!(
                "There is no calendar with id ‘{id}’."
            )));
        }
        Ok(ReplicatedOperation::SetDefaultCalendar { calendar_id })
    }
}
