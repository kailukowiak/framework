//! Slice 4 of the time spine: document calendars, retail weeks, and
//! business days.
//!
//! The NRF 4-5-4 anchors below are published facts, not implementation
//! output: fiscal 2023 runs 53 weeks ending February 3 2024, fiscal 2024
//! runs 52 weeks ending February 1 2025, and the 2026 period table starts
//! February 1 2026 with January P12 ending January 30 2027. The Saturday
//! nearest January 31 ends each year; the tests assert the implementation
//! reproduces every week between those anchors.
use crate::common::frame_named;
use framework_core::*;

fn add_column(store: &mut Store, frame: &str, name: &str, formula: &str) {
    let id = frame_named(store.document(), frame).id.clone();
    store
        .apply(Operation::AddComputedColumn {
            frame_id: id,
            name: name.into(),
            formula: formula.into(),
            after_column_id: None,
        })
        .unwrap();
}

fn nrf_calendar(store: &mut Store) {
    store
        .apply(Operation::AddCalendar {
            name: "NRF".into(),
            fy_start: 2,
            pattern: WeekPattern::FourFiveFour,
            year_end: YearEndRule::NearestWeekday {
                weekday: 6,
                month: 1,
                day: 31,
            },
            year_label: YearLabel::Start,
            weekend: vec![6, 7],
            holidays: Vec::new(),
        })
        .unwrap();
}

fn dates_store(days: &[&str]) -> Store {
    let mut store = Store::new(Document::blank("Retail"));
    let mut grid = vec![vec!["Day".into()]];
    grid.extend(days.iter().map(|day| vec![(*day).into()]));
    store
        .apply(Operation::AddFrame {
            name: "Dates".into(),
            grid,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
}

fn column_values(store: &Store, name: &str) -> Vec<String> {
    let id = frame_named(store.document(), "Dates").id.clone();
    let frame = frame_named(store.document(), "Dates");
    let index = frame
        .columns
        .iter()
        .position(|column| column.name == name)
        .unwrap();
    store
        .get_frame_page(&id, 0, 2000)
        .unwrap()
        .rows
        .iter()
        .chain(store.get_frame_page(&id, 1000, 2000).unwrap().rows.iter())
        .map(|row| row[index].clone())
        .collect()
}

#[test]
fn calendars_round_trip_through_history_with_validation() {
    let mut store = Store::new(Document::blank("Calendars"));
    nrf_calendar(&mut store);
    assert_eq!(store.document().calendars.len(), 1);
    assert_eq!(store.document().calendars[0].name, "NRF");
    store.undo();
    assert!(store.document().calendars.is_empty());
    store.redo();
    assert_eq!(store.document().calendars.len(), 1);

    // Names stay unique because formulas resolve them by name.
    let duplicate = store.apply(Operation::AddCalendar {
        name: "nrf".into(),
        fy_start: 2,
        pattern: WeekPattern::Months,
        year_end: YearEndRule::LastDayOfMonth,
        year_label: YearLabel::End,
        weekend: vec![6, 7],
        holidays: Vec::new(),
    });
    assert!(
        matches!(&duplicate, Err(CoreError::InvalidOperation(message)) if message.contains("already a calendar named")),
        "{duplicate:?}",
    );
    // Holidays are strict dates, and months run 1 to 12.
    let bad_holiday = store.apply(Operation::AddCalendar {
        name: "Bad".into(),
        fy_start: 13,
        pattern: WeekPattern::Months,
        year_end: YearEndRule::LastDayOfMonth,
        year_label: YearLabel::End,
        weekend: vec![6, 7],
        holidays: vec!["2026-02-30".into()],
    });
    assert!(
        matches!(&bad_holiday, Err(CoreError::InvalidOperation(_))),
        "{bad_holiday:?}",
    );

    // Rename by update; the id is stable across it.
    let id = store.document().calendars[0].id.clone();
    store
        .apply(Operation::UpdateCalendar {
            calendar_id: id.clone(),
            name: "NRF 4-5-4".into(),
            fy_start: 2,
            pattern: WeekPattern::FourFiveFour,
            year_end: YearEndRule::NearestWeekday {
                weekday: 6,
                month: 1,
                day: 31,
            },
            year_label: YearLabel::Start,
            weekend: vec![6, 7],
            holidays: vec!["2026-01-01".into()],
        })
        .unwrap();
    assert_eq!(store.document().calendars[0].name, "NRF 4-5-4");
    assert_eq!(store.document().calendars[0].id, id);
    assert_eq!(
        store.document().calendars[0].holidays,
        vec!["2026-01-01".to_string()]
    );
    store.undo();
    assert_eq!(store.document().calendars[0].name, "NRF");

    // The default cannot be removed while it is the default.
    store
        .apply(Operation::SetDefaultCalendar {
            calendar_id: Some(id.clone()),
        })
        .unwrap();
    assert_eq!(store.document().default_calendar_id, Some(id.clone()));
    let remove_default = store.apply(Operation::RemoveCalendar {
        calendar_id: id.clone(),
    });
    assert!(
        matches!(&remove_default, Err(CoreError::InvalidOperation(message)) if message.contains("default")),
        "{remove_default:?}",
    );
    store
        .apply(Operation::SetDefaultCalendar { calendar_id: None })
        .unwrap();
    store
        .apply(Operation::RemoveCalendar {
            calendar_id: id.clone(),
        })
        .unwrap();
    assert!(store.document().calendars.is_empty());
    store.undo();
    assert_eq!(store.document().calendars.len(), 1);
}

#[test]
fn retail_years_match_the_published_nrf_boundaries() {
    let mut store = dates_store(&[
        "2023-01-28",
        "2023-01-29",
        "2024-02-03",
        "2024-02-04",
        "2025-02-01",
        "2025-02-02",
        "2026-01-31",
        "2026-02-01",
    ]);
    nrf_calendar(&mut store);
    add_column(
        &mut store,
        "Dates",
        "FY",
        "fiscal_year(`Day`, calendar=\"NRF\")",
    );
    add_column(
        &mut store,
        "Dates",
        "W",
        "fiscal_week(`Day`, calendar=\"NRF\")",
    );
    // NRF labels years by their start: the 53-week 2023 runs January 29
    // 2023 to February 3 2024, then 52-week 2024 and 2025 follow.
    assert_eq!(
        column_values(&store, "FY"),
        vec![
            "2022", "2023", "2023", "2024", "2024", "2025", "2025", "2026"
        ]
    );
    assert_eq!(
        column_values(&store, "W"),
        vec!["52", "1", "53", "1", "52", "1", "52", "1"]
    );
}

#[test]
fn every_retail_week_of_three_years_lands_where_published() {
    // Anchors and lengths from the published calendars: 2023 starts
    // January 29 with 53 weeks, 2024 starts February 4 with 52, 2025
    // starts February 2 with 52.
    let anchors = [("2023-01-29", 53), ("2024-02-04", 52), ("2025-02-02", 52)];
    let mut days: Vec<String> = Vec::new();
    for (start, weeks) in anchors {
        let mut date = chrono::NaiveDate::parse_from_str(start, "%Y-%m-%d").unwrap();
        for _ in 0..weeks {
            days.push(date.format("%Y-%m-%d").to_string());
            // Assert every Sunday-to-Saturday week, not just its start.
            for _ in 0..6 {
                date = date.succ_opt().unwrap();
                days.push(date.format("%Y-%m-%d").to_string());
            }
            date = date.succ_opt().unwrap();
        }
    }
    let refs: Vec<&str> = days.iter().map(String::as_str).collect();
    let mut store = dates_store(&refs);
    nrf_calendar(&mut store);
    add_column(
        &mut store,
        "Dates",
        "W",
        "fiscal_week(`Day`, calendar=\"NRF\")",
    );
    let weeks = column_values(&store, "W");
    let mut index = 0;
    for (_, count) in anchors {
        for week in 1..=count {
            for _ in 0..7 {
                assert_eq!(weeks[index], week.to_string(), "day {}", days[index]);
                index += 1;
            }
        }
    }
    assert_eq!(index, weeks.len());
}

#[test]
fn retail_quarters_periods_and_bounds_follow_the_454_blocks() {
    let mut store = dates_store(&[
        "2023-01-29",
        "2023-02-26",
        "2023-04-02",
        "2023-12-31",
        "2024-02-03",
        "2024-02-04",
        "2025-02-02",
        "2026-01-31",
    ]);
    nrf_calendar(&mut store);
    add_column(
        &mut store,
        "Dates",
        "Q",
        "fiscal_quarter(`Day`, calendar=\"NRF\")",
    );
    add_column(
        &mut store,
        "Dates",
        "P",
        "fiscal_period(`Day`, calendar=\"NRF\")",
    );
    add_column(
        &mut store,
        "Dates",
        "PS",
        "period_start(`Day`, calendar=\"NRF\")",
    );
    add_column(
        &mut store,
        "Dates",
        "PE",
        "period_end(`Day`, calendar=\"NRF\")",
    );
    // 4-5-4 blocks: February weeks 1–4, March 5–9, April 10–13; the 53rd
    // week stretches January 2024 to five weeks, December 30 to February 3.
    assert_eq!(
        column_values(&store, "Q"),
        vec!["1", "1", "1", "4", "4", "1", "1", "4"]
    );
    assert_eq!(
        column_values(&store, "P"),
        vec!["1", "2", "3", "12", "12", "1", "1", "12"]
    );
    assert_eq!(
        column_values(&store, "PS"),
        vec![
            "2023-01-29",
            "2023-02-26",
            "2023-04-02",
            "2023-12-31",
            "2023-12-31",
            "2024-02-04",
            "2025-02-02",
            "2026-01-04",
        ]
    );
    assert_eq!(
        column_values(&store, "PE"),
        vec![
            "2023-02-25",
            "2023-04-01",
            "2023-04-29",
            "2024-02-03",
            "2024-02-03",
            "2024-03-02",
            "2025-03-01",
            "2026-01-31",
        ]
    );
}

#[test]
fn bare_fiscal_calls_read_the_default_calendar() {
    let mut store = dates_store(&["2026-01-15", "2026-02-01"]);
    // The tutorial's company: February start, calendar months, years
    // numbered by the calendar year they end in.
    store
        .apply(Operation::AddCalendar {
            name: "Company".into(),
            fy_start: 2,
            pattern: WeekPattern::Months,
            year_end: YearEndRule::LastDayOfMonth,
            year_label: YearLabel::End,
            weekend: vec![6, 7],
            holidays: Vec::new(),
        })
        .unwrap();
    let id = store.document().calendars[0].id.clone();
    store
        .apply(Operation::SetDefaultCalendar {
            calendar_id: Some(id),
        })
        .unwrap();
    // No year start written anywhere: January 2026 closes FY2026 and
    // February opens FY2027, exactly the lesson's checkpoints.
    add_column(&mut store, "Dates", "FY", "fiscal_year(`Day`)");
    add_column(&mut store, "Dates", "FQ", "fiscal_quarter(`Day`)");
    assert_eq!(column_values(&store, "FY"), vec!["2026", "2027"]);
    assert_eq!(column_values(&store, "FQ"), vec!["4", "1"]);
    // An explicit year start still wins over the calendar's.
    add_column(&mut store, "Dates", "Cal", "fiscal_year(`Day`, fy_start=1)");
    assert_eq!(column_values(&store, "Cal"), vec!["2026", "2026"]);
}

#[test]
fn workdays_skip_weekends_holidays_and_shift_both_ways() {
    let mut store = dates_store(&["2026-01-02", "2026-01-03", "2026-02-27"]);
    store
        .apply(Operation::AddCalendar {
            name: "Work".into(),
            fy_start: 1,
            pattern: WeekPattern::Months,
            year_end: YearEndRule::LastDayOfMonth,
            year_label: YearLabel::End,
            weekend: vec![6, 7],
            holidays: vec!["2026-01-01".into()],
        })
        .unwrap();
    // Friday January 2 plus one business day is Monday January 5: the
    // weekend falls away, and New Year's Day would too.
    add_column(
        &mut store,
        "Dates",
        "Next",
        "workday(`Day`, 1, calendar=\"Work\")",
    );
    assert_eq!(
        column_values(&store, "Next"),
        vec!["2026-01-05", "2026-01-05", "2026-03-02"]
    );
    // Back over the same days, and zero returns the date itself.
    add_column(
        &mut store,
        "Dates",
        "Prev",
        "workday(`Day`, -1, calendar=\"Work\")",
    );
    assert_eq!(
        column_values(&store, "Prev"),
        vec!["2025-12-31", "2026-01-02", "2026-02-26"]
    );
    add_column(
        &mut store,
        "Dates",
        "Same",
        "workday(`Day`, 0, calendar=\"Work\")",
    );
    assert_eq!(
        column_values(&store, "Same"),
        vec!["2026-01-02", "2026-01-03", "2026-02-27"]
    );
}

#[test]
fn networkdays_counts_inclusive_both_directions() {
    let mut store = Store::new(Document::blank("Spans"));
    store
        .apply(Operation::AddFrame {
            name: "Dates".into(),
            grid: vec![
                vec!["Start".into(), "End".into()],
                vec!["2026-01-01".into(), "2026-01-07".into()],
                vec!["2026-01-07".into(), "2026-01-01".into()],
                vec!["2026-01-03".into(), "2026-01-04".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::AddCalendar {
            name: "Work".into(),
            fy_start: 1,
            pattern: WeekPattern::Months,
            year_end: YearEndRule::LastDayOfMonth,
            year_label: YearLabel::End,
            weekend: vec![6, 7],
            holidays: vec!["2026-01-01".into()],
        })
        .unwrap();
    let id = frame_named(store.document(), "Dates").id.clone();
    store
        .apply(Operation::AddComputedColumn {
            frame_id: id.clone(),
            name: "Days".into(),
            formula: "networkdays(`Start`, `End`, calendar=\"Work\")".into(),
            after_column_id: None,
        })
        .unwrap();
    // Thursday to Wednesday counts four: the holiday Thursday and the
    // weekend never count. Reversed negates; a bare weekend is zero.
    let frame = frame_named(store.document(), "Dates");
    let index = frame
        .columns
        .iter()
        .position(|column| column.name == "Days")
        .unwrap();
    let rows = store.get_frame_page(&id, 0, 10).unwrap().rows;
    assert_eq!(rows[0][index], "4");
    assert_eq!(rows[1][index], "-4");
    assert_eq!(rows[2][index], "0");
}

#[test]
fn unknown_calendars_name_what_is_available() {
    let mut store = dates_store(&["2026-01-15"]);
    nrf_calendar(&mut store);
    let id = frame_named(store.document(), "Dates").id.clone();
    let missing = store.apply(Operation::AddComputedColumn {
        frame_id: id,
        name: "Bad".into(),
        formula: "fiscal_week(`Day`, calendar=\"Gregorian\")".into(),
        after_column_id: None,
    });
    assert!(
        matches!(&missing, Err(CoreError::Formula(message)) if message.contains("Gregorian") && message.contains("NRF")),
        "{missing:?}",
    );
}

#[test]
fn fiscal_week_counts_from_the_year_start_under_calendar_months() {
    // No calendar named and none defaulted: the built-in calendar-months
    // one answers. It has no week blocks to read a period off, which is
    // the case that used to underflow and take the store with it.
    let mut store = dates_store(&["2026-01-01", "2026-01-07", "2026-01-08", "2026-12-31"]);
    add_column(&mut store, "Dates", "W", "fiscal_week(`Day`)");
    assert_eq!(column_values(&store, "W"), vec!["1", "1", "2", "53"]);

    // A named months calendar starting in February counts from its own
    // start, not January's.
    store
        .apply(Operation::AddCalendar {
            name: "Company".into(),
            fy_start: 2,
            pattern: WeekPattern::Months,
            year_end: YearEndRule::LastDayOfMonth,
            year_label: YearLabel::End,
            weekend: vec![6, 7],
            holidays: Vec::new(),
        })
        .unwrap();
    add_column(
        &mut store,
        "Dates",
        "CW",
        "fiscal_week(`Day`, calendar=\"Company\")",
    );
    // February 1 2025 opens FY2026, so January 1 2026 is its 48th week
    // and December 31 2026 is the 48th week of FY2027.
    assert_eq!(column_values(&store, "CW"), vec!["48", "49", "49", "48"]);
}

#[test]
fn a_week_pattern_refuses_a_year_that_ends_mid_week() {
    let mut store = Store::new(Document::blank("Calendars"));
    // 4-4-5 weeks only tile a year that is a whole number of weeks; the
    // last day of a month lands wherever it lands, which used to leave a
    // 53rd week in a 52-week year and panic evaluation.
    let refused = store.apply(Operation::AddCalendar {
        name: "Broken".into(),
        fy_start: 1,
        pattern: WeekPattern::FourFourFive,
        year_end: YearEndRule::LastDayOfMonth,
        year_label: YearLabel::End,
        weekend: vec![6, 7],
        holidays: Vec::new(),
    });
    assert!(
        matches!(&refused, Err(CoreError::InvalidOperation(message)) if message.contains("whole weeks")),
        "{refused:?}",
    );

    // A weekday year end is accepted, and updating a good calendar into
    // the bad pairing is refused the same way.
    store
        .apply(Operation::AddCalendar {
            name: "Retail".into(),
            fy_start: 1,
            pattern: WeekPattern::FourFourFive,
            year_end: YearEndRule::LastWeekday { weekday: 6 },
            year_label: YearLabel::End,
            weekend: vec![6, 7],
            holidays: Vec::new(),
        })
        .unwrap();
    let id = store.document().calendars[0].id.clone();
    let refused = store.apply(Operation::UpdateCalendar {
        calendar_id: id,
        name: "Retail".into(),
        fy_start: 1,
        pattern: WeekPattern::FourFourFive,
        year_end: YearEndRule::LastDayOfMonth,
        year_label: YearLabel::End,
        weekend: vec![6, 7],
        holidays: Vec::new(),
    });
    assert!(
        matches!(&refused, Err(CoreError::InvalidOperation(message)) if message.contains("whole weeks")),
        "{refused:?}",
    );
}

#[test]
fn a_year_end_anchor_cannot_be_a_day_some_years_lack() {
    let mut store = Store::new(Document::blank("Calendars"));
    // February 29 exists in one year out of four, so anchoring a year end
    // to it has no answer in the other three.
    let refused = store.apply(Operation::AddCalendar {
        name: "Leap".into(),
        fy_start: 3,
        pattern: WeekPattern::FourFiveFour,
        year_end: YearEndRule::NearestWeekday {
            weekday: 6,
            month: 2,
            day: 29,
        },
        year_label: YearLabel::End,
        weekend: vec![6, 7],
        holidays: Vec::new(),
    });
    assert!(
        matches!(&refused, Err(CoreError::InvalidOperation(message)) if message.contains("anchor days")),
        "{refused:?}",
    );
    // February 28 is fine, and so is a 31-day month's 31st.
    store
        .apply(Operation::AddCalendar {
            name: "Leap".into(),
            fy_start: 3,
            pattern: WeekPattern::FourFiveFour,
            year_end: YearEndRule::NearestWeekday {
                weekday: 6,
                month: 2,
                day: 28,
            },
            year_label: YearLabel::End,
            weekend: vec![6, 7],
            holidays: Vec::new(),
        })
        .unwrap();
}

#[test]
fn a_calendar_keeps_at_least_one_working_day_and_workday_always_returns() {
    let mut store = dates_store(&["2026-01-02"]);
    // Every day listed as not worked leaves nothing for workday() to land
    // on, which used to spin forever inside the formula.
    let refused = store.apply(Operation::AddCalendar {
        name: "Nothing".into(),
        fy_start: 1,
        pattern: WeekPattern::Months,
        year_end: YearEndRule::LastDayOfMonth,
        year_label: YearLabel::End,
        weekend: vec![1, 2, 3, 4, 5, 6, 7],
        holidays: Vec::new(),
    });
    assert!(
        matches!(&refused, Err(CoreError::InvalidOperation(message)) if message.contains("all seven")),
        "{refused:?}",
    );

    store
        .apply(Operation::AddCalendar {
            name: "Work".into(),
            fy_start: 1,
            pattern: WeekPattern::Months,
            year_end: YearEndRule::LastDayOfMonth,
            year_label: YearLabel::End,
            weekend: vec![6, 7],
            holidays: Vec::new(),
        })
        .unwrap();
    // An absurd shift gives up with no answer instead of hanging.
    add_column(
        &mut store,
        "Dates",
        "Far",
        "workday(`Day`, 1000000000, calendar=\"Work\")",
    );
    assert_eq!(column_values(&store, "Far"), vec![""]);
    // An ordinary shift still answers.
    add_column(
        &mut store,
        "Dates",
        "Near",
        "workday(`Day`, 5, calendar=\"Work\")",
    );
    assert_eq!(column_values(&store, "Near"), vec!["2026-01-09"]);
}
