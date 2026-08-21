use crate::common::*;
use framework_core::*;
use std::fs;

#[test]
fn a_pipeline_shift_requires_a_sort_in_its_resulting_lineage() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Ordered formulas".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Rows".into(),
            grid: vec![vec!["Value".into()], vec!["2".into()], vec!["1".into()]],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Rows").clone();
    let shifted = || FrameStepInput::WithColumns {
        columns: vec![ExistingFormulaInput {
            output_column_id: uuid::Uuid::new_v4().to_string(),
            name: "Previous".into(),
            formula: "`Value`.shift(1)".into(),
        }],
    };

    let error = store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![shifted()],
        })
        .unwrap_err();
    assert!(error.to_string().contains("declared row ordering"));

    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id,
            steps: vec![
                FrameStepInput::Sort {
                    keys: vec![SortInput {
                        column_id: frame.columns[0].id.clone(),
                        descending: false,
                    }],
                },
                shifted(),
            ],
        })
        .unwrap();
}

#[test]
fn a_running_calculation_requires_and_uses_declared_order() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Running total".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Rows".into(),
            grid: vec![
                vec!["Value".into()],
                vec!["3".into()],
                vec!["1".into()],
                vec!["2".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Rows").clone();
    let output_id = framework_core::id();
    let running = || FrameStepInput::WithColumns {
        columns: vec![ExistingFormulaInput {
            output_column_id: output_id.clone(),
            name: "Running".into(),
            formula: "`Value`.cum_sum(False)".into(),
        }],
    };

    let error = store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![running()],
        })
        .unwrap_err();
    assert!(error.to_string().contains("running calculation"));

    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![
                FrameStepInput::Sort {
                    keys: vec![SortInput {
                        column_id: frame.columns[0].id.clone(),
                        descending: false,
                    }],
                },
                running(),
            ],
        })
        .unwrap();
    let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
    let running_index = page
        .columns
        .iter()
        .position(|column| column.id == output_id)
        .unwrap();
    assert_eq!(
        page.rows
            .iter()
            .map(|row| row[running_index].as_str())
            .collect::<Vec<_>>(),
        ["1", "3", "6"]
    );
}

#[test]
fn an_ordered_recurrence_reads_its_previous_result_and_restarts_by_group() {
    let mut store = Store::new(Document {
        id: framework_core::id(),
        name: "Recurrence".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Ledger".into(),
            grid: vec![
                vec!["Order".into(), "Account".into(), "Change".into()],
                vec!["3".into(), "A".into(), "2".into()],
                vec!["1".into(), "A".into(), "5".into()],
                vec!["4".into(), "B".into(), "-3".into()],
                vec!["2".into(), "B".into(), "10".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Ledger").clone();
    let order = frame.columns[0].id.clone();
    let output_id = framework_core::id();
    let recurrence = || FrameStepInput::WithColumns {
        columns: vec![ExistingFormulaInput {
            output_column_id: output_id.clone(),
            name: "Balance".into(),
            formula: "recur(`Change`, previous() + `Change`, restart_by=[`Account`])".into(),
        }],
    };

    let error = store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![recurrence()],
        })
        .unwrap_err();
    assert!(error.to_string().contains("Calculate down rows"), "{error}");

    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![
                FrameStepInput::Sort {
                    keys: vec![SortInput {
                        column_id: order,
                        descending: false,
                    }],
                },
                recurrence(),
            ],
        })
        .unwrap();

    let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
    let balance = page
        .columns
        .iter()
        .position(|column| column.id == output_id)
        .unwrap();
    assert_eq!(
        page.rows
            .iter()
            .map(|row| row[balance].as_str())
            .collect::<Vec<_>>(),
        ["5", "10", "7", "7"]
    );
    let rendered = &store.view().computed_frames[&frame.id].steps[1];
    let RenderedFrameStep::WithColumns { columns } = rendered else {
        panic!("recurrence should remain an editable calculated-column step")
    };
    assert_eq!(
        columns[0].formula,
        "recur(`Change`, previous() + `Change`, restart_by=[`Account`])"
    );
}

#[test]
fn recurrence_requires_a_seed_and_a_previous_result() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").clone();
    let order = orders.columns[0].id.clone();
    let formula = |formula: &str| FrameStepInput::WithColumns {
        columns: vec![ExistingFormulaInput {
            output_column_id: framework_core::id(),
            name: "State".into(),
            formula: formula.into(),
        }],
    };
    for (source, expected) in [
        ("recur(previous(), 1)", "first row has no previous result"),
        ("recur(0, `Quantity`)", "needs previous()"),
    ] {
        let error = store
            .apply(Operation::SetFramePipeline {
                frame_id: orders.id.clone(),
                steps: vec![
                    FrameStepInput::Sort {
                        keys: vec![SortInput {
                            column_id: order.clone(),
                            descending: false,
                        }],
                    },
                    formula(source),
                ],
            })
            .unwrap_err();
        assert!(error.to_string().to_lowercase().contains(expected));
    }

    let error = store
        .apply(Operation::SetFramePipeline {
            frame_id: orders.id.clone(),
            steps: vec![
                FrameStepInput::Sort {
                    keys: vec![SortInput {
                        column_id: order,
                        descending: false,
                    }],
                },
                FrameStepInput::WithColumns {
                    columns: vec![
                        ExistingFormulaInput {
                            output_column_id: framework_core::id(),
                            name: "State".into(),
                            formula: "recur(0, previous() + 1)".into(),
                        },
                        ExistingFormulaInput {
                            output_column_id: framework_core::id(),
                            name: "Other".into(),
                            formula: "1".into(),
                        },
                    ],
                },
            ],
        })
        .unwrap_err();
    assert!(
        error.to_string().contains("its own Wrangle step"),
        "{error}"
    );
}

#[test]
fn a_frame_length_sequence_fills_the_rows_after_a_declared_sort() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Row numbers".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Rows".into(),
            grid: vec![
                vec!["Value".into()],
                vec!["30".into()],
                vec!["10".into()],
                vec!["20".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Rows").clone();
    let sequence_id = framework_core::id();
    let sequence_step = || FrameStepInput::WithColumns {
        columns: vec![ExistingFormulaInput {
            output_column_id: sequence_id.clone(),
            name: "Row number".into(),
            formula: "sequence(1, frame.len() + 1)".into(),
        }],
    };

    let error = store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![sequence_step()],
        })
        .unwrap_err();
    assert!(error.to_string().contains("declared row ordering"));

    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![
                FrameStepInput::Sort {
                    keys: vec![SortInput {
                        column_id: frame.columns[0].id.clone(),
                        descending: false,
                    }],
                },
                sequence_step(),
            ],
        })
        .unwrap();

    let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
    let sequence_index = page
        .columns
        .iter()
        .position(|column| column.id == sequence_id)
        .unwrap();
    assert_eq!(
        page.rows
            .iter()
            .map(|row| row[sequence_index].as_str())
            .collect::<Vec<_>>(),
        ["1", "2", "3"]
    );
}

#[test]
fn a_frame_length_date_sequence_fills_calendar_months() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Monthly dates".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Rows".into(),
            grid: vec![
                vec!["Date".into()],
                vec!["2026-01-31".into()],
                vec!["2026-02-28".into()],
                vec!["2026-03-31".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Rows").clone();
    let date_id = frame.columns[0].id.clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![
                FrameStepInput::Sort {
                    keys: vec![SortInput {
                        column_id: date_id.clone(),
                        descending: false,
                    }],
                },
                FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: date_id,
                        name: "Date".into(),
                        formula: "sequence(2026-01-31, periods=frame.n_rows(), step=1mo)".into(),
                    }],
                },
            ],
        })
        .unwrap();

    let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
    assert_eq!(
        page.rows
            .iter()
            .map(|row| row[0].as_str())
            .collect::<Vec<_>>(),
        ["2026-01-31", "2026-02-28", "2026-03-31"]
    );
}

#[test]
fn a_typed_null_pipeline_column_is_visible_and_numeric() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").clone();
    let blank_id = framework_core::id();

    store
        .apply(Operation::SetFramePipeline {
            frame_id: orders.id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: blank_id.clone(),
                    name: "Column 1".into(),
                    formula: "null.cast(\"number\")".into(),
                }],
            }],
        })
        .unwrap();

    let frame = store.document().frame(&orders.id).unwrap();
    let blank = frame
        .columns
        .iter()
        .find(|column| column.id == blank_id)
        .unwrap();
    assert_eq!(blank.data_type, DataType::Number);
    let page = store.get_frame_page(&orders.id, 0, 10).unwrap();
    assert!(
        page.rows
            .iter()
            .all(|row| row.last().is_some_and(String::is_empty))
    );
}

#[test]
fn with_columns_can_transform_existing_columns_in_place() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "In-place columns".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Input".into(),
            grid: vec![
                vec!["Code".into(), "Amount".into()],
                vec!["east".into(), "1.5".into()],
                vec!["west".into(), "2.0".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let input = frame_named(store.document(), "Input").clone();
    let code = input.columns[0].clone();
    let amount = input.columns[1].clone();

    store
        .apply(Operation::SetFramePipeline {
            frame_id: input.id.clone(),
            steps: vec![
                FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: code.id.clone(),
                        name: code.name.clone(),
                        formula: "`Code`.str.to_uppercase()".into(),
                    }],
                },
                FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: amount.id.clone(),
                        name: amount.name.clone(),
                        formula: "`Amount`.cast(\"integer\")".into(),
                    }],
                },
            ],
        })
        .unwrap();

    let transformed = store.document().frame(&input.id).unwrap();
    assert_eq!(transformed.columns.len(), 2);
    assert_eq!(transformed.columns[0].id, code.id);
    assert_eq!(transformed.columns[1].id, amount.id);
    assert_eq!(transformed.columns[1].data_type, DataType::Integer);
    let page = store.get_frame_page(&input.id, 0, 10).unwrap();
    assert_eq!(page.rows[0], vec!["EAST", "1"]);
    assert_eq!(page.rows[1], vec!["WEST", "2"]);
}

#[test]
fn replacing_a_pipeline_replaces_its_trailing_header_sort() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Display sort reconciliation".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Rows".into(),
            grid: vec![
                vec!["Value".into(), "Keep".into()],
                vec!["2".into(), "B".into()],
                vec!["1".into(), "A".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Rows").clone();
    let value_id = frame.columns[0].id.clone();
    let keep_id = frame.columns[1].id.clone();
    store
        .apply(Operation::SetFrameDisplaySort {
            frame_id: frame.id.clone(),
            keys: vec![
                DerivedSort {
                    column_id: value_id,
                    descending: false,
                },
                DerivedSort {
                    column_id: keep_id.clone(),
                    descending: true,
                },
            ],
        })
        .unwrap();

    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::Select {
                column_ids: vec![keep_id.clone()],
            }],
        })
        .unwrap();

    let frame = store.document().frame(&frame.id).unwrap();
    assert!(frame.display.sort().is_empty());
    assert!(matches!(
        frame.steps.as_slice(),
        [FrameStep::Select { column_ids }] if column_ids == &[keep_id]
    ));
    assert!(store.get_frame_page(&frame.id, 0, 10).is_ok());
}

#[test]
fn an_ordered_unique_summary_reads_after_replacing_a_trailing_header_sort() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Unique summary".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Rows".into(),
            grid: vec![
                vec!["Journal entry".into()],
                vec!["JE-2".into()],
                vec!["JE-1".into()],
                vec!["JE-2".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Rows").clone();
    let source_id = frame.columns[0].id.clone();
    store
        .apply(Operation::SetFrameDisplaySort {
            frame_id: frame.id.clone(),
            keys: vec![DerivedSort {
                column_id: source_id,
                descending: false,
            }],
        })
        .unwrap();

    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::Summarize {
                group_keys: Vec::new(),
                aggregates: vec![ExistingFormulaInput {
                    output_column_id: framework_core::id(),
                    name: "unique_ids".into(),
                    formula: "`Journal entry`.unique()".into(),
                }],
                maintain_order: true,
            }],
        })
        .unwrap();

    let frame = store.document().frame(&frame.id).unwrap();
    assert!(frame.display.sort().is_empty());
    assert!(
        store.get_frame_page(&frame.id, 0, 10).is_ok(),
        "the display layer must not sort the one-column summary by its removed source column"
    );
}

#[test]
fn linked_frames_filter_project_sort_and_feed_aggregates() {
    let mut store = Store::new(Document::demo());
    let source = store
        .view()
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == "Orders" => Some(frame.clone()),
            _ => None,
        })
        .unwrap();

    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: source.id.clone(),
            name: "Large orders".into(),
            x: 500.0,
            y: 500.0,
        })
        .unwrap();
    let linked = store
        .view()
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == "Large orders" => Some(frame.clone()),
            _ => None,
        })
        .unwrap();
    let rendered = store.view().computed_frames[&linked.id].steps.clone();
    let RenderedFrameStep::WithColumns { columns } = &rendered[0] else {
        panic!("a linked frame's chain opens with the projection that mints its column ids");
    };
    let projections = columns
        .iter()
        .map(|projection| {
            let column = linked
                .columns
                .iter()
                .find(|column| column.id == projection.output_column_id)
                .unwrap();
            ExistingFormulaInput {
                output_column_id: column.id.clone(),
                name: column.name.clone(),
                formula: projection.formula.clone(),
            }
        })
        .collect::<Vec<_>>();
    let total = linked
        .columns
        .iter()
        .find(|column| column.name == "Total")
        .unwrap()
        .id
        .clone();
    let projected_ids = projections
        .iter()
        .map(|projection| projection.output_column_id.clone())
        .collect::<Vec<_>>();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: linked.id.clone(),
            steps: vec![
                FrameStepInput::Filter {
                    predicates: vec!["`Total` > 50".into(), "`Quantity` == 3".into()],
                    match_all: false,
                },
                FrameStepInput::WithColumns {
                    columns: projections,
                },
                FrameStepInput::Select {
                    column_ids: projected_ids,
                },
                FrameStepInput::Sort {
                    keys: vec![SortInput {
                        column_id: total.clone(),
                        descending: true,
                    }],
                },
            ],
        })
        .unwrap();

    let filtered = store.view();
    let linked = filtered
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == "Large orders" => Some(frame),
            _ => None,
        })
        .unwrap();
    assert_eq!(linked.rows.len(), 2);
    assert_eq!(
        filtered.computed_frames[&linked.id].rows[&linked.rows[0].id][&total].value,
        Some(58.800000000000004)
    );
    assert_eq!(
        filtered.computed_frames[&linked.id].rows[&linked.rows[1].id][&total].value,
        Some(44.1)
    );

    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: linked.id.clone(),
            name: "Large order total".into(),
            group_keys: Vec::new(),
            aggregates: vec![NamedFormulaInput {
                name: "Total".into(),
                formula: "`Total`.sum()".into(),
            }],
            maintain_order: true,
            x: 900.0,
            y: 500.0,
        })
        .unwrap();
    let result = store.view();
    let aggregate = result
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == "Large order total" => Some(frame),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        result.computed_frames[&aggregate.id].rows[&aggregate.rows[0].id][&aggregate.columns[0].id]
            .value,
        Some(102.9)
    );
}

#[test]
fn derived_aggregates_refresh_branch_and_compose() {
    let mut store = Store::new(Document::demo());
    store
        .apply(Operation::AddFrame {
            name: "Sales".into(),
            grid: vec![
                vec!["Category".into(), "Amount".into()],
                vec!["Food".into(), "40".into()],
                vec!["Fuel".into(), "70".into()],
                vec!["Food".into(), "25".into()],
            ],
            x: 100.0,
            y: 500.0,
        })
        .unwrap();
    let view = store.view();
    let sales = view
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == "Sales" => Some(frame.clone()),
            _ => None,
        })
        .unwrap();

    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: sales.id.clone(),
            name: "Sales by category".into(),
            group_keys: vec![NamedFormulaInput {
                name: "Category".into(),
                formula: "`Category`".into(),
            }],
            aggregates: vec![NamedFormulaInput {
                name: "Revenue".into(),
                formula: "`Amount`.sum()".into(),
            }],
            maintain_order: true,
            x: 700.0,
            y: 500.0,
        })
        .unwrap();
    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: sales.id.clone(),
            name: "Sales total".into(),
            group_keys: vec![],
            aggregates: vec![NamedFormulaInput {
                name: "Revenue".into(),
                formula: "`Amount`.sum()".into(),
            }],
            maintain_order: true,
            x: 700.0,
            y: 820.0,
        })
        .unwrap();

    let view = store.view();
    let grouped = view
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == "Sales by category" => Some(frame),
            _ => None,
        })
        .unwrap();
    assert_eq!(grouped.rows.len(), 2);
    let revenue = grouped
        .columns
        .iter()
        .find(|column| column.name == "Revenue")
        .unwrap();
    assert_eq!(
        view.computed_frames[&grouped.id].rows[&grouped.rows[0].id][&revenue.id].value,
        Some(65.0)
    );
    let grouped_id = grouped.id.clone();
    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: grouped_id,
            name: "Grouped grand total".into(),
            group_keys: vec![],
            aggregates: vec![NamedFormulaInput {
                name: "Revenue".into(),
                formula: "`Revenue`.sum()".into(),
            }],
            maintain_order: true,
            x: 1200.0,
            y: 500.0,
        })
        .unwrap();
    let composed = store.view();
    let grand_total = composed
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == "Grouped grand total" => Some(frame),
            _ => None,
        })
        .unwrap();
    let revenue = &grand_total.columns[0];
    assert_eq!(
        composed.computed_frames[&grand_total.id].rows[&grand_total.rows[0].id][&revenue.id].value,
        Some(135.0)
    );

    let amount = sales
        .columns
        .iter()
        .find(|column| column.name == "Amount")
        .unwrap();
    store
        .apply(Operation::SetCell {
            frame_id: sales.id.clone(),
            row_id: sales.rows[0].id.clone(),
            column_id: amount.id.clone(),
            raw: "50".into(),
        })
        .unwrap();
    let refreshed = store.view();
    let grouped = refreshed
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == "Sales by category" => Some(frame),
            _ => None,
        })
        .unwrap();
    let revenue = grouped
        .columns
        .iter()
        .find(|column| column.name == "Revenue")
        .unwrap();
    assert_eq!(
        refreshed.computed_frames[&grouped.id].rows[&grouped.rows[0].id][&revenue.id].value,
        Some(75.0)
    );
    let grand_total = refreshed
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == "Grouped grand total" => Some(frame),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        refreshed.computed_frames[&grand_total.id].rows[&grand_total.rows[0].id]
            [&grand_total.columns[0].id]
            .value,
        Some(145.0)
    );

    let grouped_view = refreshed
        .document
        .views
        .iter()
        .find(|canvas_view| canvas_view.object_id == grouped.id)
        .unwrap();
    store
        .apply(Operation::SetViewCollapsed {
            view_id: grouped_view.id.clone(),
            collapsed: true,
        })
        .unwrap();
    assert!(
        store
            .view()
            .document
            .views
            .iter()
            .find(|view| view.id == grouped_view.id)
            .unwrap()
            .collapsed
    );
}

#[test]
fn derived_transformation_updates_preserve_existing_output_identity() {
    let mut store = Store::new(Document::demo());
    let source = store
        .view()
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == "Orders" => Some(frame.clone()),
            _ => None,
        })
        .unwrap();
    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: source.id,
            name: "Order total".into(),
            group_keys: vec![],
            aggregates: vec![NamedFormulaInput {
                name: "Quantity sum".into(),
                formula: "`Quantity`.sum()".into(),
            }],
            maintain_order: true,
            x: 400.0,
            y: 500.0,
        })
        .unwrap();
    let initial = store.view();
    let result = initial
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == "Order total" => Some(frame),
            _ => None,
        })
        .unwrap();
    let aggregate_id = result.columns[0].id.clone();

    store
        .apply(Operation::SetFramePipeline {
            frame_id: result.id.clone(),
            steps: vec![FrameStepInput::Summarize {
                group_keys: vec![ExistingFormulaInput {
                    output_column_id: uuid::Uuid::new_v4().to_string(),
                    name: "Quantity".into(),
                    formula: "`Quantity`".into(),
                }],
                aggregates: vec![ExistingFormulaInput {
                    output_column_id: aggregate_id.clone(),
                    name: "Quantity sum".into(),
                    formula: "`Quantity`.sum()".into(),
                }],
                maintain_order: true,
            }],
        })
        .unwrap();
    store
        .apply(Operation::RenameObject {
            object_id: result.id.clone(),
            name: "Orders by quantity".into(),
        })
        .unwrap();

    let updated = store.view();
    let result = updated
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == "Orders by quantity" => Some(frame),
            _ => None,
        })
        .unwrap();
    assert_eq!(result.rows.len(), 3);
    assert_eq!(result.columns[1].id, aggregate_id);
    let steps = &updated.computed_frames[&result.id].steps;
    let RenderedFrameStep::Summarize { group_keys, .. } = &steps[0] else {
        panic!("the chain is a single summarize step");
    };
    assert_eq!(group_keys[0].formula, "`Quantity`");
}

#[test]
fn promoting_a_display_layer_moves_its_filter_and_sort_into_the_wrangle_chain() {
    let mut store = demo_store();
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
    let frame = frame_named(store.document(), "Roster").clone();
    let score_id = frame.columns[1].id.clone();
    let name_id = frame.columns[0].id.clone();

    store
        .apply(Operation::SetFrameDisplayFilter {
            frame_id: frame.id.clone(),
            filters: vec!["`Name` != \"Abe\"".into()],
            filter_match_all: true,
        })
        .unwrap();
    store
        .apply(Operation::SetFrameDisplaySort {
            frame_id: frame.id.clone(),
            keys: vec![DerivedSort {
                column_id: score_id.clone(),
                descending: true,
            }],
        })
        .unwrap();

    // While they are still display steps a frame derived from Roster does
    // not see them.
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: frame.id.clone(),
            name: "Downstream".into(),
            x: 600.0,
            y: 0.0,
        })
        .unwrap();
    let downstream = frame_named(store.document(), "Downstream").clone();
    assert_eq!(
        store
            .get_frame_page(&downstream.id, 0, 100)
            .unwrap()
            .total_rows,
        4,
        "a display filter does not reach the frame derived from it"
    );

    store
        .apply(Operation::PromoteDisplayToSteps {
            frame_id: frame.id.clone(),
        })
        .unwrap();

    let promoted = frame_named(store.document(), "Roster").clone();
    assert!(
        promoted.display.steps.is_empty(),
        "promotion empties the display layer"
    );
    assert_eq!(promoted.steps.len(), 2);
    assert!(
        matches!(&promoted.steps[0], FrameStep::Sort { keys } if keys[0].column_id == score_id
            && keys[0].descending)
    );
    assert!(matches!(promoted.steps[1], FrameStep::Filter { .. }));

    let page = store.get_frame_page(&frame.id, 0, 100).unwrap();
    let names = page
        .rows
        .iter()
        .map(|row| row[page.columns.iter().position(|c| c.id == name_id).unwrap()].clone())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["Cara", "Dee", "Bo"]);
    assert_eq!(
        store
            .get_frame_page(&downstream.id, 0, 100)
            .unwrap()
            .total_rows,
        3,
        "once promoted, the filter is lineage and the derived frame sees it"
    );
}

#[test]
fn materializing_a_grouped_frame_caches_it_and_reports_staleness() {
    let directory = temporary_test_directory("materialize-grouped");
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
    let artifact = create_data_artifact(&source, &directory.join("data")).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Ledger".into(),
            artifact,
            connector: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let ledger_id = frame_named(store.document(), "Ledger").id.clone();
    let period_id = frame_named(store.document(), "Ledger").columns[0]
        .id
        .clone();
    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: ledger_id.clone(),
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

    assert_eq!(store.view().computed_frames[&grouped_id].total_rows, None);

    store
        .materialize_frame(&grouped_id, &directory.join("data"))
        .unwrap();

    let view = store.view();
    let computed = &view.computed_frames[&grouped_id];
    assert_eq!(
        computed.total_rows,
        Some(2),
        "a snapshot knows its own size"
    );
    let materialization = computed
        .materialization
        .as_ref()
        .expect("the frame reports that it is cached");
    assert!(!materialization.stale, "a fresh snapshot is not stale");
    assert_eq!(materialization.row_count, 2);
    assert!(
        frame_named(&view.document, "By period")
            .derivation
            .is_some(),
        "the frame stays derived, so it can be refreshed or set live again"
    );

    let page = store.get_frame_page(&grouped_id, 0, 10).unwrap();
    assert_eq!(page.total_rows, 2);
    assert_eq!(page.rows[0][0], "2024-01");
    assert_eq!(page.rows[0][1].parse::<f64>().unwrap(), 105.0);

    store
        .apply(Operation::AddComputedColumn {
            frame_id: ledger_id,
            name: "Doubled".into(),
            formula: "`Debit` * 2".into(),
            after_column_id: None,
        })
        .unwrap();
    assert!(
        store.view().computed_frames[&grouped_id]
            .materialization
            .as_ref()
            .unwrap()
            .stale,
        "an upstream change must mark the snapshot stale"
    );
    assert_eq!(
        store.get_frame_page(&grouped_id, 0, 10).unwrap().rows[0][0],
        "2024-01",
        "a stale snapshot keeps serving until it is refreshed"
    );

    store
        .materialize_frame(&grouped_id, &directory.join("data"))
        .unwrap();
    assert!(
        !store.view().computed_frames[&grouped_id]
            .materialization
            .as_ref()
            .unwrap()
            .stale
    );

    store
        .apply(Operation::ClearFrameMaterialization {
            frame_id: grouped_id.clone(),
        })
        .unwrap();
    let computed = &store.view().computed_frames[&grouped_id];
    assert!(computed.materialization.is_none());
    assert_eq!(computed.total_rows, None);
    assert_eq!(
        store.get_frame_page(&grouped_id, 0, 10).unwrap().total_rows,
        2,
        "reading live still works after uncaching"
    );
    let _ = period_id;

    fs::remove_dir_all(directory).unwrap();
}

/// Authoring the chain by formula: names are resolved against the schema
/// at each step's own position, so `\`Doubled\`` is writable in step 2
/// only because step 1 created it. Several steps of the same kind, in
/// any order, is the whole point.
#[test]
fn set_frame_pipeline_resolves_names_against_each_step() {
    let directory = temporary_test_directory("pipeline-op");
    let source = directory.join("rows.csv");
    fs::write(&source, "Name,Score\nA,10\nB,20\nC,30\n").unwrap();

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
            name: "Chained".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let derived_id = frame_named(store.document(), "Chained").id.clone();

    let doubled = uuid::Uuid::new_v4().to_string();
    let stepped = uuid::Uuid::new_v4().to_string();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: derived_id.clone(),
            steps: vec![
                FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: doubled.clone(),
                        name: "Doubled".into(),
                        formula: "`Score` * 2".into(),
                    }],
                },
                // Reads the column the step above named.
                FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: stepped.clone(),
                        name: "Stepped".into(),
                        formula: "`Doubled` * 10".into(),
                    }],
                },
                FrameStepInput::Filter {
                    predicates: vec!["`Stepped` > 250".into()],
                    match_all: true,
                },
                FrameStepInput::Sort {
                    keys: vec![SortInput {
                        column_id: stepped.clone(),
                        descending: true,
                    }],
                },
            ],
        })
        .unwrap();

    let derived = frame_named(store.document(), "Chained");
    let names = derived
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["Name", "Score", "Doubled", "Stepped"]);

    let page = store.get_frame_page(&derived_id, 0, 10).unwrap();
    assert_eq!(page.total_rows, 2);
    assert_eq!(page.rows[0], vec!["C", "30", "60", "600"]);
    assert_eq!(page.rows[1], vec!["B", "20", "40", "400"]);

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn set_frame_pipeline_rejects_a_sort_on_a_column_no_step_produces() {
    let directory = temporary_test_directory("pipeline-op-unknown");
    let source = directory.join("rows.csv");
    fs::write(&source, "Name,Score\nA,10\n").unwrap();

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
            name: "Chained".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let derived_id = frame_named(store.document(), "Chained").id.clone();

    let result = store.apply(Operation::SetFramePipeline {
        frame_id: derived_id,
        steps: vec![FrameStepInput::Sort {
            keys: vec![SortInput {
                column_id: "nonexistent".into(),
                descending: false,
            }],
        }],
    });
    assert!(matches!(
        result,
        Err(CoreError::InvalidOperation(message)) if message.contains("nothing before it produces")
    ));

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn steps_run_in_order_so_later_steps_see_earlier_columns() {
    let directory = temporary_test_directory("ordered-steps");
    let source = directory.join("rows.csv");
    fs::write(&source, "Name,Score\nA,10\nB,20\nC,30\n").unwrap();

    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Steps".into(),
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
    let frame = frame_named(store.document(), "Rows").clone();
    let score_id = frame.columns[1].id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: frame.id.clone(),
            name: "Chained".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let derived_id = frame_named(store.document(), "Chained").id.clone();

    let doubled = uuid::Uuid::new_v4().to_string();
    let stepped = uuid::Uuid::new_v4().to_string();

    let steps = vec![
        FrameStepInput::WithColumns {
            columns: vec![ExistingFormulaInput {
                output_column_id: doubled.clone(),
                name: "Doubled".into(),
                formula: "`Score` * 2".into(),
            }],
        },
        FrameStepInput::WithColumns {
            columns: vec![ExistingFormulaInput {
                output_column_id: stepped.clone(),
                name: "Stepped".into(),
                formula: "`Doubled` * 10".into(),
            }],
        },
        FrameStepInput::Filter {
            predicates: vec!["`Stepped` > 250".into()],
            match_all: true,
        },
        FrameStepInput::Select {
            column_ids: vec![score_id.clone(), doubled.clone(), stepped.clone()],
        },
    ];

    store
        .apply(Operation::SetFramePipeline {
            frame_id: derived_id.clone(),
            steps,
        })
        .unwrap();

    let page = store.get_frame_page(&derived_id, 0, 10).unwrap();
    assert_eq!(page.total_rows, 2);
    assert_eq!(page.rows[0], vec!["20", "40", "400"]);
    assert_eq!(page.rows[1], vec!["30", "60", "600"]);

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn derived_frames_export_materialized_values_to_csv() {
    let directory = temporary_test_directory("derived-export");
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Export".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Sales".into(),
            grid: vec![
                vec!["Category".into(), "Amount".into()],
                vec!["Food".into(), "40".into()],
                vec!["Fuel".into(), "70".into()],
                vec!["Food".into(), "25".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let source_id = frame_named(store.document(), "Sales").id.clone();
    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: source_id,
            name: "Sales by category".into(),
            group_keys: vec![NamedFormulaInput {
                name: "Category".into(),
                formula: "`Category`".into(),
            }],
            aggregates: vec![NamedFormulaInput {
                name: "Revenue".into(),
                formula: "`Amount`.sum()".into(),
            }],
            maintain_order: true,
            x: 600.0,
            y: 0.0,
        })
        .unwrap();
    let derived_id = frame_named(store.document(), "Sales by category")
        .id
        .clone();

    let exported = directory.join("sales-by-category.csv");
    store.export_frame_csv(&derived_id, &exported).unwrap();
    assert_eq!(
        fs::read_to_string(&exported).unwrap(),
        "Category,Revenue\nFood,65\nFuel,70\n"
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn plots_persist_specs_and_protect_their_source_frame() {
    let mut store = Store::new(Document::demo());
    let frame_id = store
        .view()
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) => Some(frame.id.clone()),
            _ => None,
        })
        .unwrap();
    let initial_spec = serde_json::json!({
        "mark": "bar",
        "encoding": { "x": { "field": "quantity" } }
    });
    store
        .apply(Operation::AddPlot {
            name: "Orders plot".into(),
            source_frame_id: frame_id.clone(),
            spec: initial_spec.clone(),
            x: 900.0,
            y: 100.0,
            view_id: None,
        })
        .unwrap();
    let plot = store
        .view()
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Plot(plot) => Some(plot.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(plot.source_frame_id, frame_id);
    assert_eq!(plot.spec, initial_spec);

    let updated_spec = serde_json::json!({ "mark": { "type": "line", "tooltip": true } });
    store
        .apply(Operation::SetPlotSpec {
            plot_id: plot.id.clone(),
            spec: updated_spec.clone(),
        })
        .unwrap();
    let updated = store.view();
    assert!(updated.document.objects.iter().any(|object| {
        matches!(object, DataObject::Plot(candidate) if candidate.id == plot.id && candidate.spec == updated_spec)
    }));

    assert!(matches!(
        store.apply(Operation::DeleteObject {
            object_id: frame_id
        }),
        Err(CoreError::ReferencedByFormula(_))
    ));
    assert!(store
        .undo()
        .document
        .objects
        .iter()
        .any(|object| matches!(object, DataObject::Plot(candidate) if candidate.id == plot.id && candidate.spec == initial_spec)));
}

#[test]
fn a_source_frame_filters_and_summarizes_through_its_own_chain() {
    let directory = temporary_test_directory("source-chain");
    let source = directory.join("ledger.csv");
    fs::write(
        &source,
        "Period,Region,Debit\nQ1,West,100\nQ1,East,20\nQ2,West,5\nQ2,West,7\n",
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
    let frame_id = frame_named(store.document(), "Ledger").id.clone();

    let period_out = uuid::Uuid::new_v4().to_string();
    let total_out = uuid::Uuid::new_v4().to_string();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![
                FrameStepInput::Filter {
                    predicates: vec!["`Region` == \"West\"".into()],
                    match_all: true,
                },
                FrameStepInput::Summarize {
                    group_keys: vec![ExistingFormulaInput {
                        output_column_id: period_out.clone(),
                        name: "Period".into(),
                        formula: "`Period`".into(),
                    }],
                    aggregates: vec![ExistingFormulaInput {
                        output_column_id: total_out.clone(),
                        name: "Debit total".into(),
                        formula: "`Debit`.sum()".into(),
                    }],
                    maintain_order: true,
                },
            ],
        })
        .unwrap();

    let frame = frame_named(store.document(), "Ledger");
    assert!(
        frame.derivation.is_none(),
        "the frame is still the import itself, not a derivation of one"
    );
    assert_eq!(
        frame
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Period", "Debit total"],
        "what the frame shows is what the chain produces"
    );
    assert_eq!(
        frame
            .base_columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Period", "Region", "Debit"],
        "the file's own schema is remembered, or the base scan could not resolve"
    );

    let page = store.get_frame_page(&frame_id, 0, 10).unwrap();
    assert_eq!(page.total_rows, 2);
    assert_eq!(page.rows[0], vec!["Q1", "100"]);
    assert_eq!(page.rows[1], vec!["Q2", "12"]);

    let rendered = store.view().computed_frames[&frame_id].steps.clone();
    assert_eq!(rendered.len(), 2);
    let RenderedFrameStep::Summarize { aggregates, .. } = &rendered[1] else {
        panic!("the second step is the summarize");
    };
    assert_eq!(aggregates[0].formula, "`Debit`.sum()");

    assert!(matches!(
        store.apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Doubled".into(),
            formula: "`Debit total` * 2".into(),
            after_column_id: None,
        }),
        Err(CoreError::DerivedFrameReadOnly)
    ));

    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: Vec::new(),
        })
        .unwrap();
    let frame = frame_named(store.document(), "Ledger");
    assert_eq!(
        frame
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Period", "Region", "Debit"]
    );
    assert!(frame.base_columns.is_empty(), "nothing left to keep apart");
    assert_eq!(
        store.get_frame_page(&frame_id, 0, 10).unwrap().total_rows,
        4
    );

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn demo_document_is_a_ready_to_join_relational_playground() {
    let mut store = Store::new(Document::demo());
    let transactions = frame_named(store.document(), "Transactions");
    let customers = frame_named(store.document(), "Customers");
    let transaction_id = transactions.id.clone();
    let customer_frame_id = customers.id.clone();
    let sale_id = transactions.columns[0].id.clone();
    let transaction_customer_id = transactions.columns[1].id.clone();
    let customer_id = customers.columns[0].id.clone();
    let customer_name = customers.columns[1].id.clone();
    assert_eq!(transactions.rows.len(), 6);
    assert_eq!(
        customers.unique_keys[0].column_ids,
        vec![customer_id.clone()]
    );

    store
        .apply(Operation::AddJoinFrame {
            primary_frame_id: transaction_id.clone(),
            lookup_frame_id: customer_frame_id.clone(),
            primary_key_column_ids: vec![transaction_customer_id],
            lookup_key_column_ids: vec![customer_id],
            join_type: FrameJoinType::Left,
            columns: vec![
                JoinColumnInput {
                    source_frame_id: transaction_id,
                    source_column_id: sale_id,
                    name: "Sale ID".into(),
                },
                JoinColumnInput {
                    source_frame_id: customer_frame_id,
                    source_column_id: customer_name,
                    name: "Customer".into(),
                },
            ],
            name: "Transactions with customers".into(),
            x: 900.0,
            y: 900.0,
        })
        .unwrap();

    let view = store.view();
    let joined = frame_named(&view.document, "Transactions with customers");
    let customer_output = joined.columns[1].id.clone();
    assert_eq!(joined.rows.len(), 6);
    assert_eq!(
        view.computed_frames[&joined.id].rows[&joined.rows[4].id][&customer_output].typed_value,
        ScalarValue::Null
    );
}

/// A linked frame's identity projection is plumbing, not a transformation:
/// it exists so the child owns its own column ids and so deleting a source
/// column out from under it is refused. The editor hides it, which means it
/// has to survive being saved back through a chain the user did author.
#[test]
fn a_linked_frames_pass_through_projection_is_marked_and_survives_an_edit() {
    let mut store = demo_store();
    let source = frame_named(store.document(), "Orders").clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: source.id.clone(),
            name: "Linked".into(),
            x: 600.0,
            y: 0.0,
        })
        .unwrap();
    let linked = frame_named(store.document(), "Linked").clone();

    // The whole chain is plumbing, so the editor draws nothing.
    let computed = &store.view().computed_frames[&linked.id];
    assert_eq!(computed.steps.len(), 2);
    assert_eq!(computed.pass_through_steps, 2);

    // Its column ids are its own, and they depend on the source's.
    assert!(
        linked
            .columns
            .iter()
            .all(|column| !source.columns.iter().any(|origin| origin.id == column.id))
    );
    assert!(matches!(
        store.apply(Operation::DeleteColumn {
            frame_id: source.id.clone(),
            column_id: source.columns[0].id.clone(),
        }),
        Err(CoreError::ReferencedByFormula(_))
    ));

    // Saving an authored step sends the hidden prefix back with it.
    let quantity = linked.columns[0].id.clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: linked.id.clone(),
            steps: vec![
                FrameStepInput::WithColumns {
                    columns: linked
                        .columns
                        .iter()
                        .map(|column| ExistingFormulaInput {
                            output_column_id: column.id.clone(),
                            name: column.name.clone(),
                            formula: format!("`{}`", column.name),
                        })
                        .collect(),
                },
                FrameStepInput::Select {
                    column_ids: linked.columns.iter().map(|c| c.id.clone()).collect(),
                },
                FrameStepInput::Sort {
                    keys: vec![SortInput {
                        column_id: quantity,
                        descending: true,
                    }],
                },
            ],
        })
        .unwrap();

    let computed = &store.view().computed_frames[&linked.id];
    assert_eq!(
        (computed.steps.len(), computed.pass_through_steps),
        (3, 2),
        "the projection is still there and still marked; only the sort is drawn"
    );
    // And it still reads, which is the thing dropping the projection breaks.
    assert_eq!(
        store.get_frame_page(&linked.id, 0, 10).unwrap().total_rows,
        3
    );
}

/// A calculated column *after* a summarize, reading the aggregate the step
/// before it produced.
///
/// The roadmap listed this as unbuilt, on the grounds that a derivation can
/// project or group-and-aggregate but never both. That was true of the old
/// field layout — `projections` and `aggregates` side by side, one chosen —
/// and stopped being true when the chain replaced it. Steps compose, and
/// each is parsed against the schema at its own position, so this needs no
/// post-aggregate stage: it is two ordinary steps in order.
#[test]
fn a_calculated_column_can_follow_a_summarize_in_the_chain() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: orders,
            name: "Grouped".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let grouped = frame_named(store.document(), "Grouped").id.clone();

    let price = framework_core::id();
    let quantity_sum = framework_core::id();
    let doubled = framework_core::id();
    let view = store
        .apply(Operation::SetFramePipeline {
            frame_id: grouped.clone(),
            steps: vec![
                FrameStepInput::Summarize {
                    group_keys: vec![ExistingFormulaInput {
                        output_column_id: price.clone(),
                        name: "Unit price".into(),
                        formula: "`Unit price`".into(),
                    }],
                    aggregates: vec![ExistingFormulaInput {
                        output_column_id: quantity_sum.clone(),
                        name: "Quantity sum".into(),
                        formula: "`Quantity`.sum()".into(),
                    }],
                    maintain_order: true,
                },
                FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: doubled,
                        name: "Doubled".into(),
                        formula: "`Quantity sum` * 2".into(),
                    }],
                },
            ],
        })
        .expect("a step may read what the step before it produced");

    assert_eq!(
        view.document
            .frame(&grouped)
            .unwrap()
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Unit price", "Quantity sum", "Doubled"]
    );
    assert_eq!(
        store.get_frame_page(&grouped, 0, 10).unwrap().rows,
        vec![
            vec!["14", "3", "6"],
            vec!["7.5", "5", "10"],
            vec!["28", "2", "4"],
        ]
    );
}

/// A ledger grouped by month is still a ledger. Polars answers Float64 to
/// a price and to the sum of prices alike, because a dollar sign is not
/// something it stores — so a summarize was the one place in the document
/// that quietly dropped it, and a column of `$98.00` came back out as
/// `58430442.37999973`.
#[test]
fn money_summed_by_a_group_is_still_money() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: orders,
            name: "By product".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let grouped = frame_named(store.document(), "By product").id.clone();

    let view = store
        .apply(Operation::SetFramePipeline {
            frame_id: grouped.clone(),
            steps: vec![FrameStepInput::Summarize {
                group_keys: vec![ExistingFormulaInput {
                    output_column_id: framework_core::id(),
                    name: "Sold".into(),
                    formula: "`Quantity`".into(),
                }],
                aggregates: vec![
                    ExistingFormulaInput {
                        output_column_id: framework_core::id(),
                        name: "Takings".into(),
                        formula: "`Unit price`.sum()".into(),
                    },
                    // A tally of money is a tally, not money. The rule is
                    // about what the fold answers with, not what it read.
                    ExistingFormulaInput {
                        output_column_id: framework_core::id(),
                        name: "How many".into(),
                        formula: "`Unit price`.count()".into(),
                    },
                ],
                maintain_order: true,
            }],
        })
        .unwrap();

    let written = |name: &str| {
        view.document
            .frame(&grouped)
            .unwrap()
            .columns
            .iter()
            .find(|column| column.name == name)
            .unwrap()
            .data_type
    };
    assert_eq!(written("Takings"), DataType::Currency);
    assert_eq!(written("How many"), DataType::Integer);
    // The group key came in plain and goes out plain.
    assert_eq!(written("Sold"), DataType::Number);
}

/// The editor asks what its draft would produce before saving it, and gets
/// the columns each step leaves behind — with real types, from the plan,
/// without running a query.
#[test]
fn previewing_a_draft_chain_reports_the_schema_at_every_step() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: orders,
            name: "Draft".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let draft = frame_named(store.document(), "Draft").id.clone();

    let preview = store
        .preview_frame_pipeline(
            &draft,
            vec![
                FrameStepInput::Summarize {
                    group_keys: vec![ExistingFormulaInput {
                        output_column_id: framework_core::id(),
                        name: "Unit price".into(),
                        formula: "`Unit price`".into(),
                    }],
                    aggregates: vec![ExistingFormulaInput {
                        output_column_id: framework_core::id(),
                        name: "Quantity sum".into(),
                        formula: "`Quantity`.sum()".into(),
                    }],
                    maintain_order: true,
                },
                FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: framework_core::id(),
                        name: "Doubled".into(),
                        formula: "`Quantity sum` * 2".into(),
                    }],
                },
            ],
        )
        .unwrap();

    assert!(preview.failed_step.is_none());
    assert_eq!(preview.steps.len(), 2);
    // The summarize narrows to its keys and aggregates...
    assert_eq!(
        preview.steps[0]
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Unit price", "Quantity sum"]
    );
    // ...and the step after it sees those, plus what it adds. The types come
    // from the plan rather than from a guess — except for how a number is
    // written, which Polars does not carry and this document does. `Unit
    // price` went into the group by as money and comes out as money; the
    // other two counted and multiplied a quantity, and a quantity is a
    // number.
    assert_eq!(
        preview.steps[1]
            .columns
            .iter()
            .map(|column| (column.name.as_str(), column.data_type))
            .collect::<Vec<_>>(),
        vec![
            ("Unit price", DataType::Currency),
            ("Quantity sum", DataType::Number),
            ("Doubled", DataType::Number),
        ]
    );
}

/// A step the walk cannot get past is an answer, not a failure: the steps
/// before it keep their schemas and the broken one is named, so the editor
/// can point at it rather than going blank.
#[test]
fn previewing_a_broken_step_keeps_the_schemas_before_it() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: orders,
            name: "Draft".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let draft = frame_named(store.document(), "Draft").id.clone();

    let preview = store
        .preview_frame_pipeline(
            &draft,
            vec![
                FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: framework_core::id(),
                        name: "Doubled".into(),
                        formula: "`Quantity` * 2".into(),
                    }],
                },
                FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: framework_core::id(),
                        name: "Broken".into(),
                        formula: "`Nothing here` + 1".into(),
                    }],
                },
                FrameStepInput::Sort {
                    keys: vec![SortInput {
                        column_id: framework_core::id(),
                        descending: false,
                    }],
                },
            ],
        )
        .unwrap();

    assert_eq!(preview.failed_step, Some(1));
    assert!(preview.error.is_some());
    assert_eq!(
        preview.steps.len(),
        1,
        "only the step that parsed has a schema"
    );
    assert!(
        preview.steps[0]
            .columns
            .iter()
            .any(|column| column.name == "Doubled")
    );
    // Saving the same chain is still refused outright — a preview is
    // allowed to be partial, a save is not.
    assert!(
        store
            .apply(Operation::SetFramePipeline {
                frame_id: draft,
                steps: vec![FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: framework_core::id(),
                        name: "Broken".into(),
                        formula: "`Nothing here` + 1".into(),
                    }],
                }],
            })
            .is_err()
    );
}

/// A filter condition that is not yet a test says so in words.
///
/// Naming a column is how a condition starts — you type `Sold on` and then
/// the comparison. Polars catches the half-written version only when it
/// resolves the plan, and answers with the plan itself: every projection,
/// every column by id, none of which is anything the author typed. The
/// schema knows the same thing for free, so the walk asks it first and the
/// preview gets a sentence instead.
#[test]
fn a_filter_condition_that_is_not_a_test_says_what_it_produces() {
    let mut store = demo_store();
    let transactions = frame_named(store.document(), "Transactions").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: transactions,
            name: "Draft".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let draft = frame_named(store.document(), "Draft").id.clone();

    let preview = store
        .preview_frame_pipeline(
            &draft,
            vec![FrameStepInput::Filter {
                predicates: vec!["`Sold on`".into()],
                match_all: true,
            }],
        )
        .unwrap();

    assert_eq!(preview.failed_step, Some(0));
    let error = preview.error.unwrap();
    assert!(
        error.contains("yes/no test") && error.contains("date"),
        "the reason has to name what the condition produces: {error}"
    );
    // None of the plan dump, and none of the prefix that used to call this
    // a failed file import.
    assert!(!error.contains("Resolved plan"), "{error}");
    assert!(!error.contains("Could not import"), "{error}");
    // Several conditions, and the broken one is identified by position, so
    // the author knows which row of the step to fix.
    let preview = store
        .preview_frame_pipeline(
            &draft,
            vec![FrameStepInput::Filter {
                predicates: vec!["`Units` > 2".into(), "`Sold on`".into()],
                match_all: true,
            }],
        )
        .unwrap();
    assert!(preview.error.unwrap().contains("Condition 2"));
}

/// A comparison says "Boolean" even when its operands cannot be compared.
/// Catching that only when Polars reads rows persists a broken chain and
/// makes the source grid look empty; the authored edit has to be refused.
#[test]
fn an_incompatible_filter_comparison_leaves_the_working_frame_in_place() {
    let mut store = demo_store();
    let transactions = frame_named(store.document(), "Transactions").id.clone();

    let error = store
        .apply(Operation::SetFramePipeline {
            frame_id: transactions.clone(),
            steps: vec![FrameStepInput::Filter {
                predicates: vec!["`Sale ID` == `Units`".into()],
                match_all: true,
            }],
        })
        .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("Cannot compare text with an integer"),
        "{error}"
    );
    assert!(
        frame_named(store.document(), "Transactions")
            .steps
            .is_empty()
    );
    assert_eq!(
        store
            .get_frame_page(&transactions, 0, 20)
            .unwrap()
            .total_rows,
        6
    );
}

/// Completion inside a chain offers what the step can actually use.
///
/// After a summarize the source columns are gone and the aggregates are
/// what exist, so suggesting the source's names is suggesting names the
/// formula would be rejected for using.
#[test]
fn completion_after_a_summarize_offers_the_aggregates_not_the_source() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: orders,
            name: "Draft".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let draft = frame_named(store.document(), "Draft").id.clone();
    let steps = vec![
        FrameStepInput::Summarize {
            group_keys: vec![ExistingFormulaInput {
                output_column_id: framework_core::id(),
                name: "Unit price".into(),
                formula: "`Unit price`".into(),
            }],
            aggregates: vec![ExistingFormulaInput {
                output_column_id: framework_core::id(),
                name: "Quantity sum".into(),
                formula: "`Quantity`.sum()".into(),
            }],
            maintain_order: true,
        },
        FrameStepInput::WithColumns {
            columns: vec![ExistingFormulaInput {
                output_column_id: framework_core::id(),
                name: "Doubled".into(),
                formula: String::new(),
            }],
        },
    ];

    let labels = |result: CompletionResult| {
        result
            .suggestions
            .into_iter()
            .map(|suggestion| suggestion.label)
            .collect::<Vec<_>>()
    };

    // Step 1 is the `Add columns` after the summarize.
    let after = labels(store.complete_step_formula(&draft, steps.clone(), 1, "`", 1));
    assert!(
        after.iter().any(|label| label == "Quantity sum"),
        "the aggregate the summarize produced: {after:?}"
    );
    assert!(
        !after.iter().any(|label| label == "Quantity"),
        "the source column is gone after a summarize: {after:?}"
    );

    // Step 0 is the summarize itself, whose own formulas read the source.
    let inside = labels(store.complete_step_formula(&draft, steps, 0, "`", 1));
    assert!(
        inside.iter().any(|label| label == "Quantity"),
        "a summarize's aggregates read the columns before it: {inside:?}"
    );
}

#[test]
fn completion_offers_an_upstream_calculated_column_with_punctuation() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: orders.clone(),
            name: "Draft".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    // This is the important ordering: the downstream frame already exists
    // when its source gains the calculated output.
    store
        .apply(Operation::SetFramePipeline {
            frame_id: orders,
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: framework_core::id(),
                    name: "P|L".into(),
                    formula: "1".into(),
                }],
            }],
        })
        .unwrap();
    let draft = frame_named(store.document(), "Draft").id.clone();

    // The inspector carries the linked frame's hidden identity projection in
    // its draft, then asks for completion at the first authored step after
    // that prefix. Reproduce that exact scope rather than completing against
    // the source frame in isolation.
    let rendered = store.view().computed_frames[&draft].steps.clone();
    let mut steps = rendered
        .iter()
        .map(|step| match step {
            RenderedFrameStep::WithColumns { columns } => FrameStepInput::WithColumns {
                columns: columns
                    .iter()
                    .map(|column| ExistingFormulaInput {
                        output_column_id: column.output_column_id.clone(),
                        name: frame_named(store.document(), "Draft")
                            .columns
                            .iter()
                            .find(|candidate| candidate.id == column.output_column_id)
                            .map(|candidate| candidate.name.clone())
                            .unwrap_or_else(|| "Column".into()),
                        formula: column.formula.clone(),
                    })
                    .collect(),
            },
            RenderedFrameStep::Select { column_ids } => FrameStepInput::Select {
                column_ids: column_ids.clone(),
            },
            other => panic!("unexpected pass-through step {other:?}"),
        })
        .collect::<Vec<_>>();

    let result = store.complete_step_formula(&draft, steps.clone(), steps.len(), "`P|L", 4);
    assert!(
        result
            .suggestions
            .iter()
            .any(|suggestion| suggestion.label == "P|L"),
        "an upstream calculated column is in the formula scope: {:?}",
        result.suggestions
    );

    steps.push(FrameStepInput::Summarize {
        group_keys: Vec::new(),
        aggregates: vec![ExistingFormulaInput {
            output_column_id: framework_core::id(),
            name: "P|L Sum".into(),
            formula: "`P|L`.sum()".into(),
        }],
        maintain_order: true,
    });
    store
        .apply(Operation::SetFramePipeline {
            frame_id: draft.clone(),
            steps,
        })
        .unwrap();
    assert_eq!(
        store.get_frame_page(&draft, 0, 10).unwrap().rows,
        vec![vec!["3".to_string()]],
        "the same suggestion must remain valid when the transformation saves"
    );
}

#[test]
fn pipeline_output_names_are_made_unique_in_written_order() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").id.clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: orders.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![
                    ExistingFormulaInput {
                        output_column_id: framework_core::id(),
                        name: "Repeated".into(),
                        formula: "1".into(),
                    },
                    ExistingFormulaInput {
                        output_column_id: framework_core::id(),
                        name: "Repeated".into(),
                        formula: "2".into(),
                    },
                    ExistingFormulaInput {
                        output_column_id: framework_core::id(),
                        name: "Repeated_2".into(),
                        formula: "3".into(),
                    },
                    ExistingFormulaInput {
                        output_column_id: framework_core::id(),
                        name: "Column 1".into(),
                        formula: "4".into(),
                    },
                    ExistingFormulaInput {
                        output_column_id: framework_core::id(),
                        name: "Column 1".into(),
                        formula: "5".into(),
                    },
                ],
            }],
        })
        .unwrap();

    let names = frame_named(store.document(), "Orders")
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    assert!(names.contains(&"Repeated"));
    assert!(names.contains(&"Repeated_2"));
    assert!(names.contains(&"Repeated_3"));
    assert!(names.contains(&"Column 1"));
    assert!(names.contains(&"Column 2"));
}

/// A sample shows the data as it stands *at* a step, not at the end of the
/// chain — which is the whole point of being able to look partway down one.
#[test]
fn sampling_a_step_shows_the_data_at_that_step_only() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: orders,
            name: "Draft".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let draft = frame_named(store.document(), "Draft").id.clone();
    let steps = vec![
        FrameStepInput::WithColumns {
            columns: vec![ExistingFormulaInput {
                output_column_id: framework_core::id(),
                name: "Doubled".into(),
                formula: "`Quantity` * 2".into(),
            }],
        },
        FrameStepInput::Summarize {
            group_keys: Vec::new(),
            aggregates: vec![ExistingFormulaInput {
                output_column_id: framework_core::id(),
                name: "Rows".into(),
                formula: "`Quantity`.count()".into(),
            }],
            maintain_order: true,
        },
    ];

    // Step 0 still has every row, plus the column it added.
    let first = store
        .sample_frame_step(&draft, steps.clone(), 0, 10)
        .unwrap();
    assert_eq!(first.rows.len(), 3);
    assert!(first.columns.iter().any(|column| column.name == "Doubled"));
    assert!(!first.truncated);

    // Step 1 has collapsed them to one.
    let second = store
        .sample_frame_step(&draft, steps.clone(), 1, 10)
        .unwrap();
    assert_eq!(second.rows.len(), 1);
    assert_eq!(
        second
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Rows"]
    );

    // More rows than asked for is reported without counting them.
    let clipped = store.sample_frame_step(&draft, steps, 0, 2).unwrap();
    assert_eq!(clipped.rows.len(), 2);
    assert!(clipped.truncated);
}

/// Sampling past a step that cannot be worked out fails rather than
/// quietly showing the rows from before it.
#[test]
fn sampling_past_a_broken_step_reports_the_error() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: orders,
            name: "Draft".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let draft = frame_named(store.document(), "Draft").id.clone();

    assert!(
        store
            .sample_frame_step(
                &draft,
                vec![FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: framework_core::id(),
                        name: "Broken".into(),
                        formula: "`Nothing here` + 1".into(),
                    }],
                }],
                0,
                10,
            )
            .is_err()
    );
}

/// Reopening a chain has to show the formulas that were written, not the
/// column ids behind them.
///
/// A linked frame starts with a pass-through projection giving it its own
/// column ids, and the editor saves that projection back with the chain. A
/// summarize on top drops those intermediate columns from the frame's
/// declared schema — so when the steps are rendered again there is no
/// declared name to look up, and every reference below the projection came
/// back as a raw uuid. The projection is a rename, so the name it stood for
/// is still in scope and is inherited instead.
#[test]
fn a_summarize_over_a_linked_frame_renders_names_not_ids() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: orders,
            name: "Grouped".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let grouped = frame_named(store.document(), "Grouped").id.clone();

    // What the editor holds and saves back: the rendered chain as inputs,
    // pass-through steps included.
    let rendered = store.view().computed_frames[&grouped].steps.clone();
    let mut steps: Vec<FrameStepInput> = rendered
        .iter()
        .map(|step| match step {
            RenderedFrameStep::WithColumns { columns } => FrameStepInput::WithColumns {
                columns: columns
                    .iter()
                    .map(|column| ExistingFormulaInput {
                        output_column_id: column.output_column_id.clone(),
                        name: frame_named(store.document(), "Grouped")
                            .columns
                            .iter()
                            .find(|candidate| candidate.id == column.output_column_id)
                            .map(|candidate| candidate.name.clone())
                            .unwrap_or_else(|| "Column".into()),
                        formula: column.formula.clone(),
                    })
                    .collect(),
            },
            RenderedFrameStep::Select { column_ids } => FrameStepInput::Select {
                column_ids: column_ids.clone(),
            },
            other => panic!("unexpected pass-through step {other:?}"),
        })
        .collect();
    steps.push(FrameStepInput::Summarize {
        group_keys: vec![ExistingFormulaInput {
            output_column_id: framework_core::id(),
            name: "Quantity".into(),
            formula: "`Quantity`".into(),
        }],
        aggregates: vec![ExistingFormulaInput {
            output_column_id: framework_core::id(),
            name: "Price sum".into(),
            formula: "`Unit price`.sum()".into(),
        }],
        maintain_order: true,
    });
    store
        .apply(Operation::SetFramePipeline {
            frame_id: grouped.clone(),
            steps,
        })
        .unwrap();

    let reopened = store.view().computed_frames[&grouped].steps.clone();
    let summarize = reopened
        .iter()
        .find_map(|step| match step {
            RenderedFrameStep::Summarize {
                group_keys,
                aggregates,
                ..
            } => Some((group_keys.clone(), aggregates.clone())),
            _ => None,
        })
        .expect("the summarize survives the round trip");
    assert_eq!(summarize.0[0].formula, "`Quantity`");
    assert_eq!(summarize.1[0].formula, "`Unit price`.sum()");
}

/// A chain of two snapshots: refreshing has to walk it from the top, and a
/// live frame below a stale snapshot has to say that its numbers are old.
#[test]
fn staleness_inherits_and_refreshing_walks_the_lineage_from_the_top() {
    let directory = temporary_test_directory("refresh-stale-lineage");
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
    let artifact = create_data_artifact(&source, &directory.join("data")).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Ledger".into(),
            artifact,
            connector: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let ledger_id = frame_named(store.document(), "Ledger").id.clone();

    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: ledger_id.clone(),
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
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: grouped_id.clone(),
            name: "Reported".into(),
            x: 800.0,
            y: 0.0,
        })
        .unwrap();
    let reported_id = frame_named(store.document(), "Reported").id.clone();
    // A live frame below the cache: it has no snapshot of its own to fall
    // behind, so its own reckoning will always call it fresh.
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: grouped_id.clone(),
            name: "Live view".into(),
            x: 800.0,
            y: 400.0,
        })
        .unwrap();
    let live_id = frame_named(store.document(), "Live view").id.clone();

    let data = directory.join("data");
    store.materialize_frame(&grouped_id, &data).unwrap();
    store.materialize_frame(&reported_id, &data).unwrap();

    assert_eq!(
        store.snapshot_refresh_order(),
        vec![grouped_id.clone(), reported_id.clone()],
        "the frame that is read from comes before the frame that reads it"
    );
    let view = store.view();
    assert!(!view.computed_frames[&reported_id].upstream_stale);

    // An edit at the top of the chain. The grouped snapshot is now stale by
    // its own fingerprint; the one below it is not -- it points at exactly
    // the artifact it was built from -- and yet it is serving numbers taken
    // from a snapshot nobody has refreshed.
    store
        .apply(Operation::AddComputedColumn {
            frame_id: ledger_id,
            name: "Doubled".into(),
            formula: "`Debit` * 2".into(),
            after_column_id: None,
        })
        .unwrap();

    let view = store.view();
    assert!(
        view.computed_frames[&grouped_id]
            .materialization
            .as_ref()
            .unwrap()
            .stale
    );
    assert!(
        view.computed_frames[&reported_id]
            .materialization
            .as_ref()
            .unwrap()
            .stale,
        "a fingerprint covers the whole lineage, so a cached frame already \
         knows when an edit further up has moved it"
    );
    assert!(
        view.computed_frames[&live_id].upstream_stale,
        "the live frame has no snapshot to report on, and is the case \
         nothing used to say anything about: its rows come from a snapshot \
         that has not been refreshed"
    );
    assert!(
        !view.computed_frames[&grouped_id].upstream_stale,
        "the stale snapshot is its own, not something it inherited"
    );

    // What a document-wide refresh does: walk the order, refreshing
    // whatever is stale by the time it is reached.
    let mut refreshed = Vec::new();
    for frame_id in store.snapshot_refresh_order() {
        if store.snapshot_is_stale(&frame_id) {
            store.materialize_frame(&frame_id, &data).unwrap();
            refreshed.push(frame_id);
        }
    }
    assert_eq!(
        refreshed,
        vec![grouped_id.clone(), reported_id.clone()],
        "refreshing the parent makes the child stale, and the pass catches it \
         in the same sweep because it reaches the child afterwards"
    );

    let view = store.view();
    for frame_id in [&grouped_id, &reported_id] {
        let computed = &view.computed_frames[frame_id];
        assert!(!computed.materialization.as_ref().unwrap().stale);
        assert!(!computed.upstream_stale);
    }
    assert!(
        !view.computed_frames[&live_id].upstream_stale,
        "and the live frame below them stops warning once they are current"
    );
}
