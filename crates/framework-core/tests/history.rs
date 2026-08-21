use crate::common::*;
use framework_core::*;

#[test]
fn undo_and_redo_restore_state() {
    let mut store = demo_store();
    let block_id = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Block(block) => Some(block.id.clone()),
            _ => None,
        })
        .unwrap();
    store
        .apply(Operation::SetBlockSource {
            block_id: block_id.clone(),
            source: "Tax rate = 0.10".into(),
            editing: None,
        })
        .unwrap();
    assert_eq!(store.undo().document.revision, 2);
    let redone = store.redo();
    assert_eq!(redone.document.revision, 3);
    assert_eq!(redone.computed_blocks[&block_id].source, "Tax rate = 0.10");
}

#[test]
fn deletions_are_dependency_safe_and_participate_in_history() {
    let mut store = demo_store();
    let (frame_id, quantity_id, total_id, first_row_id) = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) => Some((
                frame.id.clone(),
                frame.columns[0].id.clone(),
                frame.columns[2].id.clone(),
                frame.rows[0].id.clone(),
            )),
            _ => None,
        })
        .unwrap();
    let assumptions = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Block(block) => Some(block.id.clone()),
            _ => None,
        })
        .unwrap();

    assert!(matches!(
        store.apply(Operation::DeleteColumn {
            frame_id: frame_id.clone(),
            column_id: quantity_id,
        }),
        Err(CoreError::ReferencedByFormula(_))
    ));
    assert!(matches!(
        store.apply(Operation::DeleteObject {
            object_id: assumptions,
        }),
        Err(CoreError::ReferencedByFormula(_))
    ));

    let deleted = store
        .apply(Operation::DeleteColumn {
            frame_id: frame_id.clone(),
            column_id: total_id.clone(),
        })
        .unwrap();
    let frame = deleted
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.id == frame_id => Some(frame),
            _ => None,
        })
        .unwrap();
    assert_eq!(frame.columns.len(), 2);
    assert!(frame.summaries.is_empty());

    let restored = store.undo();
    let frame = restored
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.id == frame_id => Some(frame),
            _ => None,
        })
        .unwrap();
    assert_eq!(frame.columns.len(), 3);
    assert_eq!(frame.summaries.len(), 1);
    assert!(store.redo().document.objects.iter().any(|object| {
        matches!(object, DataObject::Frame(frame) if !frame.columns.iter().any(|column| column.id == total_id))
    }));

    store.undo();
    assert_eq!(
        store
            .apply(Operation::DeleteRow {
                frame_id: frame_id.clone(),
                row_id: first_row_id,
            })
            .unwrap()
            .document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.id == frame_id => Some(frame.rows.len()),
                _ => None,
            }),
        Some(2)
    );

    let deleted_frame = store
        .apply(Operation::DeleteObject {
            object_id: frame_id.clone(),
        })
        .unwrap();
    assert!(
        !deleted_frame
            .document
            .objects
            .iter()
            .any(|object| object.id() == frame_id)
    );
    assert!(
        !deleted_frame
            .document
            .views
            .iter()
            .any(|view| view.object_id == frame_id)
    );
}

#[test]
fn multi_cell_updates_are_atomic_and_use_one_history_entry() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Grid edits".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Input".into(),
            grid: vec![
                vec!["Name".into(), "Amount".into()],
                vec!["Alpha".into(), "10".into()],
                vec!["Beta".into(), "20".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Input");
    let frame_id = frame.id.clone();
    let name_column = frame.columns[0].id.clone();
    let amount_column = frame.columns[1].id.clone();
    let first_row = frame.rows[0].id.clone();
    let second_row = frame.rows[1].id.clone();

    store
        .apply(Operation::SetCells {
            frame_id: frame_id.clone(),
            cells: vec![
                CellUpdate {
                    row_id: first_row.clone(),
                    column_id: name_column.clone(),
                    raw: "Changed".into(),
                },
                CellUpdate {
                    row_id: second_row.clone(),
                    column_id: amount_column.clone(),
                    raw: "99".into(),
                },
            ],
        })
        .unwrap();
    let frame = frame_named(store.document(), "Input");
    assert_eq!(frame.rows[0].cells[&name_column].raw, "Changed");
    assert_eq!(frame.rows[1].cells[&amount_column].raw, "99");

    store.undo();
    let frame = frame_named(store.document(), "Input");
    assert_eq!(frame.rows[0].cells[&name_column].raw, "Alpha");
    assert_eq!(frame.rows[1].cells[&amount_column].raw, "20");

    let before = store.document().clone();
    assert!(matches!(
        store.apply(Operation::SetCells {
            frame_id,
            cells: vec![
                CellUpdate {
                    row_id: first_row,
                    column_id: name_column,
                    raw: "Nope".into()
                },
                CellUpdate {
                    row_id: "missing".into(),
                    column_id: amount_column,
                    raw: "0".into()
                },
            ],
        }),
        Err(CoreError::RowNotFound)
    ));
    assert_eq!(*store.document(), before);
}

#[test]
fn frame_orientation_is_view_state_with_history() {
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
    let _ = view_id;

    let oriented = store
        .apply(Operation::SetFrameDisplayOrientation {
            frame_id: frame_id.clone(),
            orientation: FrameViewOrientation::FieldsAsRows,
        })
        .unwrap();
    assert_eq!(
        oriented
            .document
            .frame(&frame_id)
            .unwrap()
            .display
            .orientation,
        FrameViewOrientation::FieldsAsRows
    );

    let undone = store.undo();
    assert_eq!(
        undone
            .document
            .frame(&frame_id)
            .unwrap()
            .display
            .orientation,
        FrameViewOrientation::RecordsAsRows
    );
}

#[test]
fn column_formats_are_display_only_typed_operations_with_history() {
    let mut store = Store::new(Document::demo());
    let frame = frame_named(store.document(), "Orders");
    let frame_id = frame.id.clone();
    let column_id = frame
        .columns
        .iter()
        .find(|column| column.name == "Total")
        .unwrap()
        .id
        .clone();
    let raw_cells_before: Vec<String> = frame
        .rows
        .iter()
        .map(|row| row.cells[&column_id].raw.clone())
        .collect();

    let format = ColumnFormat {
        style: ColumnFormatStyle::Accounting,
        decimals: Some(0),
        scale: ColumnFormatScale::Thousands,
        negative_parens: None,
        zero_dash: None,
        currency_code: Some(" usd ".into()),
    };
    let view = store
        .apply(Operation::SetColumnFormat {
            frame_id: frame_id.clone(),
            column_id: column_id.clone(),
            format: Some(format),
        })
        .unwrap();
    let column = view
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.id == frame_id => Some(frame),
            _ => None,
        })
        .unwrap()
        .columns
        .iter()
        .find(|column| column.id == column_id)
        .unwrap();
    let stored = column.format.clone().unwrap();
    assert_eq!(stored.style, ColumnFormatStyle::Accounting);
    assert_eq!(stored.decimals, Some(0));
    assert_eq!(stored.scale, ColumnFormatScale::Thousands);
    assert_eq!(stored.currency_code.as_deref(), Some("USD"));
    assert_eq!(stored.negative_parens, None);
    assert_eq!(stored.zero_dash, None);

    let raw_cells_after: Vec<String> = frame_named(store.document(), "Orders")
        .rows
        .iter()
        .map(|row| row.cells[&column_id].raw.clone())
        .collect();
    assert_eq!(raw_cells_before, raw_cells_after);

    let undone = store.undo();
    assert!(
        frame_named(&undone.document, "Orders")
            .columns
            .iter()
            .find(|column| column.id == column_id)
            .unwrap()
            .format
            .is_none()
    );
    let redone = store.redo();
    assert!(
        frame_named(&redone.document, "Orders")
            .columns
            .iter()
            .find(|column| column.id == column_id)
            .unwrap()
            .format
            .is_some()
    );

    let cleared = store
        .apply(Operation::SetColumnFormat {
            frame_id: frame_id.clone(),
            column_id: column_id.clone(),
            format: None,
        })
        .unwrap();
    assert!(
        frame_named(&cleared.document, "Orders")
            .columns
            .iter()
            .find(|column| column.id == column_id)
            .unwrap()
            .format
            .is_none()
    );

    assert!(matches!(
        store.apply(Operation::SetColumnFormat {
            frame_id,
            column_id: "missing-column".into(),
            format: None,
        }),
        Err(CoreError::ColumnNotFound)
    ));
}

#[test]
fn frame_styles_live_on_the_frame_and_are_undoable() {
    let mut store = demo_store();
    let frame = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) => Some(frame),
            _ => None,
        })
        .unwrap();
    let frame_id = frame.id.clone();
    let row_id = frame.rows[0].id.clone();
    let column_id = frame.columns[0].id.clone();
    let target = FrameStyleTarget::Cell { row_id, column_id };
    let style = FrameCellStyle {
        bold: Some(true),
        fill_color: Some("#fff2aa".into()),
        alignment: Some(FrameCellAlignment::Center),
        line_style: Some(FrameLineStyle::Dashed),
        ..FrameCellStyle::default()
    };

    let styled = store
        .apply(Operation::SetFrameStyle {
            frame_id: frame_id.clone(),
            target: target.clone(),
            style: style.clone(),
        })
        .unwrap();
    assert_eq!(
        styled.document.frame(&frame_id).unwrap().display.styles[0],
        FrameStyle {
            target: target.clone(),
            style: style.clone()
        }
    );

    let serialized = serde_json::to_string(&styled.document).unwrap();
    let restored: Document = serde_json::from_str(&serialized).unwrap();
    assert_eq!(
        restored.frame(&frame_id).unwrap().display.styles[0].style,
        style
    );

    let undone = store.undo();
    assert!(
        undone
            .document
            .frame(&frame_id)
            .unwrap()
            .display
            .styles
            .is_empty()
    );
}

#[test]
fn conditional_formatting_rules_are_typed_ordered_and_undoable() {
    let mut store = demo_store();
    let frame = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) => Some(frame),
            _ => None,
        })
        .unwrap();
    let frame_id = frame.id.clone();
    let column = frame
        .columns
        .iter()
        .find(|column| matches!(column.data_type, DataType::Integer | DataType::Number))
        .unwrap();
    let column_id = column.id.clone();
    let column_name = column.name.clone();
    let formula = format!("`{}` > 0", column_name.replace('`', "``"));
    let style = FrameCellStyle {
        bold: Some(true),
        fill_color: Some("#fff2aa".into()),
        ..FrameCellStyle::default()
    };

    let styled = store
        .apply(Operation::SetFrameStyleRules {
            frame_id: frame_id.clone(),
            rules: vec![FrameStyleRuleInput {
                id: None,
                formula,
                column_id: Some(column_id.clone()),
                output: FrameStyleOutput::Condition {
                    style: style.clone(),
                },
            }],
        })
        .unwrap();
    let rule = &styled
        .document
        .frame(&frame_id)
        .unwrap()
        .display
        .style_rules[0];
    assert!(!rule.id.is_empty());
    assert_eq!(rule.column_id.as_deref(), Some(column_id.as_str()));
    assert_eq!(
        rule.output,
        FrameStyleOutput::Condition {
            style: style.clone()
        }
    );

    let undone = store.undo();
    assert!(
        undone
            .document
            .frame(&frame_id)
            .unwrap()
            .display
            .style_rules
            .is_empty()
    );
}

/// The three readings of a rule's hidden column, each accepted only for the
/// type it can actually read.
#[test]
fn conditional_formatting_readings_must_match_what_the_formula_returns() {
    let mut store = demo_store();
    let frame = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) => Some(frame),
            _ => None,
        })
        .unwrap();
    let frame_id = frame.id.clone();
    let column = frame
        .columns
        .iter()
        .find(|column| matches!(column.data_type, DataType::Integer | DataType::Number))
        .unwrap();
    let number = format!("`{}`", column.name.replace('`', "``"));

    // A number is not a yes-or-no answer, so it cannot be read as one.
    let refused = store.apply(Operation::SetFrameStyleRules {
        frame_id: frame_id.clone(),
        rules: vec![FrameStyleRuleInput {
            id: None,
            formula: number.clone(),
            column_id: None,
            output: FrameStyleOutput::Condition {
                style: FrameCellStyle {
                    italic: Some(true),
                    ..FrameCellStyle::default()
                },
            },
        }],
    });
    assert!(matches!(refused, Err(CoreError::InvalidOperation(_))));

    // ...and the same number read as a ramp is accepted, while reading it
    // as a list of labels is not.
    let scaled = store
        .apply(Operation::SetFrameStyleRules {
            frame_id: frame_id.clone(),
            rules: vec![FrameStyleRuleInput {
                id: None,
                formula: number.clone(),
                column_id: None,
                output: FrameStyleOutput::Scale {
                    scale: FrameStyleScale {
                        text: None,
                        fill: Some(FrameStyleColorScale {
                            low: "#ffffff".into(),
                            high: "#315c49".into(),
                            mid: None,
                        }),
                    },
                },
            }],
        })
        .unwrap();
    assert!(matches!(
        scaled
            .document
            .frame(&frame_id)
            .unwrap()
            .display
            .style_rules[0]
            .output,
        FrameStyleOutput::Scale { .. }
    ));
    let refused = store.apply(Operation::SetFrameStyleRules {
        frame_id: frame_id.clone(),
        rules: vec![FrameStyleRuleInput {
            id: None,
            formula: number.clone(),
            column_id: None,
            output: FrameStyleOutput::Category {
                cases: vec![FrameStyleCase {
                    value: "7".into(),
                    style: FrameCellStyle {
                        italic: Some(true),
                        ..FrameCellStyle::default()
                    },
                }],
                other: None,
            },
        }],
    });
    assert!(matches!(refused, Err(CoreError::InvalidOperation(_))));
}
