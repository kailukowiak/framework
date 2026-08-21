use crate::common::*;
use framework_core::*;

/// A blank document, the way `joins.rs` starts its two-frame setups: no demo
/// furniture in the way of the frames a stacking or reshaping test builds
/// for itself.
fn blank_store() -> Store {
    Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Reshape".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    })
}

/// Two frames share "Key" and "Amount"; only the first has "Extra". Stacking
/// the second under a chain over the first lines up what matches by name and
/// leaves the rest null — no mapping written by hand.
#[test]
fn union_stacks_rows_by_column_name_and_fills_gaps_with_nulls() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Left".into(),
            grid: vec![
                vec!["Key".into(), "Amount".into(), "Extra".into()],
                vec!["A".into(), "10".into(), "x".into()],
                vec!["B".into(), "20".into(), "y".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::AddFrame {
            name: "Right".into(),
            grid: vec![
                vec!["Key".into(), "Amount".into()],
                vec!["C".into(), "30".into()],
                vec!["D".into(), "40".into()],
            ],
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let left_id = frame_named(store.document(), "Left").id.clone();
    let right_id = frame_named(store.document(), "Right").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: left_id,
            name: "Chained".into(),
            x: 0.0,
            y: 800.0,
        })
        .unwrap();
    let chained_id = frame_named(store.document(), "Chained").id.clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: chained_id.clone(),
            steps: vec![FrameStepInput::Union { frame_id: right_id }],
        })
        .unwrap();

    let page = store.get_frame_page(&chained_id, 0, 10).unwrap();
    assert_eq!(page.total_rows, 4);
    assert_eq!(page.rows[0], vec!["A", "10", "x"]);
    assert_eq!(page.rows[1], vec!["B", "20", "y"]);
    // Right has nothing named "Extra", so its rows carry nothing there — a
    // page cell reads a null as the empty string, the same as any other
    // unset cell (the "—" glyph is a display-layer rendering, not what
    // `get_frame_page` itself hands back).
    assert_eq!(page.rows[2], vec!["C", "30", ""]);
    assert_eq!(page.rows[3], vec!["D", "40", ""]);
}

/// Stacking reads a frame's own written type, not just its rows — a chain
/// over "Orders" stacked with "Orders" itself keeps "Unit price" money.
#[test]
fn union_keeps_money_written_as_money() {
    let mut store = demo_store();
    let orders_id = frame_named(store.document(), "Orders").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: orders_id.clone(),
            name: "Priced".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let priced_id = frame_named(store.document(), "Priced").id.clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: priced_id.clone(),
            steps: vec![FrameStepInput::Union {
                frame_id: orders_id,
            }],
        })
        .unwrap();

    let priced = frame_named(store.document(), "Priced");
    let written = |name: &str| {
        priced
            .columns
            .iter()
            .find(|column| column.name == name)
            .unwrap()
            .data_type
    };
    assert_eq!(written("Unit price"), DataType::Currency);
    assert_eq!(written("Total"), DataType::Currency);
    assert_eq!(written("Quantity"), DataType::Number);
    assert_eq!(
        store.get_frame_page(&priced_id, 0, 10).unwrap().total_rows,
        6
    );
}

/// A union reads the stacked frame by id every time the chain runs, the way
/// a join reads its lookup — deleting it out from under the chain is refused
/// the same way.
#[test]
fn a_frame_stacked_into_another_cannot_be_deleted() {
    let mut store = demo_store();
    let orders_id = frame_named(store.document(), "Orders").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: orders_id.clone(),
            name: "Priced".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let priced_id = frame_named(store.document(), "Priced").id.clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: priced_id,
            steps: vec![FrameStepInput::Union {
                frame_id: orders_id.clone(),
            }],
        })
        .unwrap();

    let refused = store.apply(Operation::DeleteObject {
        object_id: orders_id,
    });
    let Err(CoreError::ReferencedByFormula(message)) = refused else {
        panic!("a frame stacked into another cannot be deleted");
    };
    assert!(message.contains("is built from"), "{message}");
}

/// A union in a base frame's own step list may name a frame that already
/// derives from that base frame — a cycle that only exists once the chain
/// is saved, so the walk has to catch it before it becomes a document no
/// plan can run.
#[test]
fn a_union_that_would_loop_is_refused() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Rows".into(),
            grid: vec![
                vec!["Name".into(), "Score".into()],
                vec!["A".into(), "10".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let rows_id = frame_named(store.document(), "Rows").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: rows_id.clone(),
            name: "Derived".into(),
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let derived_id = frame_named(store.document(), "Derived").id.clone();

    let result = store.apply(Operation::SetFramePipeline {
        frame_id: rows_id,
        steps: vec![FrameStepInput::Union {
            frame_id: derived_id,
        }],
    });
    let Err(error) = result else {
        panic!("a union back through its own lineage must be refused");
    };
    assert!(error.to_string().contains("Circular"), "{error}");
}

/// Expansion is the table-shaped loop: every entry line is paired with
/// every offset, while the offset column receives an id owned by the output
/// frame so formulas can safely address it after a refresh.
#[test]
fn expand_pairs_every_row_and_keeps_output_column_identity() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Entries".into(),
            grid: vec![
                vec!["Project".into()],
                vec!["Alpha".into()],
                vec!["Beta".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::AddFrame {
            name: "Offsets".into(),
            grid: vec![
                vec!["Day offset".into()],
                vec!["0".into()],
                vec!["1".into()],
                vec!["2".into()],
            ],
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let entries_id = frame_named(store.document(), "Entries").id.clone();
    let offsets_id = frame_named(store.document(), "Offsets").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: entries_id,
            name: "Calendar rows".into(),
            x: 0.0,
            y: 800.0,
        })
        .unwrap();
    let calendar_id = frame_named(store.document(), "Calendar rows").id.clone();
    let next_offset_id = framework_core::id();
    let steps = vec![
        FrameStepInput::Expand {
            frame_id: offsets_id,
        },
        FrameStepInput::WithColumns {
            columns: vec![ExistingFormulaInput {
                output_column_id: next_offset_id,
                name: "Next offset".into(),
                formula: "`Day offset` + 1".into(),
            }],
        },
    ];
    store
        .apply(Operation::SetFramePipeline {
            frame_id: calendar_id.clone(),
            steps: steps.clone(),
        })
        .unwrap();

    let first_id = frame_named(store.document(), "Calendar rows")
        .columns
        .iter()
        .find(|column| column.name == "Day offset")
        .unwrap()
        .id
        .clone();
    let page = store.get_frame_page(&calendar_id, 0, 10).unwrap();
    assert_eq!(page.total_rows, 6);
    assert_eq!(page.rows[0], vec!["Alpha", "0", "1"]);
    assert_eq!(page.rows[2], vec!["Alpha", "2", "3"]);
    assert_eq!(page.rows[3], vec!["Beta", "0", "1"]);
    let rendered = &store.view().computed_frames[&calendar_id].steps[1];
    let RenderedFrameStep::WithColumns { columns } = rendered else {
        panic!("the calculation after expansion should remain editable")
    };
    assert_eq!(columns[0].formula, "`Day offset` + 1");

    store
        .apply(Operation::SetFramePipeline {
            frame_id: calendar_id,
            steps,
        })
        .unwrap();
    let refreshed_id = frame_named(store.document(), "Calendar rows")
        .columns
        .iter()
        .find(|column| column.name == "Day offset")
        .unwrap()
        .id
        .clone();
    assert_eq!(refreshed_id, first_id);
}

/// A long frame of month, category, and money pivoted on category sums
/// amount into one baked column per category, sorted by value.
#[test]
fn pivot_bakes_one_column_per_distinct_value() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Sales".into(),
            grid: vec![
                vec!["Month".into(), "Category".into(), "Amount".into()],
                vec!["Jan".into(), "Widgets".into(), "$10.00".into()],
                vec!["Jan".into(), "Gadgets".into(), "$20.00".into()],
                vec!["Feb".into(), "Widgets".into(), "$15.00".into()],
                vec!["Feb".into(), "Gadgets".into(), "$25.00".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let sales = frame_named(store.document(), "Sales");
    let sales_id = sales.id.clone();
    let category_id = sales.columns[1].id.clone();
    let amount_id = sales.columns[2].id.clone();

    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: sales_id,
            name: "By category".into(),
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let pivoted_id = frame_named(store.document(), "By category").id.clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: pivoted_id.clone(),
            steps: vec![FrameStepInput::Pivot {
                names_column_id: category_id,
                values_column_id: amount_id,
                aggregate: PivotAggregate::Sum,
            }],
        })
        .unwrap();

    let pivoted = frame_named(store.document(), "By category");
    let names = pivoted
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["Month", "Gadgets", "Widgets"]);
    // A pivot cell is written the way the values column was.
    assert_eq!(pivoted.columns[1].data_type, DataType::Currency);
    assert_eq!(pivoted.columns[2].data_type, DataType::Currency);

    let page = store.get_frame_page(&pivoted_id, 0, 10).unwrap();
    assert_eq!(page.total_rows, 2);
    // A page cell is the raw number; the "$" is display formatting no test
    // here asks for.
    assert_eq!(page.rows[0], vec!["Jan", "20", "10"]);
    assert_eq!(page.rows[1], vec!["Feb", "25", "15"]);
}

/// A pivot saved twice over the same data hands the same value the same
/// output column id both times, so whatever was written against "Gadgets"
/// the first time is still writing against it the second.
#[test]
fn resaving_a_pivot_keeps_output_column_identity() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Sales".into(),
            grid: vec![
                vec!["Month".into(), "Category".into(), "Amount".into()],
                vec!["Jan".into(), "Widgets".into(), "$10.00".into()],
                vec!["Jan".into(), "Gadgets".into(), "$20.00".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let sales = frame_named(store.document(), "Sales");
    let sales_id = sales.id.clone();
    let category_id = sales.columns[1].id.clone();
    let amount_id = sales.columns[2].id.clone();

    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: sales_id,
            name: "By category".into(),
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let pivoted_id = frame_named(store.document(), "By category").id.clone();
    let pivot_step = || FrameStepInput::Pivot {
        names_column_id: category_id.clone(),
        values_column_id: amount_id.clone(),
        aggregate: PivotAggregate::Sum,
    };
    store
        .apply(Operation::SetFramePipeline {
            frame_id: pivoted_id.clone(),
            steps: vec![pivot_step()],
        })
        .unwrap();
    fn column_id(store: &Store, name: &str) -> Id {
        frame_named(store.document(), "By category")
            .columns
            .iter()
            .find(|column| column.name == name)
            .unwrap()
            .id
            .clone()
    }
    let gadgets_first = column_id(&store, "Gadgets");
    let widgets_first = column_id(&store, "Widgets");

    store
        .apply(Operation::SetFramePipeline {
            frame_id: pivoted_id,
            steps: vec![pivot_step()],
        })
        .unwrap();
    assert_eq!(column_id(&store, "Gadgets"), gadgets_first);
    assert_eq!(column_id(&store, "Widgets"), widgets_first);
}

/// A pivot needs a names column that can actually name new columns; a number
/// cannot, so it is refused rather than rendered.
#[test]
fn pivot_on_a_numeric_names_column_is_refused() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Sales".into(),
            grid: vec![
                vec!["Month".into(), "Category".into(), "Amount".into()],
                vec!["Jan".into(), "Widgets".into(), "$10.00".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let sales = frame_named(store.document(), "Sales");
    let sales_id = sales.id.clone();
    let month_id = sales.columns[0].id.clone();
    let amount_id = sales.columns[2].id.clone();

    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: sales_id,
            name: "Draft".into(),
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let draft_id = frame_named(store.document(), "Draft").id.clone();

    let result = store.apply(Operation::SetFramePipeline {
        frame_id: draft_id,
        steps: vec![FrameStepInput::Pivot {
            // A currency column cannot name the new columns.
            names_column_id: amount_id,
            values_column_id: month_id,
            aggregate: PivotAggregate::Sum,
        }],
    });
    let Err(CoreError::InvalidOperation(message)) = result else {
        panic!("a pivot on a non-text names column must be refused");
    };
    assert!(message.contains("New columns need names"), "{message}");
}

/// Summing text has no answer; the pivot says so rather than guessing.
#[test]
fn pivot_summing_a_text_values_column_is_refused() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Sales".into(),
            grid: vec![
                vec!["Month".into(), "Category".into(), "Amount".into()],
                vec!["Jan".into(), "Widgets".into(), "$10.00".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let sales = frame_named(store.document(), "Sales");
    let sales_id = sales.id.clone();
    let month_id = sales.columns[0].id.clone();
    let category_id = sales.columns[1].id.clone();

    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: sales_id,
            name: "Draft".into(),
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let draft_id = frame_named(store.document(), "Draft").id.clone();

    let result = store.apply(Operation::SetFramePipeline {
        frame_id: draft_id,
        steps: vec![FrameStepInput::Pivot {
            names_column_id: month_id,
            values_column_id: category_id,
            aggregate: PivotAggregate::Sum,
        }],
    });
    let Err(CoreError::InvalidOperation(message)) = result else {
        panic!("summing a text values column must be refused");
    };
    assert!(message.contains("cannot be summed"), "{message}");
}

/// A wide frame of a region and two money columns, unpivoted over both,
/// melts into twice the rows: a name column carrying the columns' display
/// names and a value column carrying what they held, still money because
/// both melted columns agreed on it.
#[test]
fn unpivot_melts_columns_into_name_value_rows() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Wide".into(),
            grid: vec![
                vec!["Region".into(), "Jan".into(), "Feb".into()],
                vec!["North".into(), "$100.00".into(), "$150.00".into()],
                vec!["South".into(), "$200.00".into(), "$250.00".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let wide = frame_named(store.document(), "Wide");
    let wide_id = wide.id.clone();

    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: wide_id,
            name: "Long".into(),
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let long_id = frame_named(store.document(), "Long").id.clone();
    // The caller mints the two new columns' ids, the way an existing
    // formula's output column id is minted before the formula is saved.
    // The melted columns arrive as written text, resolved at save time.
    let name_column_id = framework_core::id();
    let value_column_id = framework_core::id();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: long_id.clone(),
            steps: vec![FrameStepInput::Unpivot {
                columns: "`Jan`, `Feb`".into(),
                name_column_id: name_column_id.clone(),
                name_column_name: "Month".into(),
                value_column_id: value_column_id.clone(),
                value_column_name: "Amount".into(),
            }],
        })
        .unwrap();

    let long = frame_named(store.document(), "Long");
    let names = long
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["Region", "Month", "Amount"]);
    let amount_column = long
        .columns
        .iter()
        .find(|column| column.name == "Amount")
        .unwrap();
    assert_eq!(amount_column.data_type, DataType::Currency);

    let page = store.get_frame_page(&long_id, 0, 10).unwrap();
    assert_eq!(page.total_rows, 4);
    // Column-major: every row for the first melted column, then the second.
    // Raw cell values again — no "$" at this layer.
    assert_eq!(page.rows[0], vec!["North", "Jan", "100"]);
    assert_eq!(page.rows[1], vec!["South", "Jan", "200"]);
    assert_eq!(page.rows[2], vec!["North", "Feb", "150"]);
    assert_eq!(page.rows[3], vec!["South", "Feb", "250"]);
}

/// `except(`Region`)` is the short spelling of "melt the whole wide
/// frame": the selector resolves against the schema when the step is
/// saved, and what gets baked is the concrete columns it matched then.
#[test]
fn unpivot_accepts_a_selector_for_the_melted_columns() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Wide".into(),
            grid: vec![
                vec!["Region".into(), "Q1".into(), "Q2".into(), "Q3".into()],
                vec!["North".into(), "1".into(), "2".into(), "3".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let wide_id = frame_named(store.document(), "Wide").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: wide_id,
            name: "Long".into(),
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let long_id = frame_named(store.document(), "Long").id.clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: long_id.clone(),
            steps: vec![FrameStepInput::Unpivot {
                columns: "except(`Region`)".into(),
                name_column_id: framework_core::id(),
                name_column_name: "Quarter".into(),
                value_column_id: framework_core::id(),
                value_column_name: "Value".into(),
            }],
        })
        .unwrap();

    let page = store.get_frame_page(&long_id, 0, 10).unwrap();
    assert_eq!(page.total_rows, 3);
    assert_eq!(page.rows[0], vec!["North", "Q1", "1"]);
    assert_eq!(page.rows[2], vec!["North", "Q3", "3"]);
}

/// A melt list naming a column nothing before the step produces is
/// refused when the chain is saved, with the name in the sentence — the
/// resolution is the check, the same way it is for a formula.
#[test]
fn unpivot_refuses_a_column_the_list_cannot_resolve() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Wide".into(),
            grid: vec![
                vec!["Region".into(), "Jan".into()],
                vec!["North".into(), "1".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let wide_id = frame_named(store.document(), "Wide").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: wide_id,
            name: "Long".into(),
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let long_id = frame_named(store.document(), "Long").id.clone();
    let result = store.apply(Operation::SetFramePipeline {
        frame_id: long_id,
        steps: vec![FrameStepInput::Unpivot {
            columns: "`Jna`".into(),
            name_column_id: framework_core::id(),
            name_column_name: "Month".into(),
            value_column_id: framework_core::id(),
            value_column_name: "Value".into(),
        }],
    });
    let Err(error) = result else {
        panic!("a melt list naming an unknown column must be refused");
    };
    assert!(error.to_string().contains("‘Jna’"), "{error}");
}

/// The editor asks what a draft pivot would produce before saving it, the
/// same way it asks of any other step — and gets the baked value columns
/// back with their discovered types.
#[test]
fn previewing_a_draft_pivot_reports_the_discovered_schema() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Sales".into(),
            grid: vec![
                vec!["Month".into(), "Category".into(), "Amount".into()],
                vec!["Jan".into(), "Widgets".into(), "$10.00".into()],
                vec!["Jan".into(), "Gadgets".into(), "$20.00".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let sales = frame_named(store.document(), "Sales");
    let sales_id = sales.id.clone();
    let category_id = sales.columns[1].id.clone();
    let amount_id = sales.columns[2].id.clone();

    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: sales_id,
            name: "Draft".into(),
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let draft_id = frame_named(store.document(), "Draft").id.clone();

    let preview = store
        .preview_frame_pipeline(
            &draft_id,
            vec![FrameStepInput::Pivot {
                names_column_id: category_id,
                values_column_id: amount_id,
                aggregate: PivotAggregate::Sum,
            }],
        )
        .unwrap();

    assert!(preview.failed_step.is_none());
    assert_eq!(preview.steps.len(), 1);
    assert_eq!(
        preview.steps[0]
            .columns
            .iter()
            .map(|column| (column.name.as_str(), column.data_type))
            .collect::<Vec<_>>(),
        vec![
            ("Month", DataType::String),
            ("Gadgets", DataType::Currency),
            ("Widgets", DataType::Currency),
        ]
    );
}

/// The refusing policy: with one row per cell, `None` hands the values
/// through untouched, and a cell nothing landed in is null rather than a
/// zero no one wrote.
#[test]
fn a_pivot_without_an_aggregate_passes_unique_cells_through() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Sales".into(),
            grid: vec![
                vec!["Month".into(), "Category".into(), "Amount".into()],
                vec!["Jan".into(), "Widgets".into(), "$10.00".into()],
                vec!["Jan".into(), "Gadgets".into(), "$20.00".into()],
                vec!["Feb".into(), "Widgets".into(), "$15.00".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let sales = frame_named(store.document(), "Sales");
    let sales_id = sales.id.clone();
    let category_id = sales.columns[1].id.clone();
    let amount_id = sales.columns[2].id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: sales_id,
            name: "Grid".into(),
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let pivoted_id = frame_named(store.document(), "Grid").id.clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: pivoted_id.clone(),
            steps: vec![FrameStepInput::Pivot {
                names_column_id: category_id,
                values_column_id: amount_id,
                aggregate: PivotAggregate::None,
            }],
        })
        .unwrap();

    let page = store.get_frame_page(&pivoted_id, 0, 10).unwrap();
    assert_eq!(page.rows[0], vec!["Jan", "20", "10"]);
    assert_eq!(page.rows[1], vec!["Feb", "", "15"]);
    // Passing a value through is not summing it: money stays money here
    // for the same reason it does under Sum.
    let pivoted = frame_named(store.document(), "Grid");
    assert_eq!(pivoted.columns[1].data_type, DataType::Currency);
}

/// Two rows landing in one cell under `None` is the error the policy
/// exists to raise — being told the data is not one-row-per-cell is why
/// anyone picks it.
#[test]
fn a_pivot_without_an_aggregate_refuses_a_second_row_in_a_cell() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Sales".into(),
            grid: vec![
                vec!["Month".into(), "Category".into(), "Amount".into()],
                vec!["Jan".into(), "Widgets".into(), "$10.00".into()],
                vec!["Jan".into(), "Widgets".into(), "$99.00".into()],
                vec!["Jan".into(), "Gadgets".into(), "$20.00".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let sales = frame_named(store.document(), "Sales");
    let sales_id = sales.id.clone();
    let category_id = sales.columns[1].id.clone();
    let amount_id = sales.columns[2].id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: sales_id,
            name: "Grid".into(),
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let pivoted_id = frame_named(store.document(), "Grid").id.clone();
    let saved = store.apply(Operation::SetFramePipeline {
        frame_id: pivoted_id.clone(),
        steps: vec![FrameStepInput::Pivot {
            names_column_id: category_id,
            values_column_id: amount_id,
            aggregate: PivotAggregate::None,
        }],
    });
    // Whether the refusal lands at save time or on the first read depends
    // on which computation touches the cell first; either way it says why.
    let message = match saved {
        Err(error) => error.to_string(),
        Ok(_) => store
            .get_frame_page(&pivoted_id, 0, 10)
            .expect_err("two rows in one cell must not read cleanly")
            .to_string(),
    };
    assert!(
        message.contains("told not to combine"),
        "unexpected refusal: {message}"
    );
}
