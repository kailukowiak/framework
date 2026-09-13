//! What a formula's calendar reference is worth: renaming, removing, the
//! replicated back door, and undo.
//!
//! Calendars are named in formulas by name, which is the whole reason
//! this file exists. Every other reference in the document — a value, a
//! foreign column — is resolved to an id when the formula is parsed, so
//! renaming the thing is free. A calendar reference is a plain string
//! argument that may equally be supplied by a named value at plan time,
//! so it cannot be resolved that early, and renaming a calendar is
//! therefore a rewrite of every formula that says the old name. Nothing
//! rewrites them, so both rename and removal are refused while a formula
//! names the calendar — the tests below are the statement of that rule.
use crate::common::frame_named;
use framework_core::*;

fn store_with_calendar(name: &str) -> Store {
    let mut store = Store::new(Document::blank("Calendars"));
    store
        .apply(Operation::AddFrame {
            name: "Dates".into(),
            grid: vec![
                vec!["Day".into()],
                vec!["2026-01-15".into()],
                vec!["2026-02-01".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::AddCalendar {
            name: name.into(),
            fy_start: 2,
            pattern: WeekPattern::Months,
            year_end: YearEndRule::LastDayOfMonth,
            year_label: YearLabel::End,
            weekend: vec![6, 7],
            holidays: Vec::new(),
        })
        .unwrap();
    store
}

fn add_column(store: &mut Store, name: &str, formula: &str) {
    let frame_id = frame_named(store.document(), "Dates").id.clone();
    store
        .apply(Operation::AddComputedColumn {
            frame_id,
            name: name.into(),
            formula: formula.into(),
            after_column_id: None,
        })
        .unwrap();
}

fn column_values(store: &Store, name: &str) -> Vec<String> {
    let frame = frame_named(store.document(), "Dates");
    let id = frame.id.clone();
    let index = frame
        .columns
        .iter()
        .position(|column| column.name == name)
        .unwrap();
    store
        .get_frame_page(&id, 0, 100)
        .unwrap()
        .rows
        .iter()
        .map(|row| row[index].clone())
        .collect()
}

fn rename(store: &mut Store, to: &str) -> Result<DocumentView, CoreError> {
    let calendar = store.document().calendars[0].clone();
    store.apply(Operation::UpdateCalendar {
        calendar_id: calendar.id,
        name: to.into(),
        fy_start: calendar.fy_start,
        pattern: calendar.pattern,
        year_end: calendar.year_end,
        year_label: calendar.year_label,
        weekend: calendar.weekend,
        holidays: calendar.holidays,
    })
}

#[test]
fn renaming_a_referenced_calendar_is_refused_naming_the_formula_that_reads_it() {
    let mut store = store_with_calendar("Company");
    add_column(&mut store, "FY", "fiscal_year(`Day`, calendar=\"Company\")");
    assert_eq!(column_values(&store, "FY"), vec!["2026", "2027"]);

    let refused = rename(&mut store, "Group");
    assert!(
        matches!(&refused, Err(CoreError::InvalidOperation(message))
            if message.contains("‘Company’") && message.contains("‘Dates’")),
        "{refused:?}",
    );
    // Refused, not half-applied: the name and the answers both stand.
    assert_eq!(store.document().calendars[0].name, "Company");
    assert_eq!(column_values(&store, "FY"), vec!["2026", "2027"]);

    // The reference is matched the way a formula resolves it, without
    // regard to case, and a positional calendar argument counts too.
    add_column(&mut store, "Week", "fiscal_week(`Day`, \"company\")");
    assert!(rename(&mut store, "Group").is_err());

    // Everything except the name may still be edited freely: the formula
    // goes on reading the calendar and simply gets the new answer.
    let calendar = store.document().calendars[0].clone();
    store
        .apply(Operation::UpdateCalendar {
            calendar_id: calendar.id,
            name: "Company".into(),
            fy_start: 1,
            pattern: calendar.pattern,
            year_end: calendar.year_end,
            year_label: calendar.year_label,
            weekend: calendar.weekend,
            holidays: calendar.holidays,
        })
        .unwrap();
    assert_eq!(column_values(&store, "FY"), vec!["2026", "2026"]);
}

#[test]
fn an_unreferenced_calendar_renames_and_removes_freely() {
    let mut store = store_with_calendar("Company");
    add_column(&mut store, "FY", "fiscal_year(`Day`, fy_start=2)");

    rename(&mut store, "Group").unwrap();
    assert_eq!(store.document().calendars[0].name, "Group");

    let id = store.document().calendars[0].id.clone();
    store
        .apply(Operation::RemoveCalendar { calendar_id: id })
        .unwrap();
    assert!(store.document().calendars.is_empty());
    // The formula never named it, so the answers are untouched.
    assert_eq!(column_values(&store, "FY"), vec!["2026", "2027"]);
}

#[test]
fn removing_a_referenced_calendar_is_refused_naming_the_formula_that_reads_it() {
    let mut store = store_with_calendar("Company");
    add_column(&mut store, "FY", "fiscal_year(`Day`, calendar=\"Company\")");
    let id = store.document().calendars[0].id.clone();

    let refused = store.apply(Operation::RemoveCalendar {
        calendar_id: id.clone(),
    });
    assert!(
        matches!(&refused, Err(CoreError::InvalidOperation(message))
            if message.contains("‘Company’") && message.contains("‘Dates’")),
        "{refused:?}",
    );
    assert_eq!(store.document().calendars.len(), 1);
    assert_eq!(column_values(&store, "FY"), vec!["2026", "2027"]);
}

/// A calendar that will not resolve must never install quietly. It used
/// to be able to: `prepare` validated, `apply` did not, so a replicated
/// operation or a hand-edited `.fw` could seat a calendar with an
/// impossible holiday — after which a bare `fiscal_year(...)` silently
/// fell back to January months while `calendar="…"` errored.
#[test]
fn an_invalid_calendar_cannot_arrive_through_the_replicated_path() {
    let mut store = store_with_calendar("Company");
    let refused = store.apply_replicated(ReplicatedOperation::AddCalendar {
        calendar: Calendar {
            id: "cal-bad".into(),
            name: "Broken".into(),
            fy_start: 2,
            pattern: WeekPattern::Months,
            year_end: YearEndRule::LastDayOfMonth,
            year_label: YearLabel::End,
            weekend: vec![6, 7],
            holidays: vec!["2026-02-30".into()],
        },
    });
    assert!(refused.is_err(), "{refused:?}");
    assert_eq!(store.document().calendars.len(), 1);

    // Nor under a name another calendar already holds, case aside.
    let duplicate = store.apply_replicated(ReplicatedOperation::AddCalendar {
        calendar: Calendar {
            id: "cal-dup".into(),
            name: "company".into(),
            fy_start: 2,
            pattern: WeekPattern::Months,
            year_end: YearEndRule::LastDayOfMonth,
            year_label: YearLabel::End,
            weekend: vec![6, 7],
            holidays: Vec::new(),
        },
    });
    assert!(duplicate.is_err(), "{duplicate:?}");
    assert_eq!(store.document().calendars.len(), 1);
}

/// The other half of the same bug: a document that somehow holds a broken
/// default must say so rather than answer with January arithmetic. The
/// document is built valid and then broken behind the store's back, which
/// is exactly the shape a hand-edited `.fw` arrives in.
#[test]
fn a_broken_default_calendar_errors_instead_of_answering_for_january() {
    let mut store = store_with_calendar("Company");
    let id = store.document().calendars[0].id.clone();
    store
        .apply(Operation::SetDefaultCalendar {
            calendar_id: Some(id),
        })
        .unwrap();
    add_column(&mut store, "FY", "fiscal_year(`Day`)");
    assert_eq!(column_values(&store, "FY"), vec!["2026", "2027"]);

    let mut broken = store.document().clone();
    broken.calendars[0].holidays = vec!["2026-02-30".into()];
    broken.calendars[0].name = "Broken".into();
    let store = Store::new(broken);
    let report = format!("{:?}", store.view());
    assert!(
        report.contains("Broken") && report.contains("cannot be read"),
        "a bare fiscal call on a broken default must name the calendar, got {report}",
    );
}

/// Undoing the removal of the default calendar has to put the default
/// back too. Restoring only the calendar leaves the document with it
/// present and nothing defaulting to it, which sends every bare fiscal
/// call quietly back to January.
#[test]
fn undoing_the_removal_of_the_default_calendar_restores_the_default() {
    let mut store = store_with_calendar("Company");
    let id = store.document().calendars[0].id.clone();
    store
        .apply(Operation::SetDefaultCalendar {
            calendar_id: Some(id.clone()),
        })
        .unwrap();
    add_column(&mut store, "FY", "fiscal_year(`Day`)");
    assert_eq!(column_values(&store, "FY"), vec!["2026", "2027"]);

    // Removal goes through the replicated path, which is the one that
    // clears the default: `prepare` refuses to remove a default outright.
    store
        .apply_replicated(ReplicatedOperation::RemoveCalendar {
            calendar_id: id.clone(),
        })
        .unwrap();
    assert!(store.document().default_calendar_id.is_none());
    assert_eq!(column_values(&store, "FY"), vec!["2026", "2026"]);

    store.undo();
    assert_eq!(store.document().calendars.len(), 1);
    assert_eq!(store.document().default_calendar_id.as_deref(), Some(&*id));
    assert_eq!(column_values(&store, "FY"), vec!["2026", "2027"]);
}
