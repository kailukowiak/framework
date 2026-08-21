use crate::common::*;
use framework_core::*;
use std::fs;
use std::path::PathBuf;

fn paged_sorted_fixture(directory_name: &str, row_count: usize) -> (PathBuf, Store, Id, Id) {
    let directory = temporary_test_directory(directory_name);
    let source = directory.join("rows.csv");
    let mut contents = String::with_capacity(row_count * 14 + 16);
    contents.push_str("Name,Score\n");
    for index in 0..row_count {
        let score = row_count - index;
        contents.push_str(&format!("Row{index},{score}\n"));
    }
    fs::write(&source, contents).unwrap();

    let mut store = Store::new(Document::blank("Paged"));
    store
        .apply(Operation::ImportFrameFromFile {
            name: "Rows".into(),
            path: source.display().to_string(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Rows").clone();
    assert!(store.view().computed_frames[&frame.id].paged);
    let score_id = frame.columns[1].id.clone();
    store
        .apply(Operation::SetFrameDisplaySort {
            frame_id: frame.id.clone(),
            keys: vec![DerivedSort {
                column_id: score_id.clone(),
                descending: false,
            }],
        })
        .unwrap();

    (directory, store, frame.id, score_id)
}

#[test]
fn sorted_page_cache_reuses_the_computed_order_across_page_fetches() {
    let (directory, store, frame_id, _score_id) = paged_sorted_fixture("sorted-cache-reuse", 50);

    let first = store.get_frame_page(&frame_id, 0, 10).unwrap();
    assert_eq!(first.total_rows, 50);
    assert_eq!(store.sorted_page_cache_computations(), 1);

    let _ = store.get_frame_page(&frame_id, 10, 10).unwrap();
    let _ = store.get_frame_page(&frame_id, 20, 10).unwrap();
    let _ = store.get_frame_page(&frame_id, 40, 10).unwrap();
    assert_eq!(
        store.sorted_page_cache_computations(),
        1,
        "repeated page fetches under an unchanged sort must reuse the cached order"
    );

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn sorted_page_cache_covers_a_persistent_sort_with_no_display_sort() {
    let (directory, mut store, frame_id, score_id) =
        paged_sorted_fixture("persistent-sort-cache", 50);
    store
        .apply(Operation::SetFrameDisplaySort {
            frame_id: frame_id.clone(),
            keys: Vec::new(),
        })
        .unwrap();

    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: frame_id.clone(),
            name: "Sorted rows".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let derived = frame_named(store.document(), "Sorted rows").clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: derived.id.clone(),
            steps: vec![FrameStepInput::Sort {
                keys: vec![SortInput {
                    column_id: score_id.clone(),
                    descending: true,
                }],
            }],
        })
        .unwrap();

    let before = store.sorted_page_cache_computations();
    let first = store.get_frame_page(&derived.id, 0, 10).unwrap();
    assert_eq!(first.total_rows, 50);
    assert_eq!(
        store.sorted_page_cache_computations(),
        before + 1,
        "a persistent sort must be computed through the page cache"
    );

    for offset in [10, 20, 40] {
        let _ = store.get_frame_page(&derived.id, offset, 10).unwrap();
    }
    assert_eq!(
        store.sorted_page_cache_computations(),
        before + 1,
        "later pages of a persistently sorted frame must reuse that order"
    );

    assert_eq!(first.rows[0][0], "Row0");
    assert_eq!(first.rows[0][1], "50");

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn sorted_page_cache_produces_the_same_rows_as_a_direct_sort() {
    let (directory, store, frame_id, _score_id) =
        paged_sorted_fixture("sorted-cache-correctness", 37);

    let page_b = store.get_frame_page(&frame_id, 10, 10).unwrap();
    let page_a = store.get_frame_page(&frame_id, 0, 10).unwrap();
    let page_c = store.get_frame_page(&frame_id, 30, 10).unwrap();

    let expected_scores: Vec<String> = (1..=37).map(|score| score.to_string()).collect();
    assert_eq!(
        page_a
            .rows
            .iter()
            .map(|row| row[1].clone())
            .collect::<Vec<_>>(),
        expected_scores[0..10]
    );
    assert_eq!(
        page_b
            .rows
            .iter()
            .map(|row| row[1].clone())
            .collect::<Vec<_>>(),
        expected_scores[10..20]
    );
    assert_eq!(
        page_c
            .rows
            .iter()
            .map(|row| row[1].clone())
            .collect::<Vec<_>>(),
        expected_scores[30..37]
    );

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn sorted_page_cache_invalidates_when_the_sort_changes() {
    let (directory, mut store, frame_id, _score_id) =
        paged_sorted_fixture("sorted-cache-sort-change", 50);
    let name_id = frame_named(store.document(), "Rows").columns[0].id.clone();

    let _ = store.get_frame_page(&frame_id, 0, 10).unwrap();
    assert_eq!(store.sorted_page_cache_computations(), 1);

    store
        .apply(Operation::SetFrameDisplaySort {
            frame_id: frame_id.clone(),
            keys: vec![DerivedSort {
                column_id: name_id,
                descending: true,
            }],
        })
        .unwrap();

    let _ = store.get_frame_page(&frame_id, 0, 10).unwrap();
    assert_eq!(
        store.sorted_page_cache_computations(),
        2,
        "changing the sort keys must invalidate the cached order"
    );

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn sorted_page_cache_invalidates_when_filters_change() {
    let (directory, mut store, frame_id, _score_id) =
        paged_sorted_fixture("sorted-cache-filter-change", 50);

    let _ = store.get_frame_page(&frame_id, 0, 10).unwrap();
    assert_eq!(store.sorted_page_cache_computations(), 1);

    store
        .apply(Operation::SetFrameDisplayFilter {
            frame_id: frame_id.clone(),
            filters: vec!["`Score` > 10".into()],
            filter_match_all: true,
        })
        .unwrap();

    let _ = store.get_frame_page(&frame_id, 0, 10).unwrap();
    assert_eq!(
        store.sorted_page_cache_computations(),
        2,
        "changing the display filters must invalidate the cached order"
    );

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn sorted_page_cache_survives_unrelated_edits_but_not_upstream_ones() {
    let (directory, mut store, frame_id, _score_id) =
        paged_sorted_fixture("sorted-cache-lineage", 50);

    let _ = store.get_frame_page(&frame_id, 0, 10).unwrap();
    assert_eq!(store.sorted_page_cache_computations(), 1);

    store
        .apply(Operation::RenameObject {
            object_id: frame_id.clone(),
            name: "Rows Renamed".into(),
        })
        .unwrap();
    let _ = store.get_frame_page(&frame_id, 0, 10).unwrap();
    assert_eq!(
        store.sorted_page_cache_computations(),
        1,
        "renaming a frame must not throw away its computed order"
    );

    store
        .apply(Operation::AddFrame {
            name: "Somewhere else".into(),
            grid: vec![vec!["A".into()], vec!["1".into()]],
            x: 900.0,
            y: 0.0,
        })
        .unwrap();
    let _ = store.get_frame_page(&frame_id, 0, 10).unwrap();
    assert_eq!(
        store.sorted_page_cache_computations(),
        1,
        "an edit to another frame must not invalidate this one's cache"
    );

    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Doubled".into(),
            formula: "`Score` * 2".into(),
            after_column_id: None,
        })
        .unwrap();
    let _ = store.get_frame_page(&frame_id, 0, 10).unwrap();
    assert_eq!(
        store.sorted_page_cache_computations(),
        2,
        "changing the frame's own columns must invalidate the cached order"
    );

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn sorted_page_cache_survives_undo_of_an_unrelated_edit() {
    let (directory, mut store, frame_id, _score_id) = paged_sorted_fixture("sorted-cache-undo", 50);

    let _ = store.get_frame_page(&frame_id, 0, 10).unwrap();
    assert_eq!(store.sorted_page_cache_computations(), 1);

    store
        .apply(Operation::RenameObject {
            object_id: frame_id.clone(),
            name: "Renamed".into(),
        })
        .unwrap();
    store.undo();

    let _ = store.get_frame_page(&frame_id, 0, 10).unwrap();
    assert_eq!(
        store.sorted_page_cache_computations(),
        1,
        "undoing a rename must not force a re-sort"
    );

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn sorted_page_cache_first_sorted_page_is_much_slower_than_cached_pages() {
    use std::time::Instant;

    let row_count: usize = 1_000_000;
    let (directory, store, frame_id, _score_id) = {
        let directory = temporary_test_directory("sorted-cache-perf");
        let source = directory.join("rows.csv");
        let mut contents = String::with_capacity(row_count * 14);
        contents.push_str("Name,Score\n");
        for index in 0..row_count {
            let score = (index * 2_654_435_761) % row_count;
            contents.push_str(&format!("Row{index},{score}\n"));
        }
        fs::write(&source, contents).unwrap();

        let mut store = Store::new(Document::blank("Paged"));
        store
            .apply(Operation::ImportFrameFromFile {
                name: "Rows".into(),
                path: source.display().to_string(),
                x: 0.0,
                y: 0.0,
            })
            .unwrap();
        let frame = frame_named(store.document(), "Rows").clone();
        let score_id = frame.columns[1].id.clone();
        store
            .apply(Operation::SetFrameDisplaySort {
                frame_id: frame.id.clone(),
                keys: vec![DerivedSort {
                    column_id: score_id.clone(),
                    descending: false,
                }],
            })
            .unwrap();
        (directory, store, frame.id, score_id)
    };

    let start = Instant::now();
    let first_page = store.get_frame_page(&frame_id, 0, 1000).unwrap();
    let first_page_duration = start.elapsed();
    assert_eq!(first_page.total_rows, row_count);

    let start = Instant::now();
    for offset in (1000..row_count).step_by(1000).take(20) {
        let _ = store.get_frame_page(&frame_id, offset, 1000).unwrap();
    }
    let twenty_subsequent_pages_duration = start.elapsed();
    let average_subsequent_page = twenty_subsequent_pages_duration / 20;

    assert!(
        average_subsequent_page * 10 < first_page_duration,
        "expected cached pages to be much faster than the first sorted page: \
         first page = {first_page_duration:?}, average subsequent page = {average_subsequent_page:?}"
    );
    assert_eq!(
        store.sorted_page_cache_computations(),
        1,
        "only the first sorted page fetch should have triggered a full sort"
    );

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn grouped_aggregates_over_an_imported_frame_read_back_through_pages() {
    let directory = temporary_test_directory("grouped-paged-read");
    let source = directory.join("ledger.csv");
    fs::write(
        &source,
        "Period,Debit\n2024-01,100\n2024-02,20\n2024-01,5\n",
    )
    .unwrap();

    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Ledger".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::ImportFrameFromFile {
            name: "Ledger".into(),
            path: source.display().to_string(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let ledger_id = frame_named(store.document(), "Ledger").id.clone();
    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: ledger_id,
            name: "By period".into(),
            group_keys: vec![NamedFormulaInput {
                name: "Period".into(),
                formula: "`Period`".into(),
            }],
            aggregates: vec![NamedFormulaInput {
                name: "Debit total".into(),
                formula: "`Debit`.sum()".into(),
            }],
            maintain_order: true,
            x: 400.0,
            y: 0.0,
        })
        .unwrap();

    let grouped_id = frame_named(store.document(), "By period").id.clone();
    let computed = store.view().computed_frames[&grouped_id].clone();
    assert!(
        computed.paged,
        "a result derived from an artifact-backed frame is read through pages"
    );
    assert_eq!(
        computed.total_rows, None,
        "counting a grouped result means running it, so the count comes from the page"
    );

    let page = store.get_frame_page(&grouped_id, 0, 10).unwrap();
    assert_eq!(page.total_rows, 2);
    assert_eq!(page.rows.len(), 2);
    let totals = page
        .rows
        .iter()
        .map(|row| {
            (
                row[0].clone(),
                row[1]
                    .parse::<f64>()
                    .expect("the aggregate reaches the page"),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        totals,
        vec![
            ("2024-01".to_string(), 105.0),
            ("2024-02".to_string(), 20.0)
        ]
    );

    fs::remove_dir_all(directory).unwrap();
}

/// A rule is only ever run over the rows being read, so a page carries its
/// own matches — and they follow the page's sort rather than the frame's
/// stored order.
#[test]
fn conditional_formatting_rules_match_the_rows_a_page_returns() {
    let (directory, mut store, frame_id, score_id) =
        paged_sorted_fixture("conditional-formatting-paged", 50);

    store
        .apply(Operation::SetFrameStyleRules {
            frame_id: frame_id.clone(),
            rules: vec![FrameStyleRuleInput {
                id: None,
                formula: "`Score` <= 3".into(),
                column_id: Some(score_id.clone()),
                output: FrameStyleOutput::Condition {
                    style: FrameCellStyle {
                        fill_color: Some("#fff0c7".into()),
                        ..FrameCellStyle::default()
                    },
                },
            }],
        })
        .unwrap();
    let rule_id = store
        .document()
        .frame(&frame_id)
        .unwrap()
        .display
        .style_rules[0]
        .id
        .clone();

    // Ascending by Score, so the three matching rows are the first three of
    // the first page and nothing on any later one.
    let first = store.get_frame_page(&frame_id, 0, 10).unwrap();
    assert_eq!(first.style_matches.len(), first.rows.len());
    assert!(first.style_matches[..3].iter().all(|matched| {
        matched.len() == 1
            && matched[0].rule_id == rule_id
            && matched[0].style.fill_color.as_deref() == Some("#fff0c7")
    }));
    assert!(first.style_matches[3..].iter().all(|row| row.is_empty()));

    let later = store.get_frame_page(&frame_id, 10, 10).unwrap();
    assert!(later.style_matches.iter().all(|row| row.is_empty()));

    fs::remove_dir_all(directory).ok();
}

/// Editing a rule revalidates its formula, even when only its colour target
/// changed. That validation has to use the same column semantics as the rule:
/// `.normalize()` mixes frame-wide aggregates with one value per row, and a
/// projection over a wide, chunked CSV can leave those branches at different
/// heights inside Polars. It used to panic the process while changing a
/// colour scale from text to fill.
#[test]
fn conditional_formatting_revalidates_a_normalized_wide_csv_without_panicking() {
    let directory = temporary_test_directory("conditional-formatting-wide-csv");
    let source = directory.join("rows.csv");
    let mut contents = String::from("Line,A,B,C,D,E,F,G\n");
    for line in 1..=38 {
        contents.push_str(&format!("{line},a,b,c,d,e,f,g\n"));
    }
    fs::write(&source, contents).unwrap();

    let mut store = Store::new(Document::blank("Wide CSV"));
    store
        .apply(Operation::ImportFrameFromFile {
            name: "Rows".into(),
            path: source.display().to_string(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Rows").clone();
    let line_id = frame.columns[0].id.clone();

    store
        .apply(Operation::SetFrameStyleRules {
            frame_id: frame.id.clone(),
            rules: vec![FrameStyleRuleInput {
                id: None,
                formula: "`Line`.normalize()".into(),
                column_id: Some(line_id),
                output: FrameStyleOutput::Scale {
                    scale: FrameStyleScale {
                        text: None,
                        fill: Some(FrameStyleColorScale {
                            low: "#000000".into(),
                            high: "#ffffff".into(),
                            mid: None,
                        }),
                    },
                },
            }],
        })
        .unwrap();

    let page = store.get_frame_page(&frame.id, 0, 5).unwrap();
    assert_eq!(page.style_matches.len(), 5);
    assert!(page.style_matches.iter().all(|matches| matches.len() == 1));

    fs::remove_dir_all(directory).ok();
}

/// The ends of an unpinned ramp are the column's, not the page's.
///
/// This is the whole reason a rule's hidden column belongs above the slice:
/// read per page, the same value would be the lightest color on one screen
/// and the darkest on the next, and scrolling would repaint rows that never
/// changed.
#[test]
fn a_normalized_ramp_spans_the_frame_rather_than_the_page() {
    let (directory, mut store, frame_id, score_id) =
        paged_sorted_fixture("conditional-formatting-ramp", 50);

    store
        .apply(Operation::SetFrameStyleRules {
            frame_id: frame_id.clone(),
            rules: vec![FrameStyleRuleInput {
                id: None,
                formula: "`Score`.normalize()".into(),
                column_id: Some(score_id.clone()),
                output: FrameStyleOutput::Scale {
                    scale: FrameStyleScale {
                        text: None,
                        fill: Some(FrameStyleColorScale {
                            low: "#000000".into(),
                            high: "#ffffff".into(),
                            mid: None,
                        }),
                    },
                },
            }],
        })
        .unwrap();

    // Sorted ascending over 1..=50, so the first page holds the lowest ten
    // scores and the last page the highest. Against a frame-wide ramp the
    // first page is black-ish and the last white-ish; against a per-page
    // ramp both would run the full black-to-white spread.
    //
    // The aggregates are inside `.normalize()` now rather than in the plan's
    // own arithmetic, so this is what proves the move did not quietly turn a
    // frame-wide `min`/`max` into a per-page one: the rule's hidden column
    // still goes in above the slice, and the formula's aggregates see every
    // row the way `x > x.mean()` does.
    let fill = |page: &FramePage, row: usize| {
        page.style_matches[row][0]
            .style
            .fill_color
            .clone()
            .expect("the ramp fills every row it answers for")
    };
    let first = store.get_frame_page(&frame_id, 0, 10).unwrap();
    let last = store.get_frame_page(&frame_id, 40, 10).unwrap();

    assert_eq!(fill(&first, 0), "#000000", "the frame's smallest score");
    assert_eq!(fill(&last, 9), "#ffffff", "the frame's largest score");
    // Neither page spans the ramp on its own: ten of fifty rows is roughly a
    // fifth of it, and a per-page ramp would have made each page span it all.
    let spread = |page: &FramePage| {
        let channel =
            |row: usize| u8::from_str_radix(&fill(page, row)[1..3], 16).expect("#rrggbb") as i32;
        channel(9) - channel(0)
    };
    assert!(spread(&first) < 80, "first page spread {}", spread(&first));
    assert!(spread(&last) < 80, "last page spread {}", spread(&last));

    fs::remove_dir_all(directory).ok();
}
