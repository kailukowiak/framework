use crate::common::*;
use framework_core::*;
use polars::prelude as pl;
use polars::prelude::NamedFrom;
use std::fs;

/// A frame of severities written down out of order, so nothing about the row
/// order can be mistaken for the declared order.
fn severity_store() -> (Store, FrameObject) {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Triage".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Bugs".into(),
            grid: vec![
                vec!["Name".into(), "Severity".into()],
                vec!["Crash on open".into(), "High".into()],
                vec!["Typo".into(), "Low".into()],
                vec!["Slow export".into(), "Medium".into()],
                vec!["Data loss".into(), "High".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Bugs").clone();
    (store, frame)
}

fn column_id(frame: &FrameObject, name: &str) -> String {
    frame
        .columns
        .iter()
        .find(|column| column.name == name)
        .unwrap()
        .id
        .clone()
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

fn declare(store: &mut Store, frame: &FrameObject, column: &str, categories: &[&str]) {
    store
        .apply(Operation::SetColumnCategories {
            frame_id: frame.id.clone(),
            column_id: column_id(frame, column),
            categories: categories.iter().map(|value| value.to_string()).collect(),
        })
        .unwrap();
}

/// The point of writing a list of allowed values down is that the order you
/// write it in is the order the column has. Low, Medium, High sorts the way
/// it reads, which is the opposite of what the alphabet would do to it.
#[test]
fn a_declared_list_of_values_is_the_order_the_column_sorts_in() {
    let (mut store, frame) = severity_store();
    declare(&mut store, &frame, "Severity", &["Low", "Medium", "High"]);

    store
        .apply(Operation::SetFrameDisplaySort {
            frame_id: frame.id.clone(),
            keys: vec![DerivedSort {
                column_id: column_id(&frame, "Severity"),
                descending: false,
            }],
        })
        .unwrap();
    assert_eq!(
        column_values(&store, &frame.id, "Severity"),
        vec!["Low", "Medium", "High", "High"],
        "alphabetically this would have put High first"
    );

    store
        .apply(Operation::SetFrameDisplaySort {
            frame_id: frame.id.clone(),
            keys: vec![DerivedSort {
                column_id: column_id(&frame, "Severity"),
                descending: true,
            }],
        })
        .unwrap();
    assert_eq!(
        column_values(&store, &frame.id, "Severity"),
        vec!["High", "High", "Medium", "Low"]
    );
}

/// Same list, same reason: "worse than Medium" is a question about the order
/// someone declared, and a comparison is how anyone would ask it.
#[test]
fn comparing_against_a_category_reads_along_the_declared_order() {
    let (mut store, frame) = severity_store();
    declare(&mut store, &frame, "Severity", &["Low", "Medium", "High"]);

    store
        .apply(Operation::SetFrameDisplayFilter {
            frame_id: frame.id.clone(),
            filters: vec!["`Severity` >= \"Medium\"".into()],
            filter_match_all: true,
        })
        .unwrap();
    assert_eq!(
        column_values(&store, &frame.id, "Name"),
        vec!["Crash on open", "Slow export", "Data loss"]
    );

    // The worst one is High, not the alphabetical last.
    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame.id.clone(),
            name: "Worst".into(),
            formula: "`Severity`.max()".into(),
            after_column_id: None,
        })
        .unwrap();
    assert_eq!(
        column_values(&store, &frame.id, "Worst")[0],
        "High".to_string()
    );
}

/// Calling a column categorical without saying anything about the order is
/// not a decision about the order, so it must not silently become one: the
/// starting list is alphabetical, which is how the column already sorted
/// when it was text.
#[test]
fn calling_a_column_categorical_does_not_rearrange_it_behind_your_back() {
    let (mut store, frame) = severity_store();
    let before = {
        store
            .apply(Operation::SetFrameDisplaySort {
                frame_id: frame.id.clone(),
                keys: vec![DerivedSort {
                    column_id: column_id(&frame, "Severity"),
                    descending: false,
                }],
            })
            .unwrap();
        column_values(&store, &frame.id, "Severity")
    };
    assert_eq!(before, vec!["High", "High", "Low", "Medium"]);

    store
        .apply(Operation::SetColumnType {
            frame_id: frame.id.clone(),
            column_id: column_id(&frame, "Severity"),
            data_type: DataType::Categorical,
        })
        .unwrap();
    assert_eq!(
        store
            .document()
            .frame(&frame.id)
            .unwrap()
            .columns
            .iter()
            .find(|column| column.name == "Severity")
            .unwrap()
            .categories,
        vec!["High", "Low", "Medium"]
    );
    assert_eq!(column_values(&store, &frame.id, "Severity"), before);
}

/// A declared list travels: a derived frame that reads the column reads its
/// order too, so the dropdown and the sort still work downstream.
#[test]
fn a_derived_frame_inherits_the_order_its_source_declared() {
    let (mut store, frame) = severity_store();
    declare(&mut store, &frame, "Severity", &["Low", "Medium", "High"]);

    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: frame.id.clone(),
            name: "Triaged".into(),
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let derived = frame_named(store.document(), "Triaged").clone();
    let severity = derived
        .columns
        .iter()
        .find(|column| column.name == "Severity")
        .unwrap();
    assert_eq!(severity.data_type, DataType::Categorical);
    assert_eq!(severity.categories, vec!["Low", "Medium", "High"]);

    store
        .apply(Operation::SetFrameDisplaySort {
            frame_id: derived.id.clone(),
            keys: vec![DerivedSort {
                column_id: severity.id.clone(),
                descending: false,
            }],
        })
        .unwrap();
    assert_eq!(
        column_values(&store, &derived.id, "Severity"),
        vec!["Low", "Medium", "High", "High"]
    );
}

/// A column whose values come from a list is no longer plain text, and the
/// editor has always said so. Category-native string questions stay under
/// `.cat`; the one honest `.str` conversion is parsing a label as a date.
#[test]
fn the_editor_names_the_list_and_sends_string_methods_to_the_right_place() {
    let (mut store, frame) = severity_store();
    declare(&mut store, &frame, "Severity", &["Low", "Medium", "High"]);

    let text = "`Severity`.str.";
    let result =
        framework_core::complete_formula(store.document(), &frame.id, text, text.chars().count());
    assert_eq!(
        result
            .suggestions
            .iter()
            .map(|suggestion| suggestion.label.as_str())
            .collect::<Vec<_>>(),
        vec![".str.to_date"]
    );
    assert!(result.note.is_none());
    assert_eq!(
        result.receiver_dtype.as_deref(),
        Some("one of Low, Medium, High")
    );

    let text = "`Severity`.cat.";
    let result =
        framework_core::complete_formula(store.document(), &frame.id, text, text.chars().count());
    assert!(
        result
            .suggestions
            .iter()
            .any(|suggestion| suggestion.label.contains("starts_with")),
        "the cat namespace carries the everyday string questions: {:?}",
        result.suggestions
    );
}

/// Parquet stores the labels of an enum as strings and restores the logical
/// category type in its scan schema. Polars' categorical string dispatcher
/// currently trusts that logical type all the way into execution and panics
/// when the physical string column arrives. A filter is ordinary input and
/// must never be able to take the application process down with it.
#[test]
fn a_categorical_string_filter_on_an_artifact_does_not_panic() {
    let directory = temporary_test_directory("categorical-artifact-filter");
    let source = directory.join("bugs.parquet");
    let dtype = pl::DataType::from_frozen_categories(
        pl::FrozenCategories::new(["Low", "Medium", "High"]).unwrap(),
    );
    let severity = pl::Series::new("Severity".into(), ["High", "Low", "Medium"])
        .cast(&dtype)
        .unwrap();
    let mut frame = pl::DataFrame::new(3, vec![severity.into()]).unwrap();
    pl::ParquetWriter::new(fs::File::create(&source).unwrap())
        .finish(&mut frame)
        .unwrap();

    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Imported bugs".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::ImportFrameFromFile {
            name: "Bugs".into(),
            path: source.display().to_string(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Bugs").clone();
    store
        .apply(Operation::SetFrameDisplayFilter {
            frame_id: frame.id.clone(),
            filters: vec!["`Severity`.cat.starts_with(\"H\")".into()],
            filter_match_all: true,
        })
        .unwrap();

    assert_eq!(column_values(&store, &frame.id, "Severity"), vec!["High"]);
}

/// Two lists that are not the same list have no shared order, so Polars
/// refuses to compare them — which is right for `<` and wrong for a join.
/// A join asks whether two labels are the same label, and that has an answer
/// however the lists were written, so it keeps working by reading both as
/// text.
#[test]
fn a_join_still_matches_when_the_two_sides_allow_different_values() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Two lists".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    let mut side = |name: &str, values: &[&str], x: f64| {
        let mut grid = vec![vec!["Key".into(), "Note".into()]];
        for value in values {
            grid.push(vec![value.to_string(), format!("{name} {value}")]);
        }
        store
            .apply(Operation::AddFrame {
                name: name.into(),
                grid,
                x,
                y: 0.0,
            })
            .unwrap();
        let frame = frame_named(store.document(), name).clone();
        store
            .apply(Operation::SetColumnCategories {
                frame_id: frame.id.clone(),
                column_id: column_id(&frame, "Key"),
                categories: values.iter().map(|value| value.to_string()).collect(),
            })
            .unwrap();
        frame
    };
    let left = side("Left", &["a", "b"], 0.0);
    let right = side("Right", &["a", "b", "c"], 500.0);
    store
        .apply(Operation::SetUniqueKey {
            frame_id: right.id.clone(),
            column_ids: vec![column_id(&right, "Key")],
            enabled: true,
        })
        .unwrap();

    store
        .apply(Operation::AddJoinFrame {
            primary_frame_id: left.id.clone(),
            lookup_frame_id: right.id.clone(),
            primary_key_column_ids: vec![column_id(&left, "Key")],
            lookup_key_column_ids: vec![column_id(&right, "Key")],
            join_type: FrameJoinType::Left,
            columns: vec![
                JoinColumnInput {
                    source_frame_id: left.id.clone(),
                    source_column_id: column_id(&left, "Key"),
                    name: "Key".into(),
                },
                JoinColumnInput {
                    source_frame_id: right.id.clone(),
                    source_column_id: column_id(&right, "Note"),
                    name: "Right note".into(),
                },
            ],
            name: "Joined".into(),
            x: 0.0,
            y: 400.0,
        })
        .unwrap();
    let joined = frame_named(store.document(), "Joined").clone();
    assert_eq!(
        column_values(&store, &joined.id, "Right note"),
        vec!["Right a", "Right b"]
    );
    // And the key says what it now is, rather than going on offering a
    // dropdown and an order the join gave up to make the match.
    let key = joined
        .columns
        .iter()
        .find(|column| column.name == "Key")
        .unwrap();
    assert_eq!(key.data_type, DataType::String);
    assert!(key.categories.is_empty());
}

/// Comparing them in a formula genuinely has no answer, and saying so is the
/// job — in words about allowed values, not about enums and casts.
#[test]
fn comparing_two_different_lists_says_so_in_the_words_of_the_thing() {
    let (mut store, frame) = severity_store();
    declare(&mut store, &frame, "Severity", &["Low", "Medium", "High"]);
    store
        .apply(Operation::AddColumn {
            frame_id: frame.id.clone(),
            name: "Reported as".into(),
            data_type: DataType::String,
            after_column_id: None,
        })
        .unwrap();
    let reported = column_id(store.document().frame(&frame.id).unwrap(), "Reported as");
    for row in store.document().frame(&frame.id).unwrap().rows.clone() {
        store
            .apply(Operation::SetCell {
                frame_id: frame.id.clone(),
                row_id: row.id,
                column_id: reported.clone(),
                raw: "Low".into(),
            })
            .unwrap();
    }
    store
        .apply(Operation::SetColumnCategories {
            frame_id: frame.id.clone(),
            column_id: reported,
            categories: vec!["Low".into(), "Urgent".into()],
        })
        .unwrap();

    let error = store
        .apply(Operation::AddComputedColumn {
            frame_id: frame.id.clone(),
            name: "Agrees".into(),
            formula: "`Severity` == `Reported as`".into(),
            after_column_id: None,
        })
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("allow different sets of values") && error.contains("allowed values"),
        "{error}"
    );
    assert!(!error.contains("Enum"), "{error}");
}

/// A value outside the list is refused when it is typed. A document that
/// somehow carries one anyway must not take the whole frame down with it —
/// the cell reads as empty, the way an unreadable cell always has.
#[test]
fn a_value_outside_the_list_is_refused_rather_than_stored() {
    let (mut store, frame) = severity_store();
    declare(&mut store, &frame, "Severity", &["Low", "Medium", "High"]);
    let error = store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: frame.rows[0].id.clone(),
            column_id: column_id(&frame, "Severity"),
            raw: "Catastrophic".into(),
        })
        .unwrap_err();
    assert!(error.to_string().contains("not an allowed value"));

    // Narrowing the list out from under a value that is still in the column
    // is refused for the same reason, from the other direction.
    let error = store
        .apply(Operation::SetColumnCategories {
            frame_id: frame.id.clone(),
            column_id: column_id(&frame, "Severity"),
            categories: vec!["Low".into(), "Medium".into()],
        })
        .unwrap_err();
    assert!(error.to_string().contains("still contains it"));
}
