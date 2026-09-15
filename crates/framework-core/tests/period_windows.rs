//! Slice 3 of the period-aware time spine: `ytd`, `ttm` and
//! `same_period_last_year` as declaration-gated self-joins.
//!
//! The grid below is deliberately out of date order: every answer must
//! attach to its date, never to its row position, which is the §13 test
//! from the build plan wearing a monthly calendar.
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

/// Fourteen months across a February year-start boundary, shuffled.
fn yearly_store() -> Store {
    let mut store = Store::new(Document::blank("Forecast"));
    store
        .apply(Operation::AddFrame {
            name: "Actuals".into(),
            grid: vec![
                vec!["Month".into(), "Revenue".into()],
                vec!["2025-06-01".into(), "150".into()],
                vec!["2025-01-01".into(), "100".into()],
                vec!["2025-12-01".into(), "210".into()],
                vec!["2025-03-01".into(), "120".into()],
                vec!["2026-01-01".into(), "220".into()],
                vec!["2025-02-01".into(), "110".into()],
                vec!["2025-09-01".into(), "180".into()],
                vec!["2025-05-01".into(), "140".into()],
                vec!["2025-11-01".into(), "200".into()],
                vec!["2025-04-01".into(), "130".into()],
                vec!["2026-02-01".into(), "230".into()],
                vec!["2025-08-01".into(), "170".into()],
                vec!["2025-07-01".into(), "160".into()],
                vec!["2025-10-01".into(), "190".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Actuals").clone();
    let month = frame
        .columns
        .iter()
        .find(|column| column.name == "Month")
        .unwrap()
        .id
        .clone();
    store
        .apply(Operation::SetFramePeriod {
            frame_id: frame.id.clone(),
            period: Some(FramePeriod {
                column_id: month,
                partition_column_ids: Vec::new(),
            }),
        })
        .unwrap();
    store
}

fn by_month(store: &Store, name: &str) -> Vec<(String, String)> {
    let id = frame_named(store.document(), "Actuals").id.clone();
    let frame = frame_named(store.document(), "Actuals");
    let index = frame
        .columns
        .iter()
        .position(|column| column.name == name)
        .unwrap();
    let mut pairs: Vec<(String, String)> = store
        .get_frame_page(&id, 0, 100)
        .unwrap()
        .rows
        .into_iter()
        .map(|row| (row[0].clone(), row[index].clone()))
        .collect();
    pairs.sort();
    pairs
}

#[test]
fn ytd_resets_when_the_fiscal_year_turns() {
    let mut store = yearly_store();
    add_column(&mut store, "Actuals", "YTD", "ytd(`Revenue`, fy_start=2)");
    // Fiscal 2025 runs February to January: January 2025 closes the old
    // year with only itself present, February restarts from its own value.
    assert_eq!(
        by_month(&store, "YTD"),
        vec![
            ("2025-01-01".into(), "100".into()),
            ("2025-02-01".into(), "110".into()),
            ("2025-03-01".into(), "230".into()),
            ("2025-04-01".into(), "360".into()),
            ("2025-05-01".into(), "500".into()),
            ("2025-06-01".into(), "650".into()),
            ("2025-07-01".into(), "810".into()),
            ("2025-08-01".into(), "980".into()),
            ("2025-09-01".into(), "1160".into()),
            ("2025-10-01".into(), "1350".into()),
            ("2025-11-01".into(), "1550".into()),
            ("2025-12-01".into(), "1760".into()),
            ("2026-01-01".into(), "1980".into()),
            ("2026-02-01".into(), "230".into()),
        ]
    );

    // The default calendar is the calendar year: January opens a new one.
    add_column(&mut store, "Actuals", "Cal", "ytd(`Revenue`)");
    let january = by_month(&store, "Cal")
        .into_iter()
        .find(|(month, _)| month == "2025-01-01")
        .unwrap();
    assert_eq!(january.1, "100");
    let december = by_month(&store, "Cal")
        .into_iter()
        .find(|(month, _)| month == "2025-12-01")
        .unwrap();
    assert_eq!(december.1, "1860");
}

#[test]
fn ttm_sums_the_twelve_periods_ending_here() {
    let mut store = yearly_store();
    add_column(&mut store, "Actuals", "TTM", "ttm(`Revenue`)");
    assert_eq!(
        by_month(&store, "TTM"),
        vec![
            ("2025-01-01".into(), "100".into()),
            ("2025-02-01".into(), "210".into()),
            ("2025-03-01".into(), "330".into()),
            ("2025-04-01".into(), "460".into()),
            ("2025-05-01".into(), "600".into()),
            ("2025-06-01".into(), "750".into()),
            ("2025-07-01".into(), "910".into()),
            ("2025-08-01".into(), "1080".into()),
            ("2025-09-01".into(), "1260".into()),
            ("2025-10-01".into(), "1450".into()),
            ("2025-11-01".into(), "1650".into()),
            ("2025-12-01".into(), "1860".into()),
            ("2026-01-01".into(), "1980".into()),
            ("2026-02-01".into(), "2100".into()),
        ]
    );
}

#[test]
fn same_period_last_year_reads_twelve_indexes_back() {
    let mut store = yearly_store();
    add_column(
        &mut store,
        "Actuals",
        "LastYear",
        "same_period_last_year(`Revenue`)",
    );
    // No 2024 rows exist, so the whole of 2025 reads blank; January and
    // February 2026 read January and February 2025.
    assert_eq!(
        by_month(&store, "LastYear"),
        vec![
            ("2025-01-01".into(), "".into()),
            ("2025-02-01".into(), "".into()),
            ("2025-03-01".into(), "".into()),
            ("2025-04-01".into(), "".into()),
            ("2025-05-01".into(), "".into()),
            ("2025-06-01".into(), "".into()),
            ("2025-07-01".into(), "".into()),
            ("2025-08-01".into(), "".into()),
            ("2025-09-01".into(), "".into()),
            ("2025-10-01".into(), "".into()),
            ("2025-11-01".into(), "".into()),
            ("2025-12-01".into(), "".into()),
            ("2026-01-01".into(), "100".into()),
            ("2026-02-01".into(), "110".into()),
        ]
    );
}

#[test]
fn windows_sum_the_periods_present_after_a_deletion() {
    let mut store = yearly_store();
    let frame_id = frame_named(store.document(), "Actuals").id.clone();
    let june = frame_named(store.document(), "Actuals").rows[0].id.clone();
    store
        .apply(Operation::DeleteRow {
            frame_id: frame_id.clone(),
            row_id: june,
        })
        .unwrap();
    // Deleting June removes 150 from every window holding it; no window
    // fails or borrows a neighbour.
    add_column(&mut store, "Actuals", "YTD", "ytd(`Revenue`, fy_start=2)");
    add_column(&mut store, "Actuals", "TTM", "ttm(`Revenue`)");
    let july: Vec<(String, String)> = by_month(&store, "YTD")
        .into_iter()
        .filter(|(month, _)| month == "2025-07-01")
        .collect();
    assert_eq!(july, vec![("2025-07-01".into(), "660".into())]);
    let july_ttm: Vec<(String, String)> = by_month(&store, "TTM")
        .into_iter()
        .filter(|(month, _)| month == "2025-07-01")
        .collect();
    assert_eq!(july_ttm, vec![("2025-07-01".into(), "760".into())]);
}

#[test]
fn windows_skip_nulls_and_blank_an_empty_window() {
    let mut store = yearly_store();
    // June's revenue goes missing without deleting its row.
    add_column(
        &mut store,
        "Actuals",
        "Sparse",
        "when(`Month` == date(2025, 6, 1)).then(None).otherwise(`Revenue`)",
    );
    add_column(
        &mut store,
        "Actuals",
        "SparseYTD",
        "ytd(`Sparse`, fy_start=2)",
    );
    let july: Vec<(String, String)> = by_month(&store, "SparseYTD")
        .into_iter()
        .filter(|(month, _)| month == "2025-07-01")
        .collect();
    assert_eq!(july, vec![("2025-07-01".into(), "660".into())]);

    // A window with no readable value at all reads blank, not zero.
    add_column(
        &mut store,
        "Actuals",
        "Nothing",
        "when(false).then(`Revenue`).otherwise(None)",
    );
    add_column(
        &mut store,
        "Actuals",
        "NothingYTD",
        "ytd(`Nothing`, fy_start=2)",
    );
    assert!(
        by_month(&store, "NothingYTD")
            .iter()
            .all(|(_, value)| value.is_empty()),
        "{:?}",
        by_month(&store, "NothingYTD")
    );
}

#[test]
fn ytd_reads_through_a_joined_frame() {
    let mut store = Store::new(Document::blank("J"));
    store
        .apply(Operation::AddFrame {
            name: "Actuals".into(),
            grid: vec![
                vec!["Month".into(), "Revenue".into()],
                vec!["2025-01-01".into(), "100".into()],
                vec!["2025-02-01".into(), "110".into()],
                vec!["2025-03-01".into(), "120".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::AddGeneratorFrame {
            name: "Spine".into(),
            formula: "sequence(2025-01-01, 2025-04-01, 1mo)".into(),
            column_name: Some("Month".into()),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let actuals = frame_named(store.document(), "Actuals").clone();
    let spine = frame_named(store.document(), "Spine").clone();
    let actuals_month = actuals
        .columns
        .iter()
        .find(|c| c.name == "Month")
        .unwrap()
        .id
        .clone();
    let actuals_revenue = actuals
        .columns
        .iter()
        .find(|c| c.name == "Revenue")
        .unwrap()
        .id
        .clone();
    let spine_month = spine
        .columns
        .iter()
        .find(|c| c.name == "Month")
        .unwrap()
        .id
        .clone();
    store
        .apply(Operation::SetUniqueKey {
            frame_id: actuals.id.clone(),
            column_ids: vec![actuals_month.clone()],
            enabled: true,
        })
        .unwrap();
    store
        .apply(Operation::AddJoinFrame {
            primary_frame_id: spine.id.clone(),
            lookup_frame_id: actuals.id.clone(),
            primary_key_column_ids: vec![spine_month.clone()],
            lookup_key_column_ids: vec![actuals_month],
            join_type: FrameJoinType::Left,
            columns: vec![
                JoinColumnInput {
                    source_frame_id: spine.id.clone(),
                    source_column_id: spine_month,
                    name: "Month".into(),
                },
                JoinColumnInput {
                    source_frame_id: actuals.id.clone(),
                    source_column_id: actuals_revenue,
                    name: "Revenue".into(),
                },
            ],
            name: "Joined".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let joined = frame_named(store.document(), "Joined").clone();
    let joined_month = joined
        .columns
        .iter()
        .find(|c| c.name == "Month")
        .unwrap()
        .id
        .clone();
    store
        .apply(Operation::SetFramePeriod {
            frame_id: joined.id.clone(),
            period: Some(FramePeriod {
                column_id: joined_month,
                partition_column_ids: Vec::new(),
            }),
        })
        .unwrap();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: joined.id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: id(),
                    name: "YTD".into(),
                    formula: "ytd(`Revenue`, fy_start=2)".into(),
                }],
            }],
        })
        .unwrap();
    let page = store.get_frame_page(&joined.id, 0, 10).unwrap();
    let values: Vec<String> = page.rows.iter().map(|row| row[2].clone()).collect();
    // February opens the fiscal year under fy_start=2, so it totals
    // itself; March adds February. The join works on derived frames.
    assert_eq!(values, vec!["100", "110", "230"]);
}

#[test]
fn many_period_reads_in_one_step_stay_linear() {
    // Eight period-relative reads in a single step: every call derives
    // from the same input snapshot and joins back by natural keys, so the
    // plan grows with the calls. Deriving each call from the accumulation
    // instead tripled the plan per call, and three nested doublings
    // overflowed the debug stack.
    let mut store = yearly_store();
    let frame_id = frame_named(store.document(), "Actuals").id.clone();
    let column = |name: &str, formula: &str| ExistingFormulaInput {
        output_column_id: id(),
        name: name.into(),
        formula: formula.into(),
    };
    store
        .apply(Operation::SetFramePipeline {
            frame_id,
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![
                    column("YTD", "ytd(`Revenue`, fy_start=2)"),
                    column("TTM", "ttm(`Revenue`)"),
                    column("SY", "same_period_last_year(`Revenue`)"),
                    column("P1", "prior(`Revenue`, 1)"),
                    column("P2", "prior(`Revenue`, 2)"),
                    column("P3", "prior(`Revenue`, 3)"),
                    column("Cal", "ytd(`Revenue`)"),
                    column("RT", "`Revenue`.finance.ttm()"),
                ],
            }],
        })
        .unwrap();
    let id = frame_named(store.document(), "Actuals").id.clone();
    let frame = frame_named(store.document(), "Actuals");
    let index = |name: &str| {
        frame
            .columns
            .iter()
            .position(|column| column.name == name)
            .unwrap()
    };
    let march: Vec<String> = store
        .get_frame_page(&id, 0, 100)
        .unwrap()
        .rows
        .into_iter()
        .find(|row| row[0] == "2025-03-01")
        .unwrap()
        .into_iter()
        .collect();
    assert_eq!(march[index("YTD")], "230");
    assert_eq!(march[index("TTM")], "330");
    assert_eq!(march[index("SY")], "");
    assert_eq!(march[index("P1")], "110");
    assert_eq!(march[index("P2")], "100");
    assert_eq!(march[index("P3")], "");
    assert_eq!(march[index("Cal")], "330");
    assert_eq!(march[index("RT")], "330");
}

#[test]
fn receiver_and_namespace_spellings_agree_with_root_calls() {
    let mut store = yearly_store();
    add_column(
        &mut store,
        "Actuals",
        "RX",
        "`Revenue`.finance.ytd(fy_start=2)",
    );
    add_column(
        &mut store,
        "Actuals",
        "NS",
        "finance.ytd(`Revenue`, fy_start=2)",
    );
    assert_eq!(by_month(&store, "RX"), by_month(&store, "NS"));

    add_column(&mut store, "Actuals", "TT", "finance.ttm(`Revenue`)");
    add_column(
        &mut store,
        "Actuals",
        "SY",
        "`Revenue`.finance.same_period_last_year()",
    );
    let february = by_month(&store, "TT")
        .into_iter()
        .find(|(month, _)| month == "2026-02-01")
        .unwrap();
    assert_eq!(february.1, "2100");
    let january = by_month(&store, "SY")
        .into_iter()
        .find(|(month, _)| month == "2026-01-01")
        .unwrap();
    assert_eq!(january.1, "100");
}

#[test]
fn ytd_around_a_prior_resolves_the_inner_read_first() {
    let mut store = yearly_store();
    // March sums February's and March's prior-month values: 100 + 110.
    add_column(
        &mut store,
        "Actuals",
        "YTDofPrior",
        "ytd(prior(`Revenue`, 1), fy_start=2)",
    );
    let march: Vec<(String, String)> = by_month(&store, "YTDofPrior")
        .into_iter()
        .filter(|(month, _)| month == "2025-03-01")
        .collect();
    assert_eq!(march, vec![("2025-03-01".into(), "210".into())]);
}

#[test]
fn windows_without_a_declaration_name_the_frame_and_the_fix() {
    let mut store = Store::new(Document::blank("Forecast"));
    store
        .apply(Operation::AddFrame {
            name: "Actuals".into(),
            grid: vec![
                vec!["Month".into(), "Revenue".into()],
                vec!["2025-01-01".into(), "100".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    for formula in [
        "ytd(`Revenue`)",
        "ttm(`Revenue`)",
        "same_period_last_year(`Revenue`)",
    ] {
        let id = frame_named(store.document(), "Actuals").id.clone();
        let result = store.apply(Operation::AddComputedColumn {
            frame_id: id,
            name: "Bad".into(),
            formula: formula.into(),
            after_column_id: None,
        });
        assert!(
            matches!(&result, Err(CoreError::Formula(message)) if message.contains("Actuals") && message.contains("declared period")),
            "{formula}: {result:?}",
        );
    }
    // A stray year start on a twelve-period function is refused, not ignored.
    let mut declared = yearly_store();
    let id = frame_named(declared.document(), "Actuals").id.clone();
    let stray = declared
        .apply(Operation::AddComputedColumn {
            frame_id: id,
            name: "Bad".into(),
            formula: "ttm(`Revenue`, fy_start=2)".into(),
            after_column_id: None,
        })
        .unwrap_err()
        .to_string();
    assert!(
        stray.contains("takes no fy_start"),
        "unexpected error: {stray}",
    );
}

#[test]
fn scratchwork_refuses_windows_the_way_it_refuses_prior() {
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
    // Scratchwork has no frame and no plan to join into, so the refusal
    // surfaces on the line rather than on save.
    store
        .apply(Operation::SetBlockSource {
            block_id: block,
            source: "total = ytd([100, 110])".into(),
            editing: None,
        })
        .unwrap();
    let error = store
        .view()
        .computed_blocks
        .values()
        .next()
        .unwrap()
        .lines
        .first()
        .unwrap()
        .cell
        .error
        .clone()
        .unwrap_or_default();
    assert!(
        error.contains("declared period"),
        "unexpected error: {error}",
    );
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

/// Four dates, one inside each of four consecutive NRF periods across a
/// retail year boundary. Fiscal 2024 runs February 4 2024 through February
/// 1 2025, so February 1 2025 is still P12 of fiscal 2024 and fiscal 2025
/// opens on February 2 — the day month arithmetic gets wrong, since it
/// turns the year on February 1.
fn retail_store() -> Store {
    let mut store = Store::new(Document::blank("Retail"));
    nrf_calendar(&mut store);
    store
        .apply(Operation::AddFrame {
            name: "Actuals".into(),
            grid: vec![
                vec!["Month".into(), "Revenue".into()],
                vec!["2025-02-02".into(), "400".into()],
                vec!["2024-12-01".into(), "100".into()],
                vec!["2025-03-02".into(), "800".into()],
                vec!["2025-02-01".into(), "200".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Actuals").clone();
    let month = frame
        .columns
        .iter()
        .find(|column| column.name == "Month")
        .unwrap()
        .id
        .clone();
    store
        .apply(Operation::SetFramePeriod {
            frame_id: frame.id.clone(),
            period: Some(FramePeriod {
                column_id: month,
                partition_column_ids: Vec::new(),
            }),
        })
        .unwrap();
    store
}

#[test]
fn a_retail_calendar_numbers_windows_by_its_week_table() {
    let mut store = retail_store();
    add_column(
        &mut store,
        "Actuals",
        "Index",
        "period_index(`Month`, calendar=\"NRF\")",
    );
    add_column(
        &mut store,
        "Actuals",
        "Period",
        "fiscal_period(`Month`, calendar=\"NRF\")",
    );
    // The index is twelve blocks a fiscal year, so it agrees with the
    // block `fiscal_period` reports rather than with the calendar month.
    assert_eq!(
        by_month(&store, "Index"),
        vec![
            ("2024-12-01".into(), "24298".into()),
            ("2025-02-01".into(), "24299".into()),
            ("2025-02-02".into(), "24300".into()),
            ("2025-03-02".into(), "24301".into()),
        ]
    );
    assert_eq!(
        by_month(&store, "Period"),
        vec![
            ("2024-12-01".into(), "11".into()),
            ("2025-02-01".into(), "12".into()),
            ("2025-02-02".into(), "1".into()),
            ("2025-03-02".into(), "2".into()),
        ]
    );

    add_column(
        &mut store,
        "Actuals",
        "YTD",
        "ytd(`Revenue`, calendar=\"NRF\")",
    );
    // The retail year turns on February 2, not February 1: February 1 still
    // adds to fiscal 2024's running total, and February 2 starts over.
    // Month arithmetic on the same calendar's February start would reset a
    // day early and put both February rows in one period.
    assert_eq!(
        by_month(&store, "YTD"),
        vec![
            ("2024-12-01".into(), "100".into()),
            ("2025-02-01".into(), "300".into()),
            ("2025-02-02".into(), "400".into()),
            ("2025-03-02".into(), "1200".into()),
        ]
    );

    add_column(
        &mut store,
        "Actuals",
        "Prior",
        "prior(`Revenue`, 1, calendar=\"NRF\")",
    );
    // One period back is one retail block back, so February 2 reads
    // February 1 — the block before it, a single day earlier.
    assert_eq!(
        by_month(&store, "Prior"),
        vec![
            ("2024-12-01".into(), "".into()),
            ("2025-02-01".into(), "100".into()),
            ("2025-02-02".into(), "200".into()),
            ("2025-03-02".into(), "400".into()),
        ]
    );

    add_column(
        &mut store,
        "Actuals",
        "TTM",
        "ttm(`Revenue`, calendar=\"NRF\")",
    );
    // Twelve retail blocks back reaches every earlier row here, so each
    // trailing window is the running total of the blocks present.
    assert_eq!(
        by_month(&store, "TTM"),
        vec![
            ("2024-12-01".into(), "100".into()),
            ("2025-02-01".into(), "300".into()),
            ("2025-02-02".into(), "700".into()),
            ("2025-03-02".into(), "1500".into()),
        ]
    );

    add_column(
        &mut store,
        "Actuals",
        "Last year",
        "same_period_last_year(`Revenue`, calendar=\"NRF\")",
    );
    // Nothing here is a retail year old, so every row reads blank rather
    // than reaching back into a calendar month twelve names earlier.
    assert_eq!(
        by_month(&store, "Last year"),
        vec![
            ("2024-12-01".into(), "".into()),
            ("2025-02-01".into(), "".into()),
            ("2025-02-02".into(), "".into()),
            ("2025-03-02".into(), "".into()),
        ]
    );
}

#[test]
fn the_default_calendar_reaches_bare_windows_and_indexes() {
    let mut store = retail_store();
    let calendar_id = store.document().calendars[0].id.clone();
    store
        .apply(Operation::SetDefaultCalendar {
            calendar_id: Some(calendar_id),
        })
        .unwrap();
    // Named or not, the same calendar answers: a bare window in a retail
    // workbook cannot disagree with the retail `fiscal_period` beside it.
    add_column(&mut store, "Actuals", "Bare", "ytd(`Revenue`)");
    add_column(
        &mut store,
        "Actuals",
        "Named",
        "ytd(`Revenue`, calendar=\"NRF\")",
    );
    assert_eq!(by_month(&store, "Bare"), by_month(&store, "Named"));
    assert_eq!(
        by_month(&store, "Bare"),
        vec![
            ("2024-12-01".into(), "100".into()),
            ("2025-02-01".into(), "300".into()),
            ("2025-02-02".into(), "400".into()),
            ("2025-03-02".into(), "1200".into()),
        ]
    );
    // The new keyword survives the parse-and-format round trip the engine
    // echoes formulas through, so a saved `calendar=` is still the text
    // that was written rather than a reseeded rewrite.
    let frame = frame_named(store.document(), "Actuals").clone();
    let named = frame
        .columns
        .iter()
        .find(|column| column.name == "Named")
        .unwrap()
        .id
        .clone();
    assert_eq!(
        store.view().computed_frames[&frame.id].formulas[&named],
        "ytd(`Revenue`, calendar=\"NRF\")"
    );

    add_column(&mut store, "Actuals", "Bare index", "period_index(`Month`)");
    assert_eq!(
        by_month(&store, "Bare index"),
        vec![
            ("2024-12-01".into(), "24298".into()),
            ("2025-02-01".into(), "24299".into()),
            ("2025-02-02".into(), "24300".into()),
            ("2025-03-02".into(), "24301".into()),
        ]
    );
}

/// Gaps, nulls and a shuffled grid across two year boundaries. The window
/// join names the indexes it wants rather than scanning the partition, so
/// this is the test that the indexes it names are the right ones: a
/// missing month contributes nothing, a present-but-empty month keeps the
/// window readable only if something else in it is readable, and no answer
/// depends on where its row sits.
#[test]
fn windows_read_gaps_and_nulls_across_a_year_boundary() {
    let mut store = Store::new(Document::blank("Gaps"));
    store
        .apply(Operation::AddFrame {
            name: "Actuals".into(),
            grid: vec![
                vec!["Month".into(), "Revenue".into()],
                vec!["2025-03-01".into(), "30".into()],
                vec!["2024-12-01".into(), "12".into()],
                // A whole year with one month in it, and that month empty:
                // every window ending on it has nothing to read.
                vec!["2023-07-01".into(), "".into()],
                vec!["2025-08-01".into(), "80".into()],
                vec!["2024-09-01".into(), "9".into()],
                vec!["2025-01-01".into(), "10".into()],
                // October is missing outright; November is present and empty.
                vec!["2024-11-01".into(), "".into()],
                vec!["2025-05-01".into(), "50".into()],
                vec!["2025-02-01".into(), "20".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Actuals").clone();
    let month = frame
        .columns
        .iter()
        .find(|column| column.name == "Month")
        .unwrap()
        .id
        .clone();
    store
        .apply(Operation::SetFramePeriod {
            frame_id: frame.id.clone(),
            period: Some(FramePeriod {
                column_id: month,
                partition_column_ids: Vec::new(),
            }),
        })
        .unwrap();
    add_column(&mut store, "Actuals", "TTM", "ttm(`Revenue`)");
    add_column(&mut store, "Actuals", "YTD", "ytd(`Revenue`)");
    assert_eq!(
        by_month(&store, "TTM"),
        vec![
            ("2023-07-01".into(), "".into()),
            ("2024-09-01".into(), "9".into()),
            ("2024-11-01".into(), "9".into()),
            ("2024-12-01".into(), "21".into()),
            ("2025-01-01".into(), "31".into()),
            ("2025-02-01".into(), "51".into()),
            ("2025-03-01".into(), "81".into()),
            ("2025-05-01".into(), "131".into()),
            ("2025-08-01".into(), "211".into()),
        ]
    );
    assert_eq!(
        by_month(&store, "YTD"),
        vec![
            ("2023-07-01".into(), "".into()),
            ("2024-09-01".into(), "9".into()),
            ("2024-11-01".into(), "9".into()),
            ("2024-12-01".into(), "21".into()),
            ("2025-01-01".into(), "10".into()),
            ("2025-02-01".into(), "30".into()),
            ("2025-03-01".into(), "60".into()),
            ("2025-05-01".into(), "110".into()),
            ("2025-08-01".into(), "190".into()),
        ]
    );
}

/// Five thousand daily rows through one trailing window. The pairing used
/// to be every row against every other row before the range filter, which
/// is twenty-five million pairs here and minutes of work; naming the
/// twelve indexes each row wants makes it a few passes. The assertion is
/// deliberately loose — this is a "did the quadratic come back" alarm, not
/// a benchmark.
#[test]
fn a_daily_frame_runs_one_window_without_pairing_every_row() {
    let mut store = Store::new(Document::blank("Scale"));
    store
        .apply(Operation::AddGeneratorFrame {
            name: "Days".into(),
            formula: "sequence(2010-01-01, 2023-09-09, 1d)".into(),
            column_name: Some("Day".into()),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Days").clone();
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
    // A generated frame owns no rows, so its columns arrive as a pipeline.
    let column = |name: &str, formula: &str| ExistingFormulaInput {
        output_column_id: id(),
        name: name.into(),
        formula: formula.into(),
    };
    let started = std::time::Instant::now();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![
                FrameStepInput::WithColumns {
                    columns: vec![column("Revenue", "1")],
                },
                FrameStepInput::WithColumns {
                    columns: vec![column("TTM", "ttm(`Revenue`)")],
                },
            ],
        })
        .unwrap();
    let id = frame_named(store.document(), "Days").id.clone();
    let page = store.get_frame_page(&id, 0, 5).unwrap();
    let elapsed = started.elapsed();
    assert!(
        page.total_rows >= 4_900,
        "expected about five thousand days, got {}",
        page.total_rows
    );
    // A window counts whole periods, and a month is one period however
    // many days sit in it: every day of January 2010 reads all thirty-one
    // of them, because the frame starts there and a trailing window
    // reaches eleven months further back into nothing.
    assert_eq!(page.rows[0][2], "31");
    assert_eq!(page.rows[4][2], "31");
    assert!(
        elapsed < std::time::Duration::from_secs(20),
        "one trailing window over {} daily rows took {elapsed:?}",
        page.total_rows
    );
}
