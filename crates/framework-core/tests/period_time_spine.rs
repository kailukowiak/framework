//! Slice 1 of the period-aware time spine: a declared period column,
//! `period_index`, and `prior` as a self-join on the index — the period
//! before, never the row above.
use crate::common::frame_named;
use framework_core::*;

fn calculation(name: &str, formula: &str) -> FrameStepInput {
    FrameStepInput::WithColumns {
        columns: vec![ExistingFormulaInput {
            output_column_id: id(),
            name: name.into(),
            formula: formula.into(),
        }],
    }
}

fn monthly_store() -> Store {
    let mut store = Store::new(Document::blank("Forecast"));
    store
        .apply(Operation::AddFrame {
            name: "Actuals".into(),
            grid: vec![
                vec!["Month".into(), "Revenue".into()],
                vec!["2025-01-01".into(), "100000".into()],
                vec!["2025-02-01".into(), "104000".into()],
                vec!["2025-03-01".into(), "112000".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
}

fn month_id(store: &Store) -> Id {
    frame_named(store.document(), "Actuals")
        .columns
        .iter()
        .find(|column| column.name == "Month")
        .unwrap()
        .id
        .clone()
}

fn declare_period(store: &mut Store) {
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
}

fn priors(store: &Store) -> Vec<Vec<String>> {
    let frame_id = frame_named(store.document(), "Actuals").id.clone();
    store.get_frame_page(&frame_id, 0, 100).unwrap().rows
}

fn block_values(store: &Store) -> Vec<f64> {
    store
        .view()
        .computed_blocks
        .values()
        .next()
        .unwrap()
        .lines
        .iter()
        .map(|line| {
            assert!(line.cell.error.is_none(), "{:?}", line.cell.error);
            line.cell.value.unwrap()
        })
        .collect()
}

fn set_block(store: &mut Store, source: &str) {
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
    store
        .apply(Operation::SetBlockSource {
            block_id: block,
            source: source.into(),
            editing: None,
        })
        .unwrap();
}

#[test]
fn declaring_a_period_round_trips_through_history_and_reload() {
    let mut store = monthly_store();
    declare_period(&mut store);
    assert!(frame_named(store.document(), "Actuals").period.is_some());
    store.undo();
    assert!(frame_named(store.document(), "Actuals").period.is_none());
    store.redo();
    assert!(frame_named(store.document(), "Actuals").period.is_some());
    // Clearing is the same operation with no declaration.
    let frame_id = frame_named(store.document(), "Actuals").id.clone();
    store
        .apply(Operation::SetFramePeriod {
            frame_id: frame_id.clone(),
            period: None,
        })
        .unwrap();
    assert!(frame_named(store.document(), "Actuals").period.is_none());
    store.undo();
    assert!(frame_named(store.document(), "Actuals").period.is_some());

    let directory = std::env::temp_dir().join(format!("framework-period-{}", id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("forecast.fw");
    store.save(&path).unwrap();
    let loaded = Store::load(&path).unwrap();
    assert!(frame_named(loaded.document(), "Actuals").period.is_some());
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn declaring_a_period_refuses_non_dates_and_unknown_columns() {
    let mut store = monthly_store();
    let frame = frame_named(store.document(), "Actuals").clone();
    let revenue = frame
        .columns
        .iter()
        .find(|column| column.name == "Revenue")
        .unwrap()
        .id
        .clone();
    let refused = store.apply(Operation::SetFramePeriod {
        frame_id: frame.id.clone(),
        period: Some(FramePeriod {
            column_id: revenue,
            partition_column_ids: Vec::new(),
        }),
    });
    assert!(
        refused.unwrap_err().to_string().contains("not dates"),
        "a revenue column is not a period"
    );
    let missing = store.apply(Operation::SetFramePeriod {
        frame_id: frame.id.clone(),
        period: Some(FramePeriod {
            column_id: month_id(&store),
            partition_column_ids: vec!["no-such-column".into()],
        }),
    });
    assert!(missing.is_err(), "unknown partition columns are refused");
    let own = store.apply(Operation::SetFramePeriod {
        frame_id: frame.id.clone(),
        period: Some(FramePeriod {
            column_id: month_id(&store),
            partition_column_ids: vec![month_id(&store)],
        }),
    });
    assert!(
        own.unwrap_err()
            .to_string()
            .contains("cannot partition itself"),
        "a period cannot partition itself"
    );
}

#[test]
fn duplicate_and_missing_periods_fail_validation() {
    let mut store = Store::new(Document::blank("Messy"));
    store
        .apply(Operation::AddFrame {
            name: "Actuals".into(),
            grid: vec![
                vec!["Month".into(), "Revenue".into()],
                vec!["2025-01-01".into(), "100000".into()],
                vec!["2025-01-01".into(), "104000".into()],
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
    let refused = store.apply(Operation::SetFramePeriod {
        frame_id: frame.id.clone(),
        period: Some(FramePeriod {
            column_id: month,
            partition_column_ids: Vec::new(),
        }),
    });
    assert!(
        refused.unwrap_err().to_string().contains("same period"),
        "duplicate months cannot be a period"
    );

    // And a duplicate typed in after declaring fails the same way: the
    // invariant holds across edits, not just at declaration time.
    let mut store = monthly_store();
    declare_period(&mut store);
    let frame = frame_named(store.document(), "Actuals").clone();
    let refused = store.apply(Operation::SetCell {
        frame_id: frame.id.clone(),
        row_id: frame.rows[2].id.clone(),
        column_id: frame.columns[0].id.clone(),
        raw: "2025-02-01".into(),
    });
    assert!(
        refused.unwrap_err().to_string().contains("same period"),
        "editing a duplicate month is refused"
    );
}

#[test]
fn period_index_counts_calendar_and_fiscal_months() {
    let mut store = Store::new(Document::blank("Indexes"));
    set_block(
        &mut store,
        "calendar = period_index(date(2026, 1, 15))\n\
         fiscal = period_index(date(2026, 1, 15), 2)\n\
         boundary = period_index(date(2025, 2, 1), 2)\n\
         same = period_index(date(2026, 1, 31), 2)\n\
         namespaced = finance.period_index(date(2026, 1, 15))",
    );
    // January 2026 is calendar month 2026*12; in a February fiscal year it
    // is the twelfth month of fiscal 2025. February 2025 opens fiscal 2025,
    // and the last day of January 2026 shares January's index — while
    // January 2025, a year earlier, is fiscal December of FY2024.
    assert_eq!(
        block_values(&store),
        vec![24312.0, 24311.0, 24300.0, 24311.0, 24312.0]
    );
    for formula in [
        "period_index(date(2026, 1, 1), 0)",
        "period_index(date(2026, 1, 1), 13)",
        "period_index(5)",
        "period_index(date(2026, 1, 1), 2.5)",
    ] {
        let mut probe = Store::new(Document::blank("Bad index"));
        set_block(&mut probe, formula);
        assert!(
            probe
                .view()
                .computed_blocks
                .values()
                .next()
                .unwrap()
                .lines
                .last()
                .unwrap()
                .cell
                .error
                .is_some(),
            "{formula} should fail"
        );
    }
}

#[test]
fn prior_reads_the_period_before_not_the_row_above() {
    let mut store = monthly_store();
    declare_period(&mut store);
    let frame_id = frame_named(store.document(), "Actuals").id.clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![calculation("Prior month", "prior(`Revenue`, 1)")],
        })
        .unwrap();
    let rows = priors(&store);
    // No sort is declared anywhere: the join finds January for February by
    // date, not by position.
    assert_eq!(rows[0][2], "");
    assert_eq!(rows[1][2], "100000");
    assert_eq!(rows[2][2], "104000");
}

#[test]
fn deleting_a_month_blanks_its_successor() {
    let mut store = monthly_store();
    declare_period(&mut store);
    let frame_id = frame_named(store.document(), "Actuals").id.clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![calculation("Prior month", "prior(`Revenue`, 1)")],
        })
        .unwrap();
    let february = frame_named(store.document(), "Actuals").rows[1].id.clone();
    store
        .apply(Operation::DeleteRow {
            frame_id: frame_id.clone(),
            row_id: february,
        })
        .unwrap();
    // February is gone, so March has no previous period. A positional shift
    // would read January's 100000 here instead.
    let rows = priors(&store);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1][2], "");
    store.undo();
    assert_eq!(priors(&store)[2][2], "104000");
}

#[test]
fn shuffled_rows_answer_identically() {
    let mut ordered = monthly_store();
    declare_period(&mut ordered);
    let frame_id = frame_named(ordered.document(), "Actuals").id.clone();
    store_pipeline(&mut ordered, &frame_id);

    let mut shuffled = Store::new(Document::blank("Forecast"));
    shuffled
        .apply(Operation::AddFrame {
            name: "Actuals".into(),
            grid: vec![
                vec!["Month".into(), "Revenue".into()],
                vec!["2025-03-01".into(), "112000".into()],
                vec!["2025-01-01".into(), "100000".into()],
                vec!["2025-02-01".into(), "104000".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    declare_period(&mut shuffled);
    let frame_id = frame_named(shuffled.document(), "Actuals").id.clone();
    store_pipeline(&mut shuffled, &frame_id);

    // Compare by month, since stored row order differs by construction.
    let by_month = |store: &Store| {
        let frame_id = frame_named(store.document(), "Actuals").id.clone();
        store
            .get_frame_page(&frame_id, 0, 100)
            .unwrap()
            .rows
            .into_iter()
            .map(|row| (row[0].clone(), row[2].clone()))
            .collect::<Vec<_>>()
    };
    let mut ordered = by_month(&ordered);
    let mut shuffled_rows = by_month(&shuffled);
    ordered.sort();
    shuffled_rows.sort();
    assert_eq!(ordered, shuffled_rows);
}

fn store_pipeline(store: &mut Store, frame_id: &str) {
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.to_string(),
            steps: vec![calculation("Prior month", "prior(`Revenue`, 1)")],
        })
        .unwrap();
}

#[test]
fn prior_without_a_declaration_names_the_frame_and_the_fix() {
    let mut store = monthly_store();
    let frame_id = frame_named(store.document(), "Actuals").id.clone();
    // Like a shift without a declared sort, this is refused when the step
    // is saved — not when the frame is read — with the frame named and the
    // fix stated.
    let refused = store.apply(Operation::SetFramePipeline {
        frame_id: frame_id.clone(),
        steps: vec![calculation("Prior month", "prior(`Revenue`, 1)")],
    });
    let message = refused.unwrap_err().to_string();
    assert!(
        message.contains("Actuals") && message.contains("declared period"),
        "unexpected error: {message}"
    );
    // Nothing else about the frame is refused: declaring fixes the same
    // pipeline untouched.
    declare_period(&mut store);
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![calculation("Prior month", "prior(`Revenue`, 1)")],
        })
        .unwrap();
    assert_eq!(priors(&store)[1][2], "100000");
}

#[test]
fn prior_follows_upstream_edits_and_fiscal_boundaries() {
    let mut store = Store::new(Document::blank("Fiscal"));
    store
        .apply(Operation::AddFrame {
            name: "Actuals".into(),
            grid: vec![
                vec!["Month".into(), "Revenue".into()],
                vec!["2025-01-01".into(), "100000".into()],
                vec!["2025-02-01".into(), "104000".into()],
                vec!["2025-03-01".into(), "112000".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    declare_period(&mut store);
    let frame = frame_named(store.document(), "Actuals").clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![calculation(
                "Prior month",
                "prior(`Revenue`, 1, fy_start=2)",
            )],
        })
        .unwrap();
    // January 2025 is fiscal December of FY2024; February opens FY2025, one
    // index later. The join crosses the year boundary by arithmetic.
    assert_eq!(priors(&store)[1][2], "100000");
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: frame.rows[0].id.clone(),
            column_id: frame.columns[1].id.clone(),
            raw: "90000".into(),
        })
        .unwrap();
    assert_eq!(priors(&store)[1][2], "90000");
    store.undo();
    assert_eq!(priors(&store)[1][2], "100000");
}

#[test]
fn receiver_and_case_insensitive_spellings_share_the_join() {
    let mut store = monthly_store();
    declare_period(&mut store);
    let frame_id = frame_named(store.document(), "Actuals").id.clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![
                calculation("Prior month", "`Revenue`.finance.prior(1)"),
                calculation("Prior again", "PRIOR(`Revenue`, 1)"),
            ],
        })
        .unwrap();
    let rows = priors(&store);
    assert_eq!(rows[1][2], "100000");
    assert_eq!(rows[1][3], "100000");
    assert_eq!(rows[2][2], "104000");
    assert_eq!(rows[2][3], "104000");
}
