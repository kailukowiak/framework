use crate::common::*;
use framework_core::*;

/// Every frame on the canvas, by name.
fn frame_id(store: &Store, name: &str) -> Id {
    frame_named(store.document(), name).id.clone()
}

fn window_showing(store: &Store, frame_id: &str) -> Id {
    store
        .document()
        .views
        .iter()
        .find(|view| view.object_id == frame_id)
        .unwrap()
        .id
        .clone()
}

fn tabs(store: &Store, view_id: &str) -> Vec<Id> {
    store.document().view(view_id).unwrap().tabs().to_vec()
}

/// The point of the whole unification: two tabs on one card filter
/// independently because they are two frames, not because a parallel state
/// machine keeps their filters apart.
#[test]
fn branched_tabs_filter_the_same_data_independently() {
    let mut store = demo_store();
    let orders_id = frame_id(&store, "Orders");
    let window_id = window_showing(&store, &orders_id);

    let branched = store
        .apply(Operation::BranchFrame {
            view_id: window_id.clone(),
            frame_id: orders_id.clone(),
        })
        .unwrap();
    let copy_id = branched
        .document
        .view(&window_id)
        .unwrap()
        .object_id
        .clone();
    assert_eq!(
        tabs(&store, &window_id),
        [orders_id.clone(), copy_id.clone()]
    );
    assert_ne!(copy_id, orders_id);
    assert_eq!(
        branched.document.frame(&copy_id).unwrap().name,
        "Orders copy"
    );

    store
        .apply(Operation::SetFrameDisplayFilter {
            frame_id: copy_id.clone(),
            filters: vec!["`Quantity` > 2".into()],
            filter_match_all: true,
        })
        .unwrap();
    assert_eq!(
        store.get_frame_page(&orders_id, 0, 100).unwrap().total_rows,
        3
    );
    assert_eq!(
        store.get_frame_page(&copy_id, 0, 100).unwrap().total_rows,
        2
    );

    // A display filter is stored against column ids, so renaming the column
    // it reads re-renders it rather than breaking it.
    let quantity_id = frame_named(store.document(), "Orders").columns[0]
        .id
        .clone();
    store
        .apply(Operation::SetFrameDisplayFilter {
            frame_id: orders_id.clone(),
            filters: vec!["`Quantity` > 1".into()],
            filter_match_all: true,
        })
        .unwrap();
    let renamed = store
        .apply(Operation::RenameColumn {
            frame_id: orders_id.clone(),
            column_id: quantity_id,
            name: "Units".into(),
        })
        .unwrap();
    assert!(matches!(
        &renamed.computed_frames[&orders_id].display_steps[0],
        RenderedFrameStep::Filter { predicates, .. } if predicates == &["`Units` > 1"]
    ));
    // The branch has columns of its own, so its filter still names the copy's
    // "Quantity" — and still selects the same rows.
    assert!(matches!(
        &renamed.computed_frames[&copy_id].display_steps[0],
        RenderedFrameStep::Filter { predicates, .. } if predicates == &["`Quantity` > 2"]
    ));
    assert_eq!(
        store.get_frame_page(&copy_id, 0, 100).unwrap().total_rows,
        2
    );
    store
        .apply(Operation::SetFrameDisplayFilter {
            frame_id: orders_id.clone(),
            filters: Vec::new(),
            filter_match_all: true,
        })
        .unwrap();

    // Closing a branched tab is deleting the frame it is.
    store
        .apply(Operation::DeleteObject { object_id: copy_id })
        .unwrap();
    assert_eq!(tabs(&store, &window_id), [orders_id]);
}

#[test]
fn frame_tabs_move_between_windows_and_detach_without_copying_data() {
    let mut store = demo_store();
    let object_count = store.document().objects.len();
    let window_count = store.document().views.len();
    let orders_id = frame_id(&store, "Orders");
    let transactions_id = frame_id(&store, "Transactions");
    let orders_window_id = window_showing(&store, &orders_id);
    let transactions_window_id = window_showing(&store, &transactions_id);

    // Orders and Transactions are unrelated frames, so Orders is not a legal
    // tab of the Transactions card.
    assert!(matches!(
        store.apply(Operation::MoveTab {
            source_view_id: orders_window_id.clone(),
            target_view_id: transactions_window_id.clone(),
            object_id: orders_id.clone(),
            target_index: 1,
        }),
        Err(CoreError::InvalidOperation(_))
    ));

    // A branch of Transactions is, and moving it is the same edit whether it
    // lands on another card or reorders within its own.
    store
        .apply(Operation::BranchFrame {
            view_id: transactions_window_id.clone(),
            frame_id: transactions_id.clone(),
        })
        .unwrap();
    let branch_id = store
        .document()
        .view(&transactions_window_id)
        .unwrap()
        .object_id
        .clone();

    let detached = store
        .apply(Operation::DetachTab {
            view_id: transactions_window_id.clone(),
            object_id: branch_id.clone(),
            x: -10.0,
            y: -20.0,
        })
        .unwrap();
    assert_eq!(detached.document.objects.len(), object_count + 1);
    assert_eq!(detached.document.views.len(), window_count + 1);
    assert_eq!(
        detached
            .document
            .view(&transactions_window_id)
            .unwrap()
            .object_id,
        transactions_id
    );
    assert_eq!(
        tabs(&store, &transactions_window_id),
        std::slice::from_ref(&transactions_id)
    );
    let detached_window = detached
        .document
        .views
        .iter()
        .find(|view| view.object_id == branch_id)
        .unwrap();
    assert_eq!((detached_window.x, detached_window.y), (0.0, 0.0));

    // And back again, into the card its source is on.
    store
        .apply(Operation::MoveTab {
            source_view_id: detached_window.id.clone(),
            target_view_id: transactions_window_id.clone(),
            object_id: branch_id.clone(),
            target_index: 0,
        })
        .unwrap();
    assert_eq!(store.document().views.len(), window_count);
    assert_eq!(
        tabs(&store, &transactions_window_id),
        [branch_id.clone(), transactions_id]
    );
    assert_eq!(
        store
            .document()
            .view(&transactions_window_id)
            .unwrap()
            .object_id,
        branch_id
    );
    assert_eq!(orders_window_id, window_showing(&store, &orders_id));
}

#[test]
fn tabs_reorder_and_close_without_stale_host_state() {
    let mut store = demo_store();
    let orders_id = frame_id(&store, "Orders");
    let window_id = window_showing(&store, &orders_id);

    let branch = |store: &mut Store| {
        store
            .apply(Operation::BranchFrame {
                view_id: window_id.clone(),
                frame_id: orders_id.clone(),
            })
            .unwrap()
            .document
            .view(&window_id)
            .unwrap()
            .object_id
            .clone()
    };
    let first_copy = branch(&mut store);
    let second_copy = branch(&mut store);
    assert_eq!(
        tabs(&store, &window_id),
        [orders_id.clone(), first_copy.clone(), second_copy.clone()]
    );

    store
        .apply(Operation::MoveTab {
            source_view_id: window_id.clone(),
            target_view_id: window_id.clone(),
            object_id: orders_id.clone(),
            target_index: 2,
        })
        .unwrap();
    assert_eq!(
        tabs(&store, &window_id),
        [first_copy.clone(), orders_id.clone(), second_copy.clone()]
    );

    // A tab is a frame, so renaming it is renaming the object.
    let renamed = store
        .apply(Operation::RenameObject {
            object_id: first_copy.clone(),
            name: "Big orders".into(),
        })
        .unwrap();
    assert_eq!(
        renamed.document.frame(&first_copy).unwrap().name,
        "Big orders"
    );

    store
        .apply(Operation::DeleteObject {
            object_id: second_copy,
        })
        .unwrap();
    assert_eq!(tabs(&store, &window_id), [first_copy, orders_id.clone()]);

    // The card's own frame still has branches derived from it, so closing it
    // would leave them orphaned.
    assert!(matches!(
        store.apply(Operation::DeleteObject {
            object_id: orders_id,
        }),
        Err(CoreError::ReferencedByFormula(_))
    ));
}

/// A joined frame has two parents, so neither card can claim it as a tab.
#[test]
fn a_joined_frame_cannot_become_a_tab() {
    let mut store = Store::new(Document::blank("Joins"));
    store
        .apply(Operation::AddFrame {
            name: "Orders".into(),
            grid: vec![
                vec!["Order ID".into(), "Customer ID".into()],
                vec!["O-1".into(), "C-1".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::AddFrame {
            name: "Customers".into(),
            grid: vec![
                vec!["Customer ID".into(), "Customer name".into()],
                vec!["C-1".into(), "Ada".into()],
            ],
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let orders = frame_named(store.document(), "Orders").clone();
    let customers = frame_named(store.document(), "Customers").clone();

    store
        .apply(Operation::SetUniqueKey {
            frame_id: customers.id.clone(),
            column_ids: vec![customers.columns[0].id.clone()],
            enabled: true,
        })
        .unwrap();
    store
        .apply(Operation::AddJoinFrame {
            primary_frame_id: orders.id.clone(),
            lookup_frame_id: customers.id.clone(),
            primary_key_column_ids: vec![orders.columns[1].id.clone()],
            lookup_key_column_ids: vec![customers.columns[0].id.clone()],
            join_type: FrameJoinType::Left,
            columns: vec![
                JoinColumnInput {
                    source_frame_id: orders.id.clone(),
                    source_column_id: orders.columns[0].id.clone(),
                    name: "Order ID".into(),
                },
                JoinColumnInput {
                    source_frame_id: customers.id.clone(),
                    source_column_id: customers.columns[1].id.clone(),
                    name: "Customer name".into(),
                },
            ],
            name: "Joined".into(),
            x: 600.0,
            y: 0.0,
        })
        .unwrap();
    let joined_id = frame_id(&store, "Joined");
    let orders_window_id = window_showing(&store, &orders.id);

    // Joined derives from Orders, which is on that card — but it also reads
    // Customers, so it has no unambiguous home.
    assert!(matches!(
        store.apply(Operation::MoveTab {
            source_view_id: window_showing(&store, &joined_id),
            target_view_id: orders_window_id.clone(),
            object_id: joined_id.clone(),
            target_index: 1,
        }),
        Err(CoreError::InvalidOperation(_))
    ));
    // Nor can it be branched onto a card it is not already a tab of.
    assert!(matches!(
        store.apply(Operation::BranchFrame {
            view_id: orders_window_id,
            frame_id: joined_id,
        }),
        Err(CoreError::InvalidOperation(_))
    ));
}

/// A plot of a frame on the card is another rendering of the same data, so
/// it earns a tab beside it rather than a window of its own.
#[test]
fn a_plot_of_a_frame_on_the_card_becomes_a_tab_of_it() {
    let mut store = demo_store();
    let orders_id = frame_id(&store, "Orders");
    let window_id = window_showing(&store, &orders_id);
    let windows_before = store.document().views.len();

    let view = store
        .apply(Operation::AddPlot {
            name: "Units by sale".into(),
            source_frame_id: orders_id.clone(),
            spec: serde_json::json!({"mark": "bar"}),
            x: 0.0,
            y: 0.0,
            view_id: Some(window_id.clone()),
        })
        .unwrap()
        .document;

    let plot_id = view
        .objects
        .iter()
        .find(|object| object.name() == "Units by sale")
        .unwrap()
        .id()
        .to_string();
    // No new window: the plot joined the strip and became the selected tab.
    assert_eq!(view.views.len(), windows_before);
    assert_eq!(tabs(&store, &window_id), [orders_id, plot_id.clone()]);
    assert_eq!(view.view(&window_id).unwrap().object_id, plot_id);
}

/// The tab rule is about what a card shows, not about what kind of object
/// is asking: a plot of a frame the card does not hold is as homeless as a
/// derived frame would be.
#[test]
fn a_plot_cannot_tab_onto_a_card_that_does_not_show_its_frame() {
    let mut store = demo_store();
    let orders_id = frame_id(&store, "Orders");
    let customers_id = frame_id(&store, "Customers");

    assert!(matches!(
        store.apply(Operation::AddPlot {
            name: "Wrong home".into(),
            source_frame_id: orders_id,
            spec: serde_json::json!({"mark": "bar"}),
            x: 0.0,
            y: 0.0,
            view_id: Some(window_showing(&store, &customers_id)),
        }),
        Err(CoreError::InvalidOperation(_))
    ));
    // The rejected plot must not be left behind in the document.
    assert!(
        !store
            .document()
            .objects
            .iter()
            .any(|object| object.name() == "Wrong home")
    );
}

/// Popping a plot tab out gives it its own window, and closing the card's
/// last remaining tab is still the card going away — the plot is treated
/// exactly like any other tab, because that is the point.
#[test]
fn a_plot_tab_detaches_into_a_window_of_its_own() {
    let mut store = demo_store();
    let orders_id = frame_id(&store, "Orders");
    let window_id = window_showing(&store, &orders_id);
    store
        .apply(Operation::AddPlot {
            name: "Units by sale".into(),
            source_frame_id: orders_id.clone(),
            spec: serde_json::json!({"mark": "bar"}),
            x: 0.0,
            y: 0.0,
            view_id: Some(window_id.clone()),
        })
        .unwrap();
    let plot_id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Units by sale")
        .unwrap()
        .id()
        .to_string();

    let detached = store
        .apply(Operation::DetachTab {
            view_id: window_id.clone(),
            object_id: plot_id.clone(),
            x: 1200.0,
            y: 40.0,
        })
        .unwrap()
        .document;

    assert_eq!(tabs(&store, &window_id), [orders_id]);
    let plot_window = detached
        .views
        .iter()
        .find(|view| view.object_id == plot_id)
        .unwrap();
    assert_eq!((plot_window.x, plot_window.y), (1200.0, 40.0));
    assert!(plot_window.tab_object_ids.is_empty());
}

#[test]
fn frame_view_resize_is_persistent_and_clamped() {
    let mut store = demo_store();
    let frame_id = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) => Some(frame.id.clone()),
            _ => None,
        })
        .unwrap();
    let view_id = store
        .document()
        .views
        .iter()
        .find(|view| view.object_id == frame_id)
        .unwrap()
        .id
        .clone();

    let resized = store
        .apply(Operation::ResizeView {
            view_id: view_id.clone(),
            width: 840.0,
            height: 510.0,
        })
        .unwrap();
    let view = resized
        .document
        .views
        .iter()
        .find(|view| view.id == view_id)
        .unwrap();
    assert_eq!((view.width, view.height), (840.0, 510.0));

    let clamped = store
        .apply(Operation::ResizeView {
            view_id: view_id.clone(),
            width: 20.0,
            height: 20.0,
        })
        .unwrap();
    let view = clamped
        .document
        .views
        .iter()
        .find(|view| view.id == view_id)
        .unwrap();
    assert_eq!((view.width, view.height), (360.0, 210.0));
}

/// A frame that holds its own rows carries its matches on the view, keyed by
/// row id — so a rule survives sorting and filtering the way direct
/// formatting does, and a rule scoped to a column still reads the whole row.
#[test]
fn conditional_formatting_rules_match_the_rows_a_view_carries() {
    let mut store = demo_store();
    let orders_id = frame_id(&store, "Orders");
    let orders = frame_named(store.document(), "Orders").clone();
    let quantity_id = orders
        .columns
        .iter()
        .find(|column| column.name == "Quantity")
        .unwrap()
        .id
        .clone();

    let styled = store
        .apply(Operation::SetFrameStyleRules {
            frame_id: orders_id.clone(),
            rules: vec![FrameStyleRuleInput {
                id: None,
                formula: "`Quantity` > 2".into(),
                column_id: Some(quantity_id.clone()),
                output: FrameStyleOutput::Condition {
                    style: FrameCellStyle {
                        bold: Some(true),
                        ..FrameCellStyle::default()
                    },
                },
            }],
        })
        .unwrap();
    let rule_id = styled
        .document
        .frame(&orders_id)
        .unwrap()
        .display
        .style_rules[0]
        .id
        .clone();

    let computed = &styled.computed_frames[&orders_id];
    assert!(computed.style_rule_errors.is_empty());
    let expected: Vec<Id> = orders
        .rows
        .iter()
        .filter(|row| {
            row.cells
                .get(&quantity_id)
                .and_then(|cell| cell.raw.parse::<f64>().ok())
                .is_some_and(|quantity| quantity > 2.0)
        })
        .map(|row| row.id.clone())
        .collect();
    assert!(!expected.is_empty(), "the demo has rows over two");
    let mut matched: Vec<Id> = computed.style_matches.keys().cloned().collect();
    let mut expected_sorted = expected.clone();
    matched.sort();
    expected_sorted.sort();
    assert_eq!(matched, expected_sorted);
    // Only rows that matched are carried, and each names the rule and the
    // style it resolved to.
    assert!(computed.style_matches.values().all(|matched| {
        matched.len() == 1 && matched[0].rule_id == rule_id && matched[0].style.bold == Some(true)
    }));
}

/// The other two readings of a hidden column: a label per row, and a number
/// per row. Neither one picks rows — every row gets an answer, and the
/// answer is what the style is made of.
#[test]
fn conditional_formatting_reads_labels_as_cases_and_numbers_as_a_ramp() {
    let mut store = demo_store();
    let orders_id = frame_id(&store, "Orders");
    let orders = frame_named(store.document(), "Orders").clone();
    let column = |name: &str| {
        orders
            .columns
            .iter()
            .find(|column| column.name == name)
            .unwrap()
            .id
            .clone()
    };
    let quantity_id = column("Quantity");

    let styled = store
        .apply(Operation::SetFrameStyleRules {
            frame_id: orders_id.clone(),
            rules: vec![
                FrameStyleRuleInput {
                    id: None,
                    formula: "when(`Quantity` > 2).then(\"busy\").otherwise(\"quiet\")".into(),
                    column_id: None,
                    output: FrameStyleOutput::Category {
                        cases: vec![FrameStyleCase {
                            value: "busy".into(),
                            style: FrameCellStyle {
                                text_color: Some("#315cbb".into()),
                                fill_color: Some("#fff0c7".into()),
                                ..FrameCellStyle::default()
                            },
                        }],
                        other: None,
                    },
                },
                FrameStyleRuleInput {
                    id: None,
                    // The position, not the value: a ramp reads what the
                    // formula computes, so the range lives here.
                    formula: "`Quantity`.normalize(0, 10)".into(),
                    column_id: Some(quantity_id.clone()),
                    output: FrameStyleOutput::Scale {
                        scale: FrameStyleScale {
                            // Text and fill are separate dimensions: ink is
                            // a two-colour blue-to-magenta ramp while the
                            // background turns red-yellow-green.
                            text: Some(FrameStyleColorScale {
                                low: "#0000ff".into(),
                                high: "#ff00ff".into(),
                                mid: None,
                            }),
                            fill: Some(FrameStyleColorScale {
                                low: "#ff0000".into(),
                                high: "#00ff00".into(),
                                mid: Some("#ffff00".into()),
                            }),
                        },
                    },
                },
            ],
        })
        .unwrap();
    let computed = &styled.computed_frames[&orders_id];
    assert!(
        computed.style_rule_errors.is_empty(),
        "{:?}",
        computed.style_rule_errors
    );

    let quantity_of = |row: &Row| {
        row.cells
            .get(&quantity_id)
            .and_then(|cell| cell.raw.parse::<f64>().ok())
            .unwrap()
    };
    for row in &orders.rows {
        let matched = computed.style_matches.get(&row.id).unwrap();
        let quantity = quantity_of(row);
        // The ramp answers for every row; the case list only for "busy".
        assert_eq!(matched.len(), if quantity > 2.0 { 2 } else { 1 });
        let ramp = matched.last().unwrap();
        let position = (quantity / 10.0).clamp(0.0, 1.0);
        let expected_text = format!("#{:02x}00ff", (position * 255.0).round() as u8);
        let expected_fill = if position <= 0.5 {
            format!("#ff{:02x}00", (position * 2.0 * 255.0).round() as u8)
        } else {
            format!(
                "#{:02x}ff00",
                ((1.0 - position) * 2.0 * 255.0).round() as u8
            )
        };
        assert_eq!(
            ramp.style.text_color.as_deref(),
            Some(expected_text.as_str())
        );
        assert_eq!(
            ramp.style.fill_color.as_deref(),
            Some(expected_fill.as_str())
        );
        if quantity > 2.0 {
            assert_eq!(matched[0].style.text_color.as_deref(), Some("#315cbb"));
            assert_eq!(matched[0].style.fill_color.as_deref(), Some("#fff0c7"));
        }
    }
}

/// The formulas the Rules panel's presets write, run against a real frame.
///
/// A preset is only worth offering if the engine takes it, and two of these
/// lean on the placement of the hidden column: `mean()` inside a row-wise
/// rule is an aggregate broadcast back across the rows, which is only the
/// column's average because the rule runs over the column.
#[test]
fn conditional_formatting_preset_formulas_are_accepted_and_mean_what_they_say() {
    let mut store = demo_store();
    let orders_id = frame_id(&store, "Orders");
    let orders = frame_named(store.document(), "Orders").clone();
    let quantity_id = orders
        .columns
        .iter()
        .find(|column| column.name == "Quantity")
        .unwrap()
        .id
        .clone();
    let quantities: Vec<f64> = orders
        .rows
        .iter()
        .map(|row| row.cells[&quantity_id].raw.parse::<f64>().unwrap())
        .collect();
    let average = quantities.iter().sum::<f64>() / quantities.len() as f64;

    let highlight = FrameStyleOutput::Condition {
        style: FrameCellStyle {
            fill_color: Some("#fff0c7".into()),
            ..FrameCellStyle::default()
        },
    };
    let view = store
        .apply(Operation::SetFrameStyleRules {
            frame_id: orders_id.clone(),
            rules: vec![
                FrameStyleRuleInput {
                    id: None,
                    formula: "`Quantity` > `Quantity`.mean()".into(),
                    column_id: Some(quantity_id.clone()),
                    output: highlight.clone(),
                },
                FrameStyleRuleInput {
                    id: None,
                    formula: "`Quantity`.is_null()".into(),
                    column_id: Some(quantity_id.clone()),
                    output: highlight.clone(),
                },
            ],
        })
        .unwrap();
    let computed = &view.computed_frames[&orders_id];
    assert!(
        computed.style_rule_errors.is_empty(),
        "{:?}",
        computed.style_rule_errors
    );

    let above = store
        .document()
        .frame(&orders_id)
        .unwrap()
        .display
        .style_rules[0]
        .id
        .clone();
    for (row, quantity) in orders.rows.iter().zip(&quantities) {
        let matched = computed
            .style_matches
            .get(&row.id)
            .map(|matches| matches.iter().any(|entry| entry.rule_id == above))
            .unwrap_or(false);
        assert_eq!(
            matched,
            *quantity > average,
            "row with quantity {quantity} against the column average {average}"
        );
    }
    // No blanks in the demo, so the second preset styles nothing — and says
    // nothing, rather than erroring.
    assert!(
        computed
            .style_matches
            .values()
            .all(|matches| matches.len() <= 1)
    );
}

/// What a category rule's case list is filled from: the values the formula
/// actually answers, commonest first.
///
/// This is the whole reason "a color per value" arrives finished rather than
/// empty. The panel cannot work these out — the formula may span columns and
/// the rows may live in a file it has never read — so it asks, and dresses
/// the answer.
#[test]
fn the_values_a_rule_would_sort_rows_into_come_back_commonest_first() {
    let mut store = demo_store();
    let products_id = frame_id(&store, "Products");

    // Accessories and Home office have two rows each, Stationery one. Ties
    // break on the label, so the list is the same list for everybody who
    // asks — a case list is document state, and one settled by hash order is
    // a merge conflict waiting to happen.
    assert_eq!(
        store
            .frame_formula_values(&products_id, "`Category`", 10)
            .unwrap(),
        vec!["Accessories", "Home office", "Stationery"]
    );
    // A cap drops the rarest, never the commonest.
    assert_eq!(
        store
            .frame_formula_values(&products_id, "`Category`", 2)
            .unwrap(),
        vec!["Accessories", "Home office"]
    );
    // An expression, not just a column -- the rule's formula is an ordinary
    // row-wise formula and this has to answer for whatever it is.
    assert_eq!(
        store
            .frame_formula_values(
                &products_id,
                "when(`List price` > 15).then(\"dear\").otherwise(\"cheap\")",
                10
            )
            .unwrap(),
        vec!["cheap", "dear"]
    );
    // The rules run over the display layer, so the values offered are the
    // values on screen. A filter that hides every stationery row hides
    // "Stationery" from the list rather than offering a case that can never
    // paint anything.
    store
        .apply(Operation::SetFrameDisplayFilter {
            frame_id: products_id.clone(),
            filters: vec!["`Category` != \"Stationery\"".into()],
            filter_match_all: true,
        })
        .unwrap();
    assert_eq!(
        store
            .frame_formula_values(&products_id, "`Category`", 10)
            .unwrap(),
        vec!["Accessories", "Home office"]
    );

    // A formula that answers something other than text has no values to
    // list, and says so rather than handing back numbers as labels.
    assert!(
        store
            .frame_formula_values(&products_id, "`List price`", 10)
            .is_err()
    );
}

/// A ramp paints the position its formula hands it, and the formula is where
/// every variation of a scale lives.
///
/// This is the whole shape of the reading: the rule carries three colors, at
/// 0, 0.5 and 1, and nothing else. `.normalize()` spreads a column across
/// them; `.normalize(center=0)` puts zero at the turn, which is the case a
/// middle exists for — a column of profits running -100 to 300 has its
/// midpoint at 100, so a three-color ramp without that would paint half the
/// *profits* in the loss color. Pinning ends, flattening outliers and
/// standing another column's value in for the real one are the same kind of
/// edit, and not one of them needed a control on the rule.
#[test]
fn a_ramp_paints_the_position_its_formula_computes() {
    let mut store = demo_store();
    let orders_id = frame_id(&store, "Orders");
    let orders = frame_named(store.document(), "Orders").clone();
    let quantity_id = orders
        .columns
        .iter()
        .find(|column| column.name == "Quantity")
        .unwrap()
        .id
        .clone();
    // Quantities are 3, 5 and 2, in that row order.
    let ramp = |mid: Option<&str>| FrameStyleOutput::Scale {
        scale: FrameStyleScale {
            text: None,
            fill: Some(FrameStyleColorScale {
                low: "#000000".into(),
                high: "#ffffff".into(),
                mid: mid.map(str::to_string),
            }),
        },
    };
    let fills = |store: &mut Store, formula: &str, output: FrameStyleOutput| {
        let view = store
            .apply(Operation::SetFrameStyleRules {
                frame_id: orders_id.clone(),
                rules: vec![FrameStyleRuleInput {
                    id: None,
                    formula: formula.into(),
                    column_id: Some(quantity_id.clone()),
                    output,
                }],
            })
            .unwrap_or_else(|error| panic!("{formula}: {error}"));
        let computed = &view.computed_frames[&orders_id];
        assert!(
            computed.style_rule_errors.is_empty(),
            "{formula}: {:?}",
            computed.style_rule_errors
        );
        orders
            .rows
            .iter()
            .map(|row| {
                (
                    row.cells[&quantity_id].raw.clone(),
                    computed.style_matches[&row.id][0]
                        .style
                        .fill_color
                        .clone()
                        .unwrap(),
                )
            })
            .collect::<Vec<_>>()
    };
    let grey = |fraction: f64| format!("#{0:02x}{0:02x}{0:02x}", (fraction * 255.0).round() as u8);

    // Spread across the column: 2 at the floor, 5 at the ceiling, 3 a third
    // of the way up. The aggregates behind that are inside the formula and
    // still see every row, because the hidden column goes in above the page.
    let spread = fills(&mut store, "`Quantity`.normalize()", ramp(None));
    assert_eq!(spread[1], ("5".into(), "#ffffff".into()));
    assert_eq!(spread[2], ("2".into(), "#000000".into()));
    assert_eq!(spread[0].1, grey(1.0 / 3.0));

    // Pinned ends are an edit to the formula and the rule never hears about
    // it: over 0..10, five is halfway rather than at the top.
    let pinned = fills(&mut store, "`Quantity`.normalize(0, 10)", ramp(None));
    assert_eq!(pinned[1].1, grey(0.5));
    assert_eq!(pinned[2].1, grey(0.2));

    // A range narrower than the data flattens what falls outside it, which
    // is a choice somebody made by writing those numbers rather than a
    // failure to notice.
    let clipped = fills(&mut store, "`Quantity`.normalize(0, 3)", ramp(None));
    assert_eq!(clipped[0], ("3".into(), "#ffffff".into()));
    assert_eq!(clipped[1], ("5".into(), "#ffffff".into()));

    // A centred ramp turns on the number it names. The furthest quantity
    // from three is two away, so three lands on the middle color and five at
    // the top.
    let centred = fills(
        &mut store,
        "`Quantity`.normalize(center=3)",
        ramp(Some("#ff0000")),
    );
    assert_eq!(centred[0], ("3".into(), "#ff0000".into()));
    assert_eq!(centred[1], ("5".into(), "#ffffff".into()));

    // And the case a scale could not express at all while its ends lived on
    // the rule: another column's answer standing in for the real value,
    // then spread. Every quantity over four reports as a hundred, so the
    // column runs 2..100 and the two ordinary rows crowd at the bottom.
    let substituted = fills(
        &mut store,
        "when(`Quantity` > 4).then(100).otherwise(`Quantity`).normalize()",
        ramp(None),
    );
    assert_eq!(substituted[1], ("5".into(), "#ffffff".into()));
    assert_eq!(substituted[2], ("2".into(), "#000000".into()));
    assert_eq!(substituted[0].1, grey(1.0 / 98.0));
}

/// Several columns collapsing into one label, which is what a category rule
/// is usually about.
///
/// A customer is a key account, or one in the north, or neither — two
/// columns and one answer — and the rule paints the answer. Chained branches
/// are what make that readable: nesting each one inside the last one's
/// `otherwise` says the same thing inside out, and gets worse with every
/// category. The panel then fills the case list from exactly these labels.
#[test]
fn chained_branches_collapse_several_columns_into_the_category_a_rule_paints() {
    let mut store = demo_store();
    let customers_id = frame_id(&store, "Customers");
    let customers = frame_named(store.document(), "Customers").clone();
    let name_id = customers
        .columns
        .iter()
        .find(|column| column.name == "Customer")
        .unwrap()
        .id
        .clone();

    let account = "when(`Segment` == \"Enterprise\").then(\"Key\")\
                   .when(`Region code` == \"AB-N\").then(\"North\")\
                   .otherwise(\"Other\")";
    // Commonest first, ties on the label -- and these are the three values a
    // case list gets filled with, rather than the five customers or the
    // three segments either column holds on its own.
    assert_eq!(
        store
            .frame_formula_values(&customers_id, account, 10)
            .unwrap(),
        vec!["Key", "Other", "North"]
    );

    let paint = |color: &str| FrameCellStyle {
        fill_color: Some(color.into()),
        ..FrameCellStyle::default()
    };
    let styled = store
        .apply(Operation::SetFrameStyleRules {
            frame_id: customers_id.clone(),
            rules: vec![FrameStyleRuleInput {
                id: None,
                formula: account.into(),
                column_id: Some(name_id.clone()),
                output: FrameStyleOutput::Category {
                    cases: vec![
                        FrameStyleCase {
                            value: "Key".into(),
                            style: paint("#dce9df"),
                        },
                        FrameStyleCase {
                            value: "North".into(),
                            style: paint("#f6ecc8"),
                        },
                    ],
                    // Unseen values land here rather than going unpainted,
                    // which is what a refreshing CSV needs: a segment nobody
                    // has seen yet reads as "not one of the named ones"
                    // instead of as a rule that stopped working.
                    other: Some(paint("#e6e4dc")),
                },
            }],
        })
        .unwrap();
    let computed = &styled.computed_frames[&customers_id];
    assert!(
        computed.style_rule_errors.is_empty(),
        "{:?}",
        computed.style_rule_errors
    );
    // Order decides: Coastal Labs is Enterprise *and* not AB-N, and Aurora
    // Market is AB-N but not Enterprise -- the first branch that answers
    // true owns the row.
    let painted: Vec<(String, String)> = customers
        .rows
        .iter()
        .map(|row| {
            (
                row.cells[&name_id].raw.clone(),
                computed.style_matches[&row.id][0]
                    .style
                    .fill_color
                    .clone()
                    .unwrap(),
            )
        })
        .collect();
    assert_eq!(
        painted,
        vec![
            ("Northwind Studio".to_string(), "#dce9df".to_string()),
            ("Prairie Goods".to_string(), "#e6e4dc".to_string()),
            ("Coastal Labs".to_string(), "#dce9df".to_string()),
            ("Aurora Market".to_string(), "#f6ecc8".to_string()),
            ("Summit Supply".to_string(), "#e6e4dc".to_string()),
        ]
    );
}

/// A rule reads the whole row and paints one column of it: the answer to
/// "colour the day of the week by whether it is a weekend".
///
/// The formula's scope and the rule's scope are two different things, and
/// keeping them apart is what makes a rule about one column able to say
/// something about another. Nothing about the painted column has to be
/// involved in the question.
#[test]
fn a_rule_answers_from_one_column_and_paints_a_different_one() {
    let mut store = demo_store();
    let products_id = frame_id(&store, "Products");
    let products = frame_named(store.document(), "Products").clone();
    let column = |name: &str| {
        products
            .columns
            .iter()
            .find(|column| column.name == name)
            .unwrap()
            .id
            .clone()
    };
    let product_id = column("Product");

    let view = store
        .apply(Operation::SetFrameStyleRules {
            frame_id: products_id.clone(),
            rules: vec![FrameStyleRuleInput {
                id: None,
                // Asks about the price; paints the name.
                formula: "`List price` > 15".into(),
                column_id: Some(product_id.clone()),
                output: FrameStyleOutput::Condition {
                    style: FrameCellStyle {
                        bold: Some(true),
                        ..FrameCellStyle::default()
                    },
                },
            }],
        })
        .unwrap();
    let computed = &view.computed_frames[&products_id];
    assert!(
        computed.style_rule_errors.is_empty(),
        "{:?}",
        computed.style_rule_errors
    );
    let rule_id = store
        .document()
        .frame(&products_id)
        .unwrap()
        .display
        .style_rules[0]
        .id
        .clone();
    // Desk lamp at $28 and Travel mug at $19 are the two over fifteen, and
    // the match is carried against the rule so the interface knows it may
    // only reach the Product column with it.
    let painted: Vec<String> = products
        .rows
        .iter()
        .filter(|row| {
            computed
                .style_matches
                .get(&row.id)
                .is_some_and(|matches| matches.iter().any(|entry| entry.rule_id == rule_id))
        })
        .map(|row| row.cells[&product_id].raw.clone())
        .collect();
    assert_eq!(painted, vec!["Desk lamp", "Travel mug"]);
    assert_eq!(
        store
            .document()
            .frame(&products_id)
            .unwrap()
            .display
            .style_rules[0]
            .column_id,
        Some(product_id)
    );
}

/// Every preset formula the Rules panel writes, run against a real frame —
/// including the ones over booleans and dates, whose columns have to be made
/// first because the demo has none.
///
/// A preset that does not compile is worse than a missing one: it is a menu
/// item that puts a red line under a rule nobody typed. The panel cannot
/// check this — only the engine knows what a formula returns — so the check
/// lives here, one commit per preset.
#[test]
fn every_preset_formula_the_panel_offers_is_one_the_engine_takes() {
    let mut store = demo_store();
    let orders_id = frame_id(&store, "Orders");
    // A boolean and a date to point the type-specific presets at. Written as
    // a wrangle step, which is the one authoring surface for a calculated
    // column, so these are columns like any other by the time a rule sees
    // them.
    store
        .apply(Operation::SetFramePipeline {
            frame_id: orders_id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![
                    ExistingFormulaInput {
                        output_column_id: column_id("Bulk"),
                        name: "Bulk".into(),
                        formula: "`Quantity` > 2".into(),
                    },
                    ExistingFormulaInput {
                        output_column_id: column_id("Ordered"),
                        name: "Ordered".into(),
                        formula: "date(2026, 7, 2)".into(),
                    },
                ],
            }],
        })
        .unwrap();

    let fill = FrameStyleOutput::Condition {
        style: FrameCellStyle {
            fill_color: Some("#fff0c7".into()),
            ..FrameCellStyle::default()
        },
    };
    let ramp = |mid: Option<String>| FrameStyleOutput::Scale {
        scale: FrameStyleScale {
            text: None,
            fill: Some(FrameStyleColorScale {
                low: "#ffffff".into(),
                high: "#8da293".into(),
                mid,
            }),
        },
    };
    let by_value = FrameStyleOutput::Category {
        cases: Vec::new(),
        other: Some(FrameCellStyle {
            fill_color: Some("#fff0c7".into()),
            ..FrameCellStyle::default()
        }),
    };
    // Exactly what `stylePresets` in src/lib/conditionalFormatting.ts writes,
    // preset by preset, with the reading each one declares.
    let presets: Vec<(&str, String, FrameStyleOutput)> = vec![
        ("heatmap", "`Quantity`.normalize()".into(), ramp(None)),
        (
            "diverging",
            "`Quantity`.normalize(center=0)".into(),
            ramp(Some("#f6f3ec".into())),
        ),
        (
            "above-average",
            "`Quantity` > `Quantity`.mean()".into(),
            fill.clone(),
        ),
        (
            "top-tenth",
            "`Quantity` >= `Quantity`.quantile(0.9)".into(),
            fill.clone(),
        ),
        (
            "bottom-tenth",
            "`Quantity` <= `Quantity`.quantile(0.1)".into(),
            fill.clone(),
        ),
        ("negative", "`Quantity` < 0".into(), fill.clone()),
        ("true", "`Bulk`".into(), fill.clone()),
        ("false", "`Bulk`.not()".into(), fill.clone()),
        (
            "weekends",
            "`Ordered`.dt.weekday() > 5".into(),
            fill.clone(),
        ),
        ("future", "`Ordered` > today()".into(), fill.clone()),
        (
            "stale",
            "`Ordered` < today().dt.offset_by(\"-30d\")".into(),
            fill.clone(),
        ),
        ("blanks", "`Quantity`.is_null()".into(), fill.clone()),
    ];
    for (name, formula, output) in presets {
        let view = store
            .apply(Operation::SetFrameStyleRules {
                frame_id: orders_id.clone(),
                rules: vec![FrameStyleRuleInput {
                    id: None,
                    formula: formula.clone(),
                    column_id: None,
                    output,
                }],
            })
            .unwrap_or_else(|error| panic!("preset {name} wrote `{formula}`: {error}"));
        let computed = &view.computed_frames[&orders_id];
        // Accepted is not enough: a rule can be typed correctly and still
        // fail to run, and a preset that reports itself broken on the row it
        // just created is the same broken menu item wearing an error.
        assert!(
            computed.style_rule_errors.is_empty(),
            "preset {name} wrote `{formula}`: {:?}",
            computed.style_rule_errors
        );
    }

    // The one reading a preset cannot arrive finished in: a case list is
    // filled from the values the engine reports, so the preset writes the
    // formula and the panel asks what it answers.
    assert!(
        store
            .apply(Operation::SetFrameStyleRules {
                frame_id: orders_id.clone(),
                rules: vec![FrameStyleRuleInput {
                    id: None,
                    formula: "`Bulk`.cast(\"string\")".into(),
                    column_id: None,
                    output: by_value,
                }],
            })
            .is_ok()
    );
    assert_eq!(
        store
            .frame_formula_values(&orders_id, "`Bulk`.cast(\"string\")", 10)
            .unwrap(),
        vec!["true", "false"]
    );
}
