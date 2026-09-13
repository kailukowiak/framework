//! Slice 2 of the period-aware time spine: fiscal-calendar date functions
//! that read the date, never the frame — so they need no period
//! declaration, work in Scratchwork, and cannot depend on row order.
use crate::common::frame_named;
use framework_core::*;

fn add_column(store: &mut Store, name: &str, formula: &str) {
    let id = frame_named(store.document(), "Dates").id.clone();
    store
        .apply(Operation::AddComputedColumn {
            frame_id: id,
            name: name.into(),
            formula: formula.into(),
            after_column_id: None,
        })
        .unwrap();
}

/// Seven dates chosen to sit on every interesting edge: both sides of a
/// February year start, a quarter boundary, year end, leap day, and a
/// month-end clamp. The Shift column drives the per-row count test.
fn dated_store() -> Store {
    let mut store = Store::new(Document::blank("Forecast"));
    store
        .apply(Operation::AddFrame {
            name: "Dates".into(),
            grid: vec![
                vec!["Day".into(), "Shift".into()],
                vec!["2025-01-15".into(), "1".into()],
                vec!["2025-02-01".into(), "-1".into()],
                vec!["2025-04-30".into(), "12".into()],
                vec!["2025-05-01".into(), "0".into()],
                vec!["2024-12-31".into(), "2".into()],
                vec!["2024-02-29".into(), "-12".into()],
                vec!["2025-01-31".into(), "1".into()],
            ],
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
        .get_frame_page(&id, 0, 100)
        .unwrap()
        .rows
        .iter()
        .map(|row| row[index].clone())
        .collect()
}

#[test]
fn february_start_puts_boundary_dates_in_the_right_year_quarter_and_period() {
    let mut store = dated_store();
    add_column(&mut store, "FY", "fiscal_year(`Day`, fy_start=2)");
    // January sits before the February start, so it closes the fiscal
    // year that began the previous February; years are numbered by the
    // calendar year they end in, the way company accounts name them.
    assert_eq!(
        column_values(&store, "FY"),
        vec!["2025", "2026", "2026", "2026", "2025", "2025", "2025"]
    );

    add_column(&mut store, "FQ", "fiscal_quarter(`Day`, fy_start=2)");
    // Quarters run Feb–Apr, May–Jul, Aug–Oct, Nov–Jan.
    assert_eq!(
        column_values(&store, "FQ"),
        vec!["4", "1", "1", "2", "4", "1", "4"]
    );

    add_column(&mut store, "FP", "fiscal_period(`Day`, fy_start=2)");
    assert_eq!(
        column_values(&store, "FP"),
        vec!["12", "1", "3", "4", "11", "1", "12"]
    );

    // The default calendar is the calendar year: January is month one.
    add_column(
        &mut store,
        "Cal",
        "fiscal_year(`Day`).cast(\"string\") + \"-Q\" + fiscal_quarter(`Day`).cast(\"string\")",
    );
    assert_eq!(column_values(&store, "Cal")[0], "2025-Q1");
}

#[test]
fn period_start_and_end_bound_the_month_including_leap_february() {
    let mut store = dated_store();
    add_column(&mut store, "Start", "period_start(`Day`)");
    assert_eq!(
        column_values(&store, "Start"),
        vec![
            "2025-01-01",
            "2025-02-01",
            "2025-04-01",
            "2025-05-01",
            "2024-12-01",
            "2024-02-01",
            "2025-01-01",
        ]
    );

    add_column(&mut store, "End", "period_end(`Day`)");
    assert_eq!(
        column_values(&store, "End"),
        vec![
            "2025-01-31",
            "2025-02-28",
            "2025-04-30",
            "2025-05-31",
            "2024-12-31",
            "2024-02-29",
            "2025-01-31",
        ]
    );
}

#[test]
fn add_periods_shifts_both_directions_with_month_end_clamping() {
    let mut store = dated_store();
    add_column(&mut store, "Next", "add_periods(`Day`, 1)");
    // January 31 plus one month is February 28, not March 3: the day
    // clamps to the shorter month, matching EDATE.
    assert_eq!(
        column_values(&store, "Next"),
        vec![
            "2025-02-15",
            "2025-03-01",
            "2025-05-30",
            "2025-06-01",
            "2025-01-31",
            "2024-03-29",
            "2025-02-28",
        ]
    );

    add_column(&mut store, "Prev", "add_periods(`Day`, -1)");
    assert_eq!(
        column_values(&store, "Prev"),
        vec![
            "2024-12-15",
            "2025-01-01",
            "2025-03-30",
            "2025-04-01",
            "2024-11-30",
            "2024-01-29",
            "2024-12-31",
        ]
    );
}

#[test]
fn add_periods_accepts_a_per_row_count_and_blanks_on_missing() {
    let mut store = dated_store();
    add_column(&mut store, "Shifted", "add_periods(`Day`, `Shift`)");
    assert_eq!(
        column_values(&store, "Shifted"),
        vec![
            "2025-02-15",
            "2025-01-01",
            "2026-04-30",
            "2025-05-01",
            "2025-02-28",
            "2023-02-28",
            "2025-02-28",
        ]
    );

    // A missing count makes a missing date rather than a corrupt offset.
    add_column(
        &mut store,
        "Unknown",
        "add_periods(`Day`, when(`Shift` > 100).then(1).otherwise(None))",
    );
    assert!(
        column_values(&store, "Unknown")
            .iter()
            .all(|value| value.is_empty()),
        "{:?}",
        column_values(&store, "Unknown")
    );
}

#[test]
fn receiver_and_namespace_spellings_agree_with_root_calls() {
    let mut store = dated_store();
    add_column(&mut store, "NS", "finance.fiscal_year(`Day`, fy_start=2)");
    add_column(&mut store, "RX", "`Day`.finance.fiscal_year(fy_start=2)");
    assert_eq!(column_values(&store, "NS"), column_values(&store, "RX"));

    add_column(&mut store, "PE", "finance.period_end(`Day`)");
    add_column(&mut store, "PEX", "`Day`.finance.period_end()");
    assert_eq!(column_values(&store, "PE"), column_values(&store, "PEX"));

    add_column(&mut store, "AP", "`Day`.finance.add_periods(1)");
    assert_eq!(
        column_values(&store, "AP"),
        vec![
            "2025-02-15",
            "2025-03-01",
            "2025-05-30",
            "2025-06-01",
            "2025-01-31",
            "2024-03-29",
            "2025-02-28",
        ]
    );
}

#[test]
fn out_of_range_year_starts_and_bad_arity_name_the_problem() {
    let mut store = dated_store();
    for formula in [
        "fiscal_year(`Day`, fy_start=13)",
        "fiscal_quarter(`Day`, 0)",
        "fiscal_period(`Day`, fy_start=2, fy_start=3)",
    ] {
        let id = frame_named(store.document(), "Dates").id.clone();
        let result = store.apply(Operation::AddComputedColumn {
            frame_id: id,
            name: "Bad".into(),
            formula: formula.into(),
            after_column_id: None,
        });
        assert!(
            matches!(&result, Err(CoreError::Formula(message)) if message.contains("fy_start")),
            "{formula}: {result:?}",
        );
    }
    let id = frame_named(store.document(), "Dates").id.clone();
    let missing = store.apply(Operation::AddComputedColumn {
        frame_id: id,
        name: "Bad".into(),
        formula: "add_periods(`Day`)".into(),
        after_column_id: None,
    });
    assert!(
        matches!(&missing, Err(CoreError::Formula(message)) if message.contains("month count")),
        "{missing:?}",
    );
}

#[test]
fn scratchwork_reads_fiscal_dates_with_no_declaration() {
    let mut store = Store::new(Document::blank("Checks"));
    store
        .apply(Operation::AddBlock {
            name: "Checks".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let block = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Checks")
        .unwrap()
        .id()
        .to_string();
    // Unlike `prior`, these read the date itself, so no frame and no
    // declared period column are involved at all.
    store
        .apply(Operation::SetBlockSource {
            block_id: block,
            source: "fy = fiscal_year(\"2025-01-15\".str.to_date(), fy_start=2)\nend = period_end(\"2024-02-15\".str.to_date())\nq = \"2025-05-01\".str.to_date().finance.fiscal_quarter(fy_start=2)".into(),
            editing: None,
        })
        .unwrap();
    let displays: Vec<String> = store
        .view()
        .computed_blocks
        .values()
        .next()
        .unwrap()
        .lines
        .iter()
        .map(|line| {
            assert!(line.cell.error.is_none(), "{:?}", line.cell.error);
            line.cell.display.clone()
        })
        .collect();
    assert_eq!(displays, vec!["2025", "2024-02-29", "2"]);
}

/// The published NRF 4-5-4 retail calendar: February start, years labelled
/// by the year they open, ending the Saturday nearest January 31.
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

#[test]
fn a_retail_calendar_refuses_fy_start_rather_than_dropping_it() {
    let mut store = dated_store();
    nrf_calendar(&mut store);
    // A retail year opens the day after the previous year's Saturday, so
    // there is no month number to move it to. Writing one used to parse
    // and then vanish, answering about the calendar's own year start
    // instead; every fiscal call now says why it cannot.
    for formula in [
        "fiscal_year(`Day`, fy_start=1, calendar=\"NRF\")",
        "fiscal_quarter(`Day`, 1, calendar=\"NRF\")",
        "fiscal_period(`Day`, fy_start=1, calendar=\"NRF\")",
    ] {
        let frame_id = frame_named(store.document(), "Dates").id.clone();
        let refused = store.apply(Operation::AddComputedColumn {
            frame_id,
            name: "Refused".into(),
            formula: formula.into(),
            after_column_id: None,
        });
        let message = refused.unwrap_err().to_string();
        assert!(
            message.contains("cannot take fy_start")
                && message.contains("retail week calendar")
                && message.contains("calendar-month calendar"),
            "unexpected error for {formula}: {message}"
        );
    }

    // Without fy_start the same calls read the week table, and 2025-01-15
    // lands in the retail year that opened in February 2024.
    add_column(&mut store, "FY", "fiscal_year(`Day`, calendar=\"NRF\")");
    assert_eq!(column_values(&store, "FY")[0], "2024");
}

#[test]
fn period_relative_calls_refuse_a_retail_fy_start_in_the_same_words() {
    let mut store = dated_store();
    nrf_calendar(&mut store);
    // `prior` and the windows check the declaration before the call, so
    // declare one: the refusal under test is about the calendar, not about
    // a missing period column.
    let frame = frame_named(store.document(), "Dates").clone();
    let day = frame
        .columns
        .iter()
        .find(|column| column.name == "Day")
        .unwrap()
        .id
        .clone();
    store
        .apply(Operation::SetFramePeriod {
            frame_id: frame.id.clone(),
            period: Some(FramePeriod {
                column_id: day,
                partition_column_ids: Vec::new(),
            }),
        })
        .unwrap();
    // The index and the window aggregates share the refusal, so a workbook
    // cannot end up with a retail `fiscal_period` beside a January-counted
    // `period_index` on the same dates.
    for formula in [
        "period_index(`Day`, fy_start=1, calendar=\"NRF\")",
        "prior(`Shift`, 1, fy_start=1, calendar=\"NRF\")",
        "ytd(`Shift`, fy_start=1, calendar=\"NRF\")",
    ] {
        let frame_id = frame_named(store.document(), "Dates").id.clone();
        let refused = store.apply(Operation::AddComputedColumn {
            frame_id,
            name: "Refused".into(),
            formula: formula.into(),
            after_column_id: None,
        });
        let message = refused.unwrap_err().to_string();
        assert!(
            message.contains("cannot take fy_start") && message.contains("retail week calendar"),
            "unexpected error for {formula}: {message}"
        );
    }
}
