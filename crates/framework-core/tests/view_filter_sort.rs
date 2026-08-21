use crate::common::*;
use framework_core::*;
use std::fs;

/// The names a frame shows, in display order — read through the page reader,
/// which is now the only way rows reach the screen.
fn display_names(store: &Store, frame_id: &str) -> Vec<String> {
    let page = store.get_frame_page(frame_id, 0, 1000).unwrap();
    let name_column = page
        .columns
        .iter()
        .position(|column| column.name == "Name")
        .unwrap();
    page.rows
        .iter()
        .map(|row| row[name_column].clone())
        .collect()
}

fn sort_by(store: &mut Store, frame_id: &str, column_id: &str, descending: bool) {
    store
        .apply(Operation::SetFrameDisplaySort {
            frame_id: frame_id.to_string(),
            keys: vec![DerivedSort {
                column_id: column_id.to_string(),
                descending,
            }],
        })
        .unwrap();
}

#[test]
fn display_filters_must_be_boolean_and_protect_referenced_columns() {
    let mut store = demo_store();
    let frame = frame_named(store.document(), "Orders").clone();
    assert!(matches!(
        store.apply(Operation::SetFrameDisplayFilter {
            frame_id: frame.id.clone(),
            filters: vec!["`Quantity` + 1".into()],
            filter_match_all: true,
        }),
        Err(CoreError::InvalidOperation(_))
    ));

    store
        .apply(Operation::SetFrameDisplayFilter {
            frame_id: frame.id.clone(),
            filters: vec!["`Quantity` > 2".into()],
            filter_match_all: true,
        })
        .unwrap();
    assert!(matches!(
        store.apply(Operation::DeleteColumn {
            frame_id: frame.id,
            column_id: frame.columns[0].id.clone(),
        }),
        Err(CoreError::ReferencedByFormula(_))
    ));
}

fn build_sort_fixture(store: &mut Store) -> FrameObject {
    store
        .apply(Operation::AddFrame {
            name: "Roster".into(),
            grid: vec![
                vec![
                    "Name".into(),
                    "Score".into(),
                    "Active".into(),
                    "Joined".into(),
                ],
                vec![
                    "Dee".into(),
                    "20".into(),
                    "true".into(),
                    "2024-02-05".into(),
                ],
                vec![
                    "Cara".into(),
                    "50".into(),
                    "true".into(),
                    "2024-01-10".into(),
                ],
                vec!["Abe".into(), "".into(), "false".into(), "2024-03-01".into()],
                vec!["Bo".into(), "20".into(), "".into(), "".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    frame_named(store.document(), "Roster").clone()
}

#[test]
fn a_display_sort_orders_by_multiple_keys_with_nulls_last_in_both_directions() {
    let mut store = demo_store();
    let frame = build_sort_fixture(&mut store);
    let score_id = frame.columns[1].id.clone();
    let active_id = frame.columns[2].id.clone();
    let joined_id = frame.columns[3].id.clone();

    sort_by(&mut store, &frame.id, &score_id, false);
    assert_eq!(
        display_names(&store, &frame.id),
        ["Dee", "Bo", "Cara", "Abe"]
    );

    sort_by(&mut store, &frame.id, &score_id, true);
    assert_eq!(
        display_names(&store, &frame.id),
        ["Cara", "Dee", "Bo", "Abe"]
    );

    sort_by(&mut store, &frame.id, &active_id, false);
    assert_eq!(
        display_names(&store, &frame.id),
        ["Abe", "Dee", "Cara", "Bo"]
    );

    sort_by(&mut store, &frame.id, &joined_id, false);
    assert_eq!(
        display_names(&store, &frame.id),
        ["Cara", "Dee", "Abe", "Bo"]
    );

    store
        .apply(Operation::SetFrameDisplaySort {
            frame_id: frame.id.clone(),
            keys: vec![
                DerivedSort {
                    column_id: score_id,
                    descending: false,
                },
                DerivedSort {
                    column_id: frame.columns[0].id.clone(),
                    descending: false,
                },
            ],
        })
        .unwrap();
    assert_eq!(
        display_names(&store, &frame.id),
        ["Bo", "Dee", "Cara", "Abe"]
    );
}

/// The wrangle chain and the display layer are the same evaluator, so a sort
/// written as a pipeline step must place nulls exactly where a display sort
/// does. This is the assertion that would have caught the two conventions
/// the three old sort paths disagreed on.
#[test]
fn a_wrangle_sort_and_a_display_sort_place_nulls_identically() {
    let mut store = demo_store();
    let frame = build_sort_fixture(&mut store);
    let score_id = frame.columns[1].id.clone();

    sort_by(&mut store, &frame.id, &score_id, false);
    let displayed = display_names(&store, &frame.id);

    store
        .apply(Operation::SetFrameDisplaySort {
            frame_id: frame.id.clone(),
            keys: Vec::new(),
        })
        .unwrap();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::Sort {
                keys: vec![SortInput {
                    column_id: score_id,
                    descending: false,
                }],
            }],
        })
        .unwrap();
    assert_eq!(display_names(&store, &frame.id), displayed);
    assert_eq!(displayed, ["Dee", "Bo", "Cara", "Abe"]);
}

#[test]
fn a_display_sort_composes_with_a_display_filter_filtering_before_ordering() {
    let mut store = demo_store();
    let frame = build_sort_fixture(&mut store);
    let score_id = frame.columns[1].id.clone();

    store
        .apply(Operation::SetFrameDisplayFilter {
            frame_id: frame.id.clone(),
            filters: vec!["`Name` != \"Abe\"".into()],
            filter_match_all: true,
        })
        .unwrap();
    sort_by(&mut store, &frame.id, &score_id, true);
    assert_eq!(display_names(&store, &frame.id), ["Cara", "Dee", "Bo"]);
}

#[test]
fn a_display_sort_flows_through_paged_reads_with_correct_page_boundaries() {
    let directory = temporary_test_directory("view-sort-paging");
    let source = directory.join("scores.csv");
    fs::write(&source, "Name,Score\nE,5\nC,3\nA,1\nD,4\nB,2\n").unwrap();

    let mut store = Store::new(Document::blank("Paging"));
    store
        .apply(Operation::ImportFrameFromFile {
            name: "Scores".into(),
            path: source.display().to_string(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Scores").clone();
    assert!(store.view().computed_frames[&frame.id].paged);
    sort_by(&mut store, &frame.id, &frame.columns[1].id.clone(), false);

    let first_page = store.get_frame_page(&frame.id, 0, 2).unwrap();
    assert_eq!(first_page.total_rows, 5);
    assert_eq!(first_page.rows, vec![vec!["A", "1"], vec!["B", "2"]]);
    assert_eq!(
        store.get_frame_page(&frame.id, 2, 2).unwrap().rows,
        vec![vec!["C", "3"], vec!["D", "4"]]
    );
    assert_eq!(
        store.get_frame_page(&frame.id, 4, 2).unwrap().rows,
        vec![vec!["E", "5"]]
    );

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn a_header_sort_round_trips_as_a_trailing_pipeline_step() {
    let mut store = demo_store();
    let frame = build_sort_fixture(&mut store);
    sort_by(&mut store, &frame.id, &frame.columns[1].id.clone(), true);

    let serialized = serde_json::to_string(store.document()).unwrap();
    let restored: Document = serde_json::from_str(&serialized).unwrap();
    let steps = &restored.frame(&frame.id).unwrap().steps;
    assert!(matches!(
        steps.last(),
        Some(FrameStep::Sort { keys }) if keys.len() == 1 && keys[0].descending
    ));

    // A document written before the display layer existed simply has none.
    let legacy = r#"{
        "id": "frame-1",
        "name": "Frame",
        "columns": [],
        "rows": []
    }"#;
    let loaded: FrameObject = serde_json::from_str(legacy).unwrap();
    assert!(loaded.display.is_empty());
}

#[test]
fn a_display_sort_undo_restores_the_prior_order() {
    let mut store = demo_store();
    let frame = build_sort_fixture(&mut store);
    let original_order = display_names(&store, &frame.id);

    sort_by(&mut store, &frame.id, &frame.columns[1].id.clone(), false);
    assert_ne!(display_names(&store, &frame.id), original_order);

    store.undo();
    assert_eq!(display_names(&store, &frame.id), original_order);
}

#[test]
fn a_display_sort_rejects_keys_for_columns_outside_the_frame() {
    let mut store = demo_store();
    let frame = build_sort_fixture(&mut store);
    let foreign_column_id = frame_named(store.document(), "Orders").columns[0]
        .id
        .clone();

    assert!(matches!(
        store.apply(Operation::SetFrameDisplaySort {
            frame_id: frame.id,
            keys: vec![DerivedSort {
                column_id: foreign_column_id,
                descending: false,
            }],
        }),
        Err(CoreError::ColumnNotFound)
    ));
}

#[test]
fn a_filter_predicate_composes_boolean_logic_the_match_mode_cannot() {
    let directory = temporary_test_directory("pipeline-boolean");
    let source = directory.join("rows.csv");
    fs::write(
        &source,
        "Name,Region,Score\nA,West,10\nB,West,90\nC,East,90\nD,East,10\n",
    )
    .unwrap();

    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Pipeline".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::ImportFrameFromFile {
            name: "Rows".into(),
            path: source.display().to_string(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let source_id = frame_named(store.document(), "Rows").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: source_id,
            name: "Picked".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let derived_id = frame_named(store.document(), "Picked").id.clone();

    store
        .apply(Operation::SetFramePipeline {
            frame_id: derived_id.clone(),
            steps: vec![FrameStepInput::Filter {
                predicates: vec![
                    "(`Region` == \"West\" & `Score` < 50) | (`Region` == \"East\" & `Score` > 50)"
                        .into(),
                ],
                match_all: true,
            }],
        })
        .unwrap();

    let page = store.get_frame_page(&derived_id, 0, 10).unwrap();
    assert_eq!(page.total_rows, 2);
    assert_eq!(page.rows[0][0], "A");
    assert_eq!(page.rows[1][0], "C");

    let rendered = store.view().computed_frames[&derived_id].steps.clone();
    let RenderedFrameStep::Filter { predicates, .. } = &rendered[0] else {
        panic!("the chain is a single filter step");
    };
    assert_eq!(
        predicates[0],
        "`Region` == \"West\" & `Score` < 50 | `Region` == \"East\" & `Score` > 50"
    );

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn a_filter_compares_dates_with_the_date_constructor_and_dt_methods() {
    let directory = temporary_test_directory("pipeline-dates");
    let source = directory.join("bookings.csv");
    fs::write(
        &source,
        "Name,Booked\nA,2024-01-15\nB,2024-02-20\nC,2024-03-05\n",
    )
    .unwrap();

    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Pipeline".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::ImportFrameFromFile {
            name: "Bookings".into(),
            path: source.display().to_string(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let source_id = frame_named(store.document(), "Bookings").id.clone();
    assert_eq!(
        frame_named(store.document(), "Bookings").columns[1].data_type,
        DataType::Date,
        "an ISO date column imports as a date, not text"
    );
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: source_id,
            name: "In February".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let derived_id = frame_named(store.document(), "In February").id.clone();

    store
        .apply(Operation::SetFramePipeline {
            frame_id: derived_id.clone(),
            steps: vec![FrameStepInput::Filter {
                predicates: vec!["`Booked` >= date(2024, 2, 1) & `Booked`.dt.month() == 2".into()],
                match_all: true,
            }],
        })
        .unwrap();

    let page = store.get_frame_page(&derived_id, 0, 10).unwrap();
    assert_eq!(page.total_rows, 1);
    assert_eq!(page.rows[0], vec!["B", "2024-02-20"]);

    fs::remove_dir_all(directory).unwrap();
}

/// The requirement the whole unification exists to serve: a filter written
/// in the Wrangle tab is lineage and reaches everything downstream, while a
/// filter written in the View tab is presentation and reaches nothing. Both
/// on the same frame, at the same time, through the same evaluator.
#[test]
fn a_wrangle_filter_propagates_downstream_and_a_display_filter_does_not() {
    let mut store = demo_store();
    let frame = build_sort_fixture(&mut store);

    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: frame.id.clone(),
            name: "Downstream".into(),
            x: 600.0,
            y: 0.0,
        })
        .unwrap();
    let downstream = frame_named(store.document(), "Downstream").id.clone();
    let rows = |store: &Store, id: &str| store.get_frame_page(id, 0, 100).unwrap().total_rows;
    assert_eq!((rows(&store, &frame.id), rows(&store, &downstream)), (4, 4));

    // Wrangle: a step in the frame's own chain. Lineage, so it propagates.
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::Filter {
                predicates: vec!["`Name` != \"Abe\"".into()],
                match_all: true,
            }],
        })
        .unwrap();
    assert_eq!((rows(&store, &frame.id), rows(&store, &downstream)), (3, 3));

    // View: a step in the display layer. Presentation, so it stops here.
    store
        .apply(Operation::SetFrameDisplayFilter {
            frame_id: frame.id.clone(),
            filters: vec!["`Name` != \"Bo\"".into()],
            filter_match_all: true,
        })
        .unwrap();
    assert_eq!(
        (rows(&store, &frame.id), rows(&store, &downstream)),
        (2, 3),
        "the display filter narrows this frame's own reads and nothing else"
    );

    // And a second tab is a second frame, so it filters independently of both.
    let view_id = store
        .document()
        .views
        .iter()
        .find(|view| view.object_id == frame.id)
        .unwrap()
        .id
        .clone();
    store
        .apply(Operation::BranchFrame {
            view_id,
            frame_id: frame.id.clone(),
        })
        .unwrap();
    let branch = frame_named(store.document(), "Roster copy").id.clone();
    assert_eq!(
        (rows(&store, &frame.id), rows(&store, &branch)),
        (2, 3),
        "a branch inherits the wrangle chain but starts with no display layer"
    );
}
