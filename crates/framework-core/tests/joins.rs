use crate::common::*;
use framework_core::*;

#[test]
fn join_diagnostics_scan_generated_rows_beyond_any_rendered_page() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Join diagnostics".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
        scenarios: Vec::new(),
        active_scenario: None,
        calendars: Vec::new(),
        default_calendar_id: None,
    });
    for (name, stop) in [("Orders", 2001), ("Customers", 1501)] {
        store
            .apply(Operation::AddGeneratorFrame {
                name: name.into(),
                formula: format!("sequence(1, {stop})"),
                column_name: Some("Customer ID".into()),
                x: 0.0,
                y: 0.0,
            })
            .unwrap();
    }
    let orders = frame_named(store.document(), "Orders");
    let customers = frame_named(store.document(), "Customers");
    let diagnostics = store
        .get_join_diagnostics(
            &orders.id,
            &customers.id,
            &[orders.columns[0].id.clone()],
            &[customers.columns[0].id.clone()],
        )
        .unwrap();

    assert_eq!(diagnostics.primary_rows, 2000);
    assert_eq!(diagnostics.matched_rows, 1500);
    assert_eq!(diagnostics.unmatched_rows, 500);
    assert_eq!(diagnostics.lookup_rows, 1500);
    assert_eq!(diagnostics.lookup_null_key_rows, 0);
    assert_eq!(diagnostics.lookup_duplicate_key_values, 0);
}

#[test]
fn join_diagnostics_separate_blank_and_duplicate_lookup_keys() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Join diagnostics".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
        scenarios: Vec::new(),
        active_scenario: None,
        calendars: Vec::new(),
        default_calendar_id: None,
    });
    store
        .apply(Operation::AddFrame {
            name: "Orders".into(),
            grid: vec![
                vec!["Customer ID".into()],
                vec!["A".into()],
                vec!["B".into()],
                vec!["C".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::AddFrame {
            name: "Customers".into(),
            grid: vec![
                vec!["Customer ID".into()],
                vec!["A".into()],
                vec!["A".into()],
                vec![String::new()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let orders = frame_named(store.document(), "Orders");
    let customers = frame_named(store.document(), "Customers");
    let diagnostics = store
        .get_join_diagnostics(
            &orders.id,
            &customers.id,
            &[orders.columns[0].id.clone()],
            &[customers.columns[0].id.clone()],
        )
        .unwrap();

    assert_eq!(diagnostics.matched_rows, 1);
    assert_eq!(diagnostics.unmatched_rows, 2);
    assert_eq!(diagnostics.lookup_null_key_rows, 1);
    assert_eq!(diagnostics.lookup_duplicate_key_values, 1);
}

#[test]
fn a_join_relationship_can_change_keys_without_replacing_its_outputs() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Editable join".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
        scenarios: Vec::new(),
        active_scenario: None,
        calendars: Vec::new(),
        default_calendar_id: None,
    });
    store
        .apply(Operation::AddFrame {
            name: "Orders".into(),
            grid: vec![
                vec!["Old key".into(), "New key".into()],
                vec!["A".into(), "1".into()],
                vec!["B".into(), "2".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::AddFrame {
            name: "Lookup".into(),
            grid: vec![
                vec!["Old key".into(), "New key".into(), "Label".into()],
                vec!["A".into(), "1".into(), "Alpha".into()],
                vec!["X".into(), "2".into(), "Beta".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let primary = frame_named(store.document(), "Orders").clone();
    let lookup = frame_named(store.document(), "Lookup").clone();
    for key in [&lookup.columns[0], &lookup.columns[1]] {
        store
            .apply(Operation::SetUniqueKey {
                frame_id: lookup.id.clone(),
                column_ids: vec![key.id.clone()],
                enabled: true,
            })
            .unwrap();
    }
    store
        .apply(Operation::AddJoinFrame {
            primary_frame_id: primary.id.clone(),
            lookup_frame_id: lookup.id.clone(),
            primary_key_column_ids: vec![primary.columns[0].id.clone()],
            lookup_key_column_ids: vec![lookup.columns[0].id.clone()],
            join_type: FrameJoinType::Left,
            columns: vec![JoinColumnInput {
                source_frame_id: lookup.id.clone(),
                source_column_id: lookup.columns[2].id.clone(),
                name: "Label".into(),
            }],
            name: "Orders looked up".into(),
            x: 700.0,
            y: 0.0,
        })
        .unwrap();
    let joined = frame_named(store.document(), "Orders looked up").clone();
    let output_id = joined.columns[0].id.clone();

    store
        .apply(Operation::SetFrameJoinKeys {
            frame_id: joined.id.clone(),
            primary_key_column_ids: vec![primary.columns[1].id.clone()],
            lookup_key_column_ids: vec![lookup.columns[1].id.clone()],
        })
        .unwrap();

    let view = store.view();
    let joined_after = view.document.frame(&joined.id).unwrap();
    assert_eq!(joined_after.columns[0].id, output_id);
    assert_eq!(
        view.computed_frames[&joined.id].rows[&joined_after.rows[1].id][&output_id].typed_value,
        ScalarValue::String("Beta".into())
    );
}

#[test]
fn joined_frames_require_unique_lookup_keys_and_refresh_both_inputs() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Joins".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
        scenarios: Vec::new(),
        active_scenario: None,
        calendars: Vec::new(),
        default_calendar_id: None,
    });
    store
        .apply(Operation::AddFrame {
            name: "Orders".into(),
            grid: vec![
                vec!["Order ID".into(), "Customer ID".into()],
                vec!["O-1".into(), "C-1".into()],
                vec!["O-2".into(), "C-2".into()],
                vec!["O-3".into(), "C-9".into()],
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
                vec!["C-2".into(), "Grace".into()],
            ],
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let (orders_id, order_id, order_customer_id) = {
        let frame = frame_named(store.document(), "Orders");
        (
            frame.id.clone(),
            frame.columns[0].id.clone(),
            frame.columns[1].id.clone(),
        )
    };
    let (customers_id, customer_id, customer_name, second_customer_row) = {
        let frame = frame_named(store.document(), "Customers");
        (
            frame.id.clone(),
            frame.columns[0].id.clone(),
            frame.columns[1].id.clone(),
            frame.rows[1].id.clone(),
        )
    };

    assert!(matches!(
        store.apply(Operation::AddJoinFrame {
            primary_frame_id: orders_id.clone(),
            lookup_frame_id: customers_id.clone(),
            primary_key_column_ids: vec![order_customer_id.clone()],
            lookup_key_column_ids: vec![customer_id.clone()],
            join_type: FrameJoinType::Left,
            columns: Vec::new(),
            name: "Invalid".into(),
            x: 600.0,
            y: 0.0,
        }),
        Err(CoreError::InvalidOperation(_))
    ));

    store
        .apply(Operation::SetUniqueKey {
            frame_id: customers_id.clone(),
            column_ids: vec![customer_id.clone()],
            enabled: true,
        })
        .unwrap();
    store
        .apply(Operation::AddJoinFrame {
            primary_frame_id: orders_id.clone(),
            lookup_frame_id: customers_id.clone(),
            primary_key_column_ids: vec![order_customer_id.clone()],
            lookup_key_column_ids: vec![customer_id.clone()],
            join_type: FrameJoinType::Left,
            columns: vec![
                JoinColumnInput {
                    source_frame_id: orders_id.clone(),
                    source_column_id: order_id.clone(),
                    name: "Order ID".into(),
                },
                JoinColumnInput {
                    source_frame_id: customers_id.clone(),
                    source_column_id: customer_name.clone(),
                    name: "Customer name".into(),
                },
            ],
            name: "Orders with customers".into(),
            x: 700.0,
            y: 0.0,
        })
        .unwrap();

    let joined = frame_named(store.document(), "Orders with customers");
    let joined_id = joined.id.clone();
    let name_output = joined.columns[1].id.clone();
    let view = store.view();
    let joined = view.document.frame(&joined_id).unwrap();
    assert_eq!(joined.rows.len(), 3);
    assert_eq!(
        view.computed_frames[&joined_id].rows[&joined.rows[0].id][&name_output].typed_value,
        ScalarValue::String("Ada".into())
    );
    assert_eq!(
        view.computed_frames[&joined_id].rows[&joined.rows[2].id][&name_output].typed_value,
        ScalarValue::Null
    );

    store
        .apply(Operation::SetCell {
            frame_id: customers_id.clone(),
            row_id: second_customer_row.clone(),
            column_id: customer_name,
            raw: "Hopper".into(),
        })
        .unwrap();
    let refreshed = store.view();
    let joined = refreshed.document.frame(&joined_id).unwrap();
    assert_eq!(
        refreshed.computed_frames[&joined_id].rows[&joined.rows[1].id][&name_output].typed_value,
        ScalarValue::String("Hopper".into())
    );

    let match_flag = column_id("Match flag");
    store
        .apply(Operation::SetFramePipeline {
            frame_id: joined_id.clone(),
            steps: vec![
                FrameStepInput::Filter {
                    predicates: vec!["`Customer name` == \"Ada\"".into()],
                    match_all: true,
                },
                FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: match_flag.clone(),
                        name: "Match flag".into(),
                        formula: "1 + 1".into(),
                    }],
                },
            ],
        })
        .unwrap();
    let transformed = store.view();
    let joined = transformed.document.frame(&joined_id).unwrap();
    assert_eq!(joined.rows.len(), 1);
    assert_eq!(joined.base_columns.len(), 2);
    assert!(joined.derivation.as_ref().unwrap().join.is_some());
    assert_eq!(joined.derivation.as_ref().unwrap().steps().len(), 3);
    assert_eq!(
        transformed.computed_frames[&joined_id].rows[&joined.rows[0].id][&match_flag].typed_value,
        ScalarValue::Number(2.0)
    );

    // Replacing the editable chain must retain exactly one fixed join and
    // start again from its full result, not from the previous filter's
    // output or the primary frame's pre-join schema.
    store
        .apply(Operation::SetFramePipeline {
            frame_id: joined_id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: match_flag,
                    name: "Match flag".into(),
                    formula: "1 + 2".into(),
                }],
            }],
        })
        .unwrap();
    let transformed = store.view();
    let joined = transformed.document.frame(&joined_id).unwrap();
    assert_eq!(joined.rows.len(), 3);
    assert_eq!(
        joined
            .derivation
            .as_ref()
            .unwrap()
            .steps()
            .iter()
            .filter(|step| matches!(step, FrameStep::Join { .. }))
            .count(),
        1
    );

    store
        .apply(Operation::AddJoinFrame {
            primary_frame_id: orders_id.clone(),
            lookup_frame_id: customers_id.clone(),
            primary_key_column_ids: vec![order_customer_id],
            lookup_key_column_ids: vec![customer_id.clone()],
            join_type: FrameJoinType::Inner,
            columns: vec![JoinColumnInput {
                source_frame_id: orders_id,
                source_column_id: order_id,
                name: "Order ID".into(),
            }],
            name: "Matched orders".into(),
            x: 700.0,
            y: 400.0,
        })
        .unwrap();
    assert_eq!(
        frame_named(&store.view().document, "Matched orders")
            .rows
            .len(),
        2
    );

    assert!(matches!(
        store.apply(Operation::SetCell {
            frame_id: customers_id.clone(),
            row_id: second_customer_row,
            column_id: customer_id.clone(),
            raw: "C-1".into(),
        }),
        Err(CoreError::InvalidOperation(message)) if message.contains("duplicates")
    ));
    assert!(matches!(
        store.apply(Operation::SetUniqueKey {
            frame_id: customers_id,
            column_ids: vec![customer_id],
            enabled: false,
        }),
        Err(CoreError::InvalidOperation(message)) if message.contains("unique key")
    ));
}

#[test]
fn anti_and_semi_joins_partition_rows_and_allow_duplicate_lookup_keys() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Membership joins".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
        scenarios: Vec::new(),
        active_scenario: None,
        calendars: Vec::new(),
        default_calendar_id: None,
    });
    store
        .apply(Operation::AddFrame {
            name: "Orders".into(),
            grid: vec![
                vec!["Order ID".into(), "Customer ID".into(), "Amount".into()],
                vec!["O-1".into(), "C-1".into(), "10".into()],
                vec!["O-2".into(), "C-2".into(), "20".into()],
                vec!["O-3".into(), "C-9".into(), "30".into()],
                vec!["O-4".into(), "".into(), "40".into()],
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
                vec!["C-1".into(), "Ada duplicate".into()],
                vec!["C-2".into(), "Grace".into()],
                vec!["".into(), "No key".into()],
            ],
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let (orders_id, order_id, order_customer_id, order_amount) = {
        let frame = frame_named(store.document(), "Orders");
        (
            frame.id.clone(),
            frame.columns[0].id.clone(),
            frame.columns[1].id.clone(),
            frame.columns[2].id.clone(),
        )
    };
    let (customers_id, customer_id) = {
        let frame = frame_named(store.document(), "Customers");
        (frame.id.clone(), frame.columns[0].id.clone())
    };

    assert!(matches!(
        store.apply(Operation::AddJoinFrame {
            primary_frame_id: orders_id.clone(),
            lookup_frame_id: customers_id.clone(),
            primary_key_column_ids: vec![order_customer_id.clone()],
            lookup_key_column_ids: vec![customer_id.clone()],
            join_type: FrameJoinType::Left,
            columns: vec![JoinColumnInput {
                source_frame_id: orders_id.clone(),
                source_column_id: order_id.clone(),
                name: "Order ID".into(),
            }],
            name: "Invalid left".into(),
            x: 600.0,
            y: 0.0,
        }),
        Err(CoreError::InvalidOperation(message)) if message.contains("unique key")
    ));

    store
        .apply(Operation::AddJoinFrame {
            primary_frame_id: orders_id.clone(),
            lookup_frame_id: customers_id.clone(),
            primary_key_column_ids: vec![order_customer_id.clone()],
            lookup_key_column_ids: vec![customer_id.clone()],
            join_type: FrameJoinType::Anti,
            columns: vec![
                JoinColumnInput {
                    source_frame_id: orders_id.clone(),
                    source_column_id: order_id.clone(),
                    name: "Order ID".into(),
                },
                JoinColumnInput {
                    source_frame_id: orders_id.clone(),
                    source_column_id: order_amount,
                    name: "Amount".into(),
                },
            ],
            name: "Unmatched orders".into(),
            x: 700.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::AddJoinFrame {
            primary_frame_id: orders_id.clone(),
            lookup_frame_id: customers_id.clone(),
            primary_key_column_ids: vec![order_customer_id.clone()],
            lookup_key_column_ids: vec![customer_id],
            join_type: FrameJoinType::Semi,
            columns: vec![JoinColumnInput {
                source_frame_id: orders_id.clone(),
                source_column_id: order_id,
                name: "Order ID".into(),
            }],
            name: "Matched orders".into(),
            x: 700.0,
            y: 400.0,
        })
        .unwrap();
    let anti_id = frame_named(store.document(), "Unmatched orders").id.clone();
    let semi_id = frame_named(store.document(), "Matched orders").id.clone();

    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: anti_id.clone(),
            name: "Unmatched total".into(),
            group_keys: Vec::new(),
            aggregates: vec![NamedFormulaInput {
                name: "Total".into(),
                formula: "`Amount`.sum()".into(),
            }],
            maintain_order: true,
            x: 900.0,
            y: 0.0,
        })
        .unwrap();
    let total_id = frame_named(store.document(), "Unmatched total").id.clone();

    let view = store.view();
    let anti = view.document.frame(&anti_id).unwrap();
    let anti_order_output = anti.columns[0].id.clone();
    assert_eq!(anti.rows.len(), 2);
    assert_eq!(
        view.computed_frames[&anti_id].rows[&anti.rows[0].id][&anti_order_output].typed_value,
        ScalarValue::String("O-3".into())
    );
    assert_eq!(
        view.computed_frames[&anti_id].rows[&anti.rows[1].id][&anti_order_output].typed_value,
        ScalarValue::String("O-4".into())
    );
    let semi = view.document.frame(&semi_id).unwrap();
    let semi_order_output = semi.columns[0].id.clone();
    assert_eq!(semi.rows.len(), 2);
    assert_eq!(
        view.computed_frames[&semi_id].rows[&semi.rows[0].id][&semi_order_output].typed_value,
        ScalarValue::String("O-1".into())
    );
    assert_eq!(
        view.computed_frames[&semi_id].rows[&semi.rows[1].id][&semi_order_output].typed_value,
        ScalarValue::String("O-2".into())
    );
    let total = view.document.frame(&total_id).unwrap();
    assert_eq!(
        view.computed_frames[&total_id].rows[&total.rows[0].id][&total.columns[0].id].value,
        Some(70.0)
    );

    // The new policies serialize cleanly and round-trip unchanged.
    let serialized = serde_json::to_string(store.document()).unwrap();
    assert!(serialized.contains("\"joinType\":\"anti\""));
    assert!(serialized.contains("\"joinType\":\"semi\""));
    let restored: Document = serde_json::from_str(&serialized).unwrap();
    assert_eq!(&restored, store.document());

    // Matching every order empties the anti result and fills the semi one.
    let (third_row, fourth_row) = {
        let orders = frame_named(store.document(), "Orders");
        (orders.rows[2].id.clone(), orders.rows[3].id.clone())
    };
    store
        .apply(Operation::SetCell {
            frame_id: orders_id.clone(),
            row_id: third_row,
            column_id: order_customer_id.clone(),
            raw: "C-2".into(),
        })
        .unwrap();
    store
        .apply(Operation::SetCell {
            frame_id: orders_id,
            row_id: fourth_row,
            column_id: order_customer_id,
            raw: "C-1".into(),
        })
        .unwrap();
    let refreshed = store.view();
    assert_eq!(refreshed.document.frame(&anti_id).unwrap().rows.len(), 0);
    assert_eq!(refreshed.document.frame(&semi_id).unwrap().rows.len(), 4);
    let total = refreshed.document.frame(&total_id).unwrap();
    assert_eq!(
        refreshed.computed_frames[&total_id].rows[&total.rows[0].id][&total.columns[0].id].value,
        Some(0.0)
    );
}

#[test]
fn a_join_matches_on_several_key_columns_together() {
    let mut store = Store::new(Document::demo());
    for (name, grid) in [
        (
            "Actuals",
            vec![
                vec!["Account", "Dept", "Amount"],
                vec!["4000", "East", "100"],
                vec!["4000", "West", "200"],
                vec!["5000", "East", "300"],
            ],
        ),
        (
            "Budget",
            vec![
                vec!["Account", "Dept", "Plan"],
                vec!["4000", "East", "90"],
                vec!["4000", "West", "210"],
                vec!["5000", "West", "999"],
            ],
        ),
    ] {
        store
            .apply(Operation::AddFrame {
                name: name.into(),
                grid: grid
                    .into_iter()
                    .map(|row| row.into_iter().map(String::from).collect())
                    .collect(),
                x: 0.0,
                y: 0.0,
            })
            .unwrap();
    }
    let actuals = crate::common::frame_named(store.document(), "Actuals").clone();
    let budget = crate::common::frame_named(store.document(), "Budget").clone();
    let id = |frame: &FrameObject, name: &str| {
        frame
            .columns
            .iter()
            .find(|c| c.name == name)
            .unwrap()
            .id
            .clone()
    };
    store
        .apply(Operation::SetUniqueKey {
            frame_id: budget.id.clone(),
            column_ids: vec![id(&budget, "Account"), id(&budget, "Dept")],
            enabled: true,
        })
        .unwrap();
    let join = |keys: (Vec<Id>, Vec<Id>)| Operation::AddJoinFrame {
        primary_frame_id: actuals.id.clone(),
        lookup_frame_id: budget.id.clone(),
        primary_key_column_ids: keys.0,
        lookup_key_column_ids: keys.1,
        join_type: FrameJoinType::Left,
        columns: vec![
            JoinColumnInput {
                source_frame_id: actuals.id.clone(),
                source_column_id: id(&actuals, "Dept"),
                name: "Dept".into(),
            },
            JoinColumnInput {
                source_frame_id: actuals.id.clone(),
                source_column_id: id(&actuals, "Amount"),
                name: "Amount".into(),
            },
            JoinColumnInput {
                source_frame_id: budget.id.clone(),
                source_column_id: id(&budget, "Plan"),
                name: "Plan".into(),
            },
        ],
        name: "Variance".into(),
        x: 0.0,
        y: 0.0,
    };
    // Pairs must line up.
    let lopsided = store
        .apply(join((
            vec![id(&actuals, "Account"), id(&actuals, "Dept")],
            vec![id(&budget, "Account")],
        )))
        .unwrap_err()
        .to_string();
    assert!(lopsided.contains("in pairs"), "{lopsided}");

    store
        .apply(join((
            vec![id(&actuals, "Account"), id(&actuals, "Dept")],
            vec![id(&budget, "Account"), id(&budget, "Dept")],
        )))
        .unwrap();
    let variance = crate::common::frame_named(store.document(), "Variance")
        .id
        .clone();
    let page = store.get_frame_page(&variance, 0, 10).unwrap();
    let plans: Vec<String> = page.rows.iter().map(|row| row[2].clone()).collect();
    // 4000/East → 90, 4000/West → 210, 5000/East → no budget line.
    assert_eq!(plans, ["90", "210", ""]);
}
