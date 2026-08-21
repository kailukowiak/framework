use crate::common::*;
use framework_core::*;

use std::path::PathBuf;

/// Sales, and a grand total derived from it. Nothing is materialized yet —
/// each test decides what to make readable and when.
fn ledger_store(name: &str) -> (Store, PathBuf, String, String) {
    let directory = temporary_test_directory(name);
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Sales".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Sales".into(),
            grid: vec![
                vec!["Region".into(), "Amount".into()],
                vec!["East".into(), "100".into()],
                vec!["West".into(), "20".into()],
                vec!["East".into(), "5".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let sales_id = frame_named(store.document(), "Sales").id.clone();
    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: sales_id.clone(),
            name: "Totals".into(),
            group_keys: Vec::new(),
            aggregates: vec![NamedFormulaInput {
                name: "Grand total".into(),
                formula: "`Amount`.sum()".into(),
            }],
            maintain_order: true,
            x: 400.0,
            y: 0.0,
        })
        .unwrap();
    let totals_id = frame_named(store.document(), "Totals").id.clone();
    (store, directory, sales_id, totals_id)
}

fn column_values(store: &Store, frame_id: &str, name: &str) -> Vec<String> {
    let page = store.get_frame_page(frame_id, 0, 1000).unwrap();
    let index = page
        .columns
        .iter()
        .position(|column| column.name == name)
        .unwrap();
    page.rows.iter().map(|row| row[index].clone()).collect()
}

/// The workflow this exists for: summarize a frame down to one row,
/// materialize it, and then multiply by it from somewhere else.
#[test]
fn a_one_row_frame_can_be_read_as_a_value_from_another_frame() {
    let (mut store, directory, sales_id, totals_id) = ledger_store("cross-value");
    store
        .materialize_frame(&totals_id, &directory.join("data"))
        .unwrap();

    store
        .apply(Operation::AddComputedColumn {
            frame_id: sales_id.clone(),
            name: "Share".into(),
            formula: "`Amount` / `Totals`.`Grand total`".into(),
            after_column_id: None,
        })
        .unwrap();
    assert_eq!(
        column_values(&store, &sales_id, "Share"),
        vec!["0.8", "0.16", "0.04"]
    );

    // It reads back as what was written, rather than as an id.
    let share = frame_named(store.document(), "Sales")
        .columns
        .iter()
        .find(|column| column.name == "Share")
        .unwrap()
        .id
        .clone();
    let view = store.view();
    assert_eq!(
        view.computed_frames[&sales_id].formulas[&share],
        "`Amount` / `Totals`.`Grand total`"
    );
}

/// Share-of-total reads back through the frame it came from, and has to
/// keep working — it is the commonest cross-frame formula there is.
///
/// It is not the paradox it looks like. A snapshot is a recorded value, not
/// a live computation, so reading one is reading a number somebody wrote
/// down. Refreshing sets aside only the snapshot being replaced, and the
/// recompute below it finds the previous one still sitting there and stops.
#[test]
fn a_total_of_a_frame_can_be_read_back_by_the_frame_it_totals() {
    let (mut store, directory, sales_id, totals_id) = ledger_store("cross-share");
    let data = directory.join("data");
    store.materialize_frame(&totals_id, &data).unwrap();
    store
        .apply(Operation::AddComputedColumn {
            frame_id: sales_id.clone(),
            name: "Share".into(),
            formula: "`Amount` / `Totals`.`Grand total`".into(),
            after_column_id: None,
        })
        .unwrap();

    // Refreshing the total now has to walk back through Sales, which reads
    // the total. It terminates, and the answer is right.
    store.materialize_frame(&totals_id, &data).unwrap();
    assert_eq!(
        column_values(&store, &totals_id, "Grand total"),
        vec!["125"]
    );
    assert_eq!(
        column_values(&store, &sales_id, "Share"),
        vec!["0.8", "0.16", "0.04"]
    );

    // And a new row moves both, once the snapshot is refreshed.
    let row_id = frame_named(store.document(), "Sales").rows[0].id.clone();
    let amount = frame_named(store.document(), "Sales")
        .columns
        .iter()
        .find(|column| column.name == "Amount")
        .unwrap()
        .id
        .clone();
    store
        .apply(Operation::SetCell {
            frame_id: sales_id.clone(),
            row_id,
            column_id: amount,
            raw: "200".into(),
        })
        .unwrap();
    store.materialize_frame(&totals_id, &data).unwrap();
    assert_eq!(
        column_values(&store, &totals_id, "Grand total"),
        vec!["225"]
    );
}

/// The rule that keeps the dangerous case from happening by accident. Two
/// frames whose row counts happen to agree would otherwise be zipped
/// together by position, on no key at all, with nothing said about it.
#[test]
fn a_column_of_a_longer_frame_is_a_list_and_has_to_be_used_as_one() {
    let (mut store, directory, sales_id, _) = ledger_store("cross-list");
    // A three-row frame, materialized: exactly as long as Sales, which is
    // what makes the silent positional zip available to be refused.
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: sales_id.clone(),
            name: "Regions".into(),
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let regions_id = frame_named(store.document(), "Regions").id.clone();
    store
        .materialize_frame(&regions_id, &directory.join("data"))
        .unwrap();

    let error = store
        .apply(Operation::AddComputedColumn {
            frame_id: sales_id.clone(),
            name: "Doubled".into(),
            formula: "`Regions`.`Amount` * 2".into(),
            after_column_id: None,
        })
        .unwrap_err()
        .to_string();
    assert!(error.contains("3 rows"), "{error}");
    assert!(error.contains("list of 3 values"), "{error}");

    // Handed to something that takes a list, the same reference is fine.
    store
        .apply(Operation::AddComputedColumn {
            frame_id: sales_id.clone(),
            name: "Known region".into(),
            formula: "`Region`.is_in(`Regions`.`Region`)".into(),
            after_column_id: None,
        })
        .unwrap();
    assert_eq!(
        column_values(&store, &sales_id, "Known region"),
        vec!["true", "true", "true"]
    );
}

/// Nobody can type a syntax they have not been shown, so a materialized
/// frame's columns are offered from the same backtick that offers this
/// frame's own — qualified, and saying how many rows are behind them.
#[test]
fn the_editor_offers_the_columns_of_every_frame_that_can_be_read() {
    let (mut store, directory, sales_id, totals_id) = ledger_store("cross-complete");
    let before = framework_core::complete_formula(store.document(), &sales_id, "`", 1);
    assert!(
        !before
            .suggestions
            .iter()
            .any(|suggestion| suggestion.label.contains("Totals")),
        "a frame with no snapshot is not offered: {:?}",
        before.suggestions
    );

    store
        .materialize_frame(&totals_id, &directory.join("data"))
        .unwrap();
    let after = framework_core::complete_formula(store.document(), &sales_id, "`", 1);
    let offered = after
        .suggestions
        .iter()
        .find(|suggestion| suggestion.label == "Totals.Grand total")
        .expect("the snapshot's columns are offered");
    assert_eq!(offered.insert_text, "Totals`.`Grand total`");
    assert!(
        offered.detail.contains("value from Totals"),
        "one row, so it is a value: {}",
        offered.detail
    );
}

/// A frame with no snapshot is not refused so much as deferred, and the
/// message says which action makes the reference work.
#[test]
fn a_frame_has_to_be_materialized_before_anything_can_read_it() {
    let (mut store, directory, sales_id, totals_id) = ledger_store("cross-live");
    let error = store
        .apply(Operation::AddComputedColumn {
            frame_id: sales_id.clone(),
            name: "Share".into(),
            formula: "`Amount` / `Totals`.`Grand total`".into(),
            after_column_id: None,
        })
        .unwrap_err()
        .to_string();
    assert!(error.contains("has to be materialized"), "{error}");

    store
        .materialize_frame(&totals_id, &directory.join("data"))
        .unwrap();
    store
        .apply(Operation::AddComputedColumn {
            frame_id: sales_id,
            name: "Share".into(),
            formula: "`Amount` / `Totals`.`Grand total`".into(),
            after_column_id: None,
        })
        .unwrap();
}

/// A reference is a lineage edge, so everything that travels an edge has to
/// travel this one: what a snapshot is stale against, and what order
/// snapshots refresh in.
#[test]
fn reading_another_frame_puts_it_upstream_for_staleness_and_refresh_order() {
    let (mut store, directory, sales_id, totals_id) = ledger_store("cross-lineage");
    let data = directory.join("data");
    store.materialize_frame(&totals_id, &data).unwrap();
    store
        .apply(Operation::AddComputedColumn {
            frame_id: sales_id.clone(),
            name: "Share".into(),
            formula: "`Amount` / `Totals`.`Grand total`".into(),
            after_column_id: None,
        })
        .unwrap();

    // Sales now reads Totals, so a snapshot of Sales has to be refreshed
    // after Totals rather than before it.
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: sales_id.clone(),
            name: "Report".into(),
            x: 0.0,
            y: 800.0,
        })
        .unwrap();
    let report_id = frame_named(store.document(), "Report").id.clone();
    store.materialize_frame(&report_id, &data).unwrap();
    let order = store.snapshot_refresh_order();
    let totals_at = order.iter().position(|id| *id == totals_id).unwrap();
    let report_at = order.iter().position(|id| *id == report_id).unwrap();
    assert!(
        totals_at < report_at,
        "the frame being read is rewritten first: {order:?}"
    );

    // And changing Totals makes the snapshot that reads it stale.
    assert!(!store.snapshot_is_stale(&report_id));
    store
        .apply(Operation::RenameColumn {
            frame_id: totals_id,
            column_id: frame_named(store.document(), "Totals").columns[0]
                .id
                .clone(),
            name: "Total".into(),
        })
        .unwrap();
    assert!(
        store.snapshot_is_stale(&report_id),
        "a change upstream of the formula reaches the snapshot that depends on it"
    );
}

/// Nothing a formula points at is allowed to vanish under it: not the
/// frame, not the column, and not the snapshot that made it readable.
#[test]
fn what_a_formula_reads_cannot_be_deleted_out_from_under_it() {
    let (mut store, directory, sales_id, totals_id) = ledger_store("cross-guards");
    store
        .materialize_frame(&totals_id, &directory.join("data"))
        .unwrap();
    store
        .apply(Operation::AddComputedColumn {
            frame_id: sales_id,
            name: "Share".into(),
            formula: "`Amount` / `Totals`.`Grand total`".into(),
            after_column_id: None,
        })
        .unwrap();

    assert!(matches!(
        store.apply(Operation::DeleteObject {
            object_id: totals_id.clone(),
        }),
        Err(CoreError::ReferencedByFormula(_))
    ));
    // The snapshot is what made the frame readable, so going back to live
    // is refused too. Refreshing it is not — that replaces the snapshot
    // rather than removing it.
    assert!(matches!(
        store.apply(Operation::ClearFrameMaterialization {
            frame_id: totals_id.clone(),
        }),
        Err(CoreError::ReferencedByFormula(_))
    ));
    store
        .materialize_frame(&totals_id, &directory.join("data"))
        .unwrap();

    // And rewriting the chain so it stops producing the column that is
    // being read is the same refusal from the other direction.
    assert!(matches!(
        store.apply(Operation::SetFramePipeline {
            frame_id: totals_id,
            steps: vec![FrameStepInput::Summarize {
                group_keys: Vec::new(),
                aggregates: vec![ExistingFormulaInput {
                    output_column_id: framework_core::id(),
                    name: "How many".into(),
                    formula: "`Amount`.count()".into(),
                }],
                maintain_order: true,
            }],
        }),
        Err(CoreError::ReferencedByFormula(_))
    ));
}
