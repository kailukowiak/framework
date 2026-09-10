use crate::common::*;
use framework_core::*;

fn dictionary(store: &mut Store) -> FrameObject {
    store
        .apply(Operation::AddDictionary {
            name: "Fixes".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Fixes").clone();
    store
        .apply(Operation::PasteCells {
            frame_id: frame.id.clone(),
            row_id: frame.rows[0].id.clone(),
            column_id: frame.columns[0].id.clone(),
            grid: vec![
                vec!["typo".into(), "Correct".into()],
                vec!["clear".into(), "".into()],
            ],
        })
        .unwrap();
    frame_named(store.document(), "Fixes").clone()
}

fn answers(store: &mut Store, source: &str) -> Vec<ComputedCell> {
    store
        .apply(Operation::AddBlock {
            name: "Checks".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Checks")
        .unwrap()
        .id()
        .to_string();
    store
        .apply(Operation::SetBlockSource {
            block_id: id.clone(),
            source: source.into(),
            editing: None,
        })
        .unwrap();
    store.view().computed_blocks[&id]
        .lines
        .iter()
        .map(|line| line.cell.clone())
        .collect()
}

#[test]
fn lookup_distinguishes_missing_keys_fallbacks_and_null_replacements() {
    let mut store = Store::new(Document::blank("Dictionaries"));
    dictionary(&mut store);
    let cells = answers(
        &mut store,
        "a = lookup(\"typo\", `Fixes`.`Key`, `Fixes`.`Value`)\nb = lookup(\"missing\", `Fixes`.`Key`, `Fixes`.`Value`, \"fallback\")\nc = lookup(\"missing\", `Fixes`.`Key`, `Fixes`.`Value`)\nd = map_values(\"missing\", `Fixes`.`Key`, `Fixes`.`Value`)\ne = map_values(\"clear\", `Fixes`.`Key`, `Fixes`.`Value`)",
    );
    assert_eq!(cells[0].display, "Correct");
    assert_eq!(cells[1].display, "fallback");
    assert!(cells[2].error.is_some());
    assert_eq!(cells[3].display, "missing");
    assert!(cells[4].error.is_none(), "{:?}", cells[4]);
    assert_eq!(cells[4].typed_value, ScalarValue::Null);
}

#[test]
fn mapped_columns_follow_dictionary_edits_and_undo_and_reject_duplicate_keys() {
    let mut store = Store::new(Document::blank("Dictionaries"));
    let dict = dictionary(&mut store);
    store
        .apply(Operation::AddFrame {
            name: "Sales".into(),
            grid: vec![
                vec!["Category".into()],
                vec!["typo".into()],
                vec!["other".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let source = frame_named(store.document(), "Sales").clone();
    store
        .apply(Operation::SetColumnCategories {
            frame_id: source.id.clone(),
            column_id: source.columns[0].id.clone(),
            categories: vec!["typo".into(), "other".into()],
        })
        .unwrap();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: source.id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: source.columns[0].id.clone(),
                    name: "Category".into(),
                    formula: "map_values(`Category`, `Fixes`.`Key`, `Fixes`.`Value`)".into(),
                }],
            }],
        })
        .unwrap();
    let values = |store: &Store| store.get_frame_page(&source.id, 0, 100).unwrap().rows;
    assert_eq!(values(&store), vec![vec!["Correct"], vec!["other"]]);
    store
        .apply(Operation::SetCell {
            frame_id: dict.id.clone(),
            row_id: dict.rows[0].id.clone(),
            column_id: dict.columns[1].id.clone(),
            raw: "Updated".into(),
        })
        .unwrap();
    assert_eq!(values(&store)[0][0], "Updated");
    store.undo();
    assert_eq!(values(&store)[0][0], "Correct");
    let before = store.document().clone();
    assert!(
        store
            .apply(Operation::SetCell {
                frame_id: dict.id.clone(),
                row_id: dict.rows[1].id.clone(),
                column_id: dict.columns[0].id.clone(),
                raw: "typo".into()
            })
            .is_err()
    );
    assert_eq!(store.document(), &before);
    let serialized = serde_json::to_string(store.document()).unwrap();
    let reloaded = Store::new(serde_json::from_str(&serialized).unwrap());
    assert_eq!(values(&reloaded), values(&store));
}

#[test]
fn mapping_a_live_derived_frame_tracks_both_inputs_and_refuses_back_edges() {
    let mut store = Store::new(Document::blank("Live mapping"));
    let dict = dictionary(&mut store);
    store
        .apply(Operation::AddFrame {
            name: "Source".into(),
            grid: vec![vec!["Category".into()], vec!["typo".into()]],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let source = frame_named(store.document(), "Source").clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: source.id.clone(),
            name: "Live".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let live = frame_named(store.document(), "Live").clone();
    let mapping = |frame: &FrameObject, formula: &str| Operation::SetFramePipeline {
        frame_id: frame.id.clone(),
        steps: vec![FrameStepInput::WithColumns {
            columns: vec![ExistingFormulaInput {
                output_column_id: frame.columns[0].id.clone(),
                name: frame.columns[0].name.clone(),
                formula: formula.into(),
            }],
        }],
    };
    store
        .apply(Operation::SetFramePipeline {
            frame_id: live.id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: source.columns[0].id.clone(),
                    name: "Category".into(),
                    formula: "map_values(`Category`, `Fixes`.`Key`, `Fixes`.`Value`)".into(),
                }],
            }],
        })
        .unwrap();
    assert_eq!(
        store.get_frame_page(&live.id, 0, 10).unwrap().rows[0][0],
        "Correct"
    );
    store
        .apply(Operation::SetCell {
            frame_id: source.id.clone(),
            row_id: source.rows[0].id.clone(),
            column_id: source.columns[0].id.clone(),
            raw: "other".into(),
        })
        .unwrap();
    assert_eq!(
        store.get_frame_page(&live.id, 0, 10).unwrap().rows[0][0],
        "other"
    );
    // A dictionary computed from its own consumer would close a cycle.
    let result = store.apply(mapping(
        &dict,
        "map_values(`Key`, `Live`.`Category`, `Live`.`Category`)",
    ));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("circular"));
    let cell = store.apply(Operation::SetCell {
        frame_id: live.id.clone(),
        row_id: "derived:0".into(),
        column_id: live.columns[0].id.clone(),
        raw: "manual".into(),
    });
    assert!(cell.is_err());
}

#[test]
fn dictionaries_retain_numeric_types() {
    let mut store = Store::new(Document::blank("Rates"));
    store
        .apply(Operation::AddFrame {
            name: "Rates".into(),
            grid: vec![
                vec!["Key".into(), "Value".into()],
                vec!["1".into(), "2.5".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Rates").clone();
    store
        .apply(Operation::SetUniqueKey {
            frame_id: frame.id.clone(),
            column_ids: vec![frame.columns[0].id.clone()],
            enabled: true,
        })
        .unwrap();
    let cells = answers(
        &mut store,
        "rate = lookup(1, `Rates`.`Key`, `Rates`.`Value`) * 2",
    );
    assert_eq!(cells[0].value, Some(5.0));
    let catalog = formula_function_catalog();
    assert!(
        catalog
            .iter()
            .any(|function| function.name == "lookup" && function.return_type == "dynamic")
    );
}

#[test]
fn a_mapped_column_joins_the_dictionary_lazily_and_a_strict_lookup_names_its_miss() {
    let mut store = Store::new(Document::blank("Lazy mapping"));
    dictionary(&mut store);
    store
        .apply(Operation::AddFrame {
            name: "Sales".into(),
            grid: vec![
                vec!["Category".into()],
                vec!["typo".into()],
                vec!["other".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let source = frame_named(store.document(), "Sales").clone();
    let mapping = |formula: &str| Operation::SetFramePipeline {
        frame_id: source.id.clone(),
        steps: vec![FrameStepInput::WithColumns {
            columns: vec![ExistingFormulaInput {
                output_column_id: source.columns[0].id.clone(),
                name: "Category".into(),
                formula: formula.into(),
            }],
        }],
    };
    store
        .apply(mapping(
            "map_values(`Category`, `Fixes`.`Key`, `Fixes`.`Value`)",
        ))
        .unwrap();
    // The dictionary is a join in the plan, not a literal pasted into it,
    // and the join's working columns do not leak into the frame's schema.
    let plan = store.get_frame_query_plan(&source.id).unwrap();
    assert!(plan.logical.contains("JOIN"), "{}", plan.logical);
    assert_eq!(
        store.get_frame_page(&source.id, 0, 10).unwrap().rows,
        vec![vec!["Correct"], vec!["other"]]
    );
    let path = std::env::temp_dir().join(format!("framework-mapped-{}.csv", id()));
    store.export_frame_file(&source.id, &path).unwrap();
    let header = std::fs::read_to_string(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(header.lines().next(), Some("Category"), "{header}");
    // A strict lookup down a column fails on the first key it cannot find.
    store
        .apply(mapping(
            "lookup(`Category`, `Fixes`.`Key`, `Fixes`.`Value`)",
        ))
        .unwrap();
    let error = store
        .get_frame_page(&source.id, 0, 10)
        .unwrap_err()
        .to_string();
    assert!(error.contains("‘other’"), "{error}");
    assert!(error.contains("Fixes"), "{error}");
}

#[test]
fn blank_keys_match_missing_values_and_blank_replacements_stay_null() {
    let mut store = Store::new(Document::blank("Missing mappings"));
    let mapping = dictionary(&mut store);
    store
        .apply(Operation::SetCell {
            frame_id: mapping.id.clone(),
            row_id: mapping.rows[0].id.clone(),
            column_id: mapping.columns[0].id.clone(),
            raw: "".into(),
        })
        .unwrap();
    let cells = answers(
        &mut store,
        "a = map_values(None, `Fixes`.`Key`, `Fixes`.`Value`)\nb = map_values(\"clear\", `Fixes`.`Key`, `Fixes`.`Value`)\nc = map_values(\"unmatched\", `Fixes`.`Key`, `Fixes`.`Value`, None)",
    );
    assert_eq!(cells[0].display, "Correct");
    assert_eq!(cells[1].typed_value, ScalarValue::Null);
    assert_eq!(cells[2].typed_value, ScalarValue::Null);
    assert!(cells.iter().all(|cell| cell.error.is_none()));
}
