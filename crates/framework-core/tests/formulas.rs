use crate::common::*;
use framework_core::*;

#[test]
fn computed_column_uses_stable_ids_after_rename() {
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
    let quantity_id = frame.columns[0].id.clone();
    let total_id = frame.columns[2].id.clone();

    store
        .apply(Operation::RenameColumn {
            frame_id: frame_id.clone(),
            column_id: quantity_id,
            name: "Units".into(),
        })
        .unwrap();

    let view = store.view();
    let computed = &view.computed_frames[&frame_id];
    assert!(computed.formulas[&total_id].contains("Units"));
    assert!(
        computed
            .rows
            .values()
            .all(|row| row[&total_id].value.is_some())
    );
}

#[test]
fn rejects_circular_column_formulas() {
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
    let quantity_id = frame.columns[0].id.clone();

    let error = store
        .apply(Operation::SetColumnFormula {
            frame_id,
            column_id: quantity_id,
            formula: "`Total` + 1".into(),
        })
        .unwrap_err();
    assert!(matches!(error, CoreError::CircularDependency));
}

#[test]
fn cell_override_is_computed_and_rendered_separately() {
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
    let total_id = frame.columns[2].id.clone();

    let view = store
        .apply(Operation::SetCellOverride {
            frame_id: frame_id.clone(),
            row_id: row_id.clone(),
            column_id: total_id.clone(),
            formula: Some("`Quantity` * `Unit price`".into()),
        })
        .unwrap();

    let computed = &view.computed_frames[&frame_id];
    assert_eq!(computed.rows[&row_id][&total_id].value, Some(42.0));
    assert!(computed.rows[&row_id][&total_id].is_override);
    assert_eq!(
        computed.override_formulas[&row_id][&total_id],
        "`Quantity` * `Unit price`"
    );
}

#[test]
fn rejects_recursive_cell_override() {
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
    let error = store
        .apply(Operation::SetCellOverride {
            frame_id: frame.id.clone(),
            row_id: frame.rows[0].id.clone(),
            column_id: frame.columns[2].id.clone(),
            formula: Some("`Total` + 1".into()),
        })
        .unwrap_err();
    assert!(matches!(error, CoreError::CircularDependency));
}

#[test]
fn literal_column_can_be_converted_to_a_calculated_column() {
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
    let quantity_id = store
        .document()
        .frame(&frame_id)
        .unwrap()
        .columns
        .iter()
        .find(|column| column.name == "Quantity")
        .unwrap()
        .id
        .clone();

    let view = store
        .apply(Operation::SetColumnFormula {
            frame_id: frame_id.clone(),
            column_id: quantity_id.clone(),
            formula: "`Unit price` * 2".into(),
        })
        .unwrap();
    let frame = view.document.frame(&frame_id).unwrap();
    assert!(
        frame
            .columns
            .iter()
            .find(|column| column.id == quantity_id)
            .unwrap()
            .formula
            .is_some()
    );
    let values = frame
        .rows
        .iter()
        .map(|row| view.computed_frames[&frame_id].rows[&row.id][&quantity_id].value)
        .collect::<Vec<_>>();
    assert_eq!(values, vec![Some(28.0), Some(15.0), Some(56.0)]);
}

#[test]
fn filter_composes_excel_style_conditional_aggregates() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Conditional aggregates".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Sales".into(),
            grid: vec![
                vec!["Amount".into(), "Region".into()],
                vec!["10".into(), "East".into()],
                vec!["20".into(), "West".into()],
                vec!["5".into(), "East".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Sales").id.clone();
    for (name, formula) in [
        ("East total", "`Amount`.filter(`Region` == \"East\").sum()"),
        (
            "East count",
            "`Amount`.filter(`Region` == \"East\").count()",
        ),
        (
            "East average",
            "`Amount`.filter(`Region` == \"East\").mean()",
        ),
    ] {
        store
            .apply(Operation::AddComputedColumn {
                frame_id: frame_id.clone(),
                name: name.into(),
                formula: formula.into(),
                after_column_id: None,
            })
            .unwrap();
    }

    let frame = frame_named(store.document(), "Sales");
    let column_index = |name: &str| {
        frame
            .columns
            .iter()
            .position(|column| column.name == name)
            .unwrap()
    };
    let page = store.get_frame_page(&frame_id, 0, 10).unwrap();
    for row in &page.rows {
        assert_eq!(row[column_index("East total")], "15");
        assert_eq!(row[column_index("East count")], "2");
        assert_eq!(row[column_index("East average")], "7.5");
    }

    let direct_filter = store.apply(Operation::AddComputedColumn {
        frame_id: frame_id.clone(),
        name: "Invalid direct filter".into(),
        formula: "`Amount`.filter(`Region` == \"East\")".into(),
        after_column_id: None,
    });
    assert!(
        matches!(direct_filter, Err(CoreError::Formula(message)) if message.contains("finish it with an aggregate"))
    );

    let non_boolean_predicate = store.apply(Operation::AddComputedColumn {
        frame_id,
        name: "Invalid predicate".into(),
        formula: "`Amount`.filter(1).sum()".into(),
        after_column_id: None,
    });
    assert!(
        matches!(non_boolean_predicate, Err(CoreError::Formula(message)) if message.contains("true/false predicate"))
    );
}

#[test]
fn string_to_date_is_a_strict_iso_cast_without_hidden_options() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "String date conversion".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Rows".into(),
            grid: vec![vec!["Label".into()], vec!["One".into()]],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Rows").id.clone();

    // Chaining into the date namespace proves that the formula's declared
    // type follows the conversion, rather than merely compiling a string
    // method with a date-like name.
    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Month".into(),
            formula: "\"2026-02-03\".str.to_date().dt.month()".into(),
            after_column_id: None,
        })
        .unwrap();
    assert_eq!(
        store.get_frame_page(&frame_id, 0, 10).unwrap().rows[0][1],
        "2"
    );

    let options = store.apply(Operation::AddComputedColumn {
        frame_id,
        name: "Unsupported format".into(),
        formula: "\"2026-02-03\".str.to_date(\"%Y-%m-%d\")".into(),
        after_column_id: None,
    });
    assert!(
        matches!(options, Err(CoreError::Formula(message)) if message.contains("takes no arguments"))
    );
}

#[test]
fn calculated_column_autofills_every_row_from_a_canvas_value() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Reference test".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddValue {
            name: "Safety Factor".into(),
            raw: "1.7".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    store
        .apply(Operation::AddFrame {
            name: "Imported data".into(),
            grid: vec![
                vec!["Name".into(), "Amount".into()],
                vec!["Alpha".into(), "120".into()],
                vec!["Beta".into(), "85".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) => Some(frame.id.clone()),
            _ => None,
        })
        .unwrap();

    let view = store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Adjusted".into(),
            formula: "`Amount` * `Safety Factor`".into(),
            after_column_id: None,
        })
        .unwrap();
    let frame = view
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) => Some(frame),
            _ => None,
        })
        .unwrap();
    let adjusted_id = &frame.columns.last().unwrap().id;
    let values: Vec<f64> = frame
        .rows
        .iter()
        .map(|row| {
            view.computed_frames[&frame_id].rows[&row.id][adjusted_id]
                .value
                .unwrap()
        })
        .collect();
    assert_eq!(values, vec![204.0, 144.5]);
}

#[test]
fn polars_syntax_executes_arithmetic_namespaces_windows_and_conditionals() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Native Polars formulas".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "People".into(),
            grid: vec![
                vec![
                    "Weight".into(),
                    "Height".into(),
                    "Birthdate".into(),
                    "Group".into(),
                ],
                vec!["80".into(), "2".into(), "1990-04-10".into(), "A".into()],
                vec!["90".into(), "3".into(), "1985-01-02".into(), "A".into()],
                vec!["100".into(), "2".into(), "2000-12-31".into(), "B".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let (frame_id, weight_id) = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) => Some((frame.id.clone(), frame.columns[0].id.clone())),
            _ => None,
        })
        .unwrap();
    // Shift is only meaningful against declared order. The rows already
    // happen to be ascending, but the declaration is what makes that an
    // invariant rather than an accident of this fixture's insertion order.
    store
        .apply(Operation::SetFrameDisplaySort {
            frame_id: frame_id.clone(),
            keys: vec![DerivedSort {
                column_id: weight_id,
                descending: false,
            }],
        })
        .unwrap();

    for (name, formula) in [
        ("BMI", "`Weight` / (`Height` ** 2)"),
        ("Birth year", "`Birthdate`.dt.year()"),
        ("Previous", "`Weight`.shift(1)"),
        ("Group total", "`Weight`.sum().over(`Group`)"),
        ("Rolling", "`Weight`.rolling_mean(window_size=2)"),
        (
            "Horizontal mean",
            "mean_horizontal([`Weight`, None], ignore_nulls=True)",
        ),
        (
            "Strict horizontal mean",
            "mean_horizontal([`Weight`, None], ignore_nulls=False)",
        ),
        (
            "Band",
            "when(`Weight` >= 90).then(\"High\").otherwise(\"Low\")",
        ),
    ] {
        store
            .apply(Operation::AddComputedColumn {
                frame_id: frame_id.clone(),
                name: name.into(),
                formula: formula.into(),
                after_column_id: None,
            })
            .unwrap();
    }

    let view = store.view();
    let frame = view.document.frame(&frame_id).unwrap();
    let page = store.get_frame_page(&frame_id, 0, 10).unwrap();
    let values = |name: &str| {
        let column_index = frame
            .columns
            .iter()
            .position(|column| column.name == name)
            .unwrap();
        page.rows
            .iter()
            .map(|row| row[column_index].clone())
            .collect::<Vec<_>>()
    };

    assert_eq!(values("BMI"), ["20", "10", "25"]);
    assert_eq!(values("Birth year"), ["1990", "1985", "2000"]);
    assert_eq!(values("Previous"), ["", "80", "90"]);
    assert_eq!(values("Group total"), ["170", "170", "100"]);
    assert_eq!(values("Rolling"), ["80", "85", "95"]);
    assert_eq!(values("Horizontal mean"), ["80", "90", "100"]);
    assert_eq!(values("Strict horizontal mean"), ["", "", ""]);
    assert_eq!(values("Band"), ["Low", "High", "High"]);

    let invalid = store.apply(Operation::AddComputedColumn {
        frame_id,
        name: "Invalid".into(),
        formula: "`Group`.dt.year()".into(),
        after_column_id: None,
    });
    // What the engine says about it, rather than the fact that an engine
    // said it: the message reaches the editor as the reason, unprefixed.
    let Err(CoreError::Formula(message)) = invalid else {
        panic!("a date method on a text column is not a formula this can run");
    };
    assert!(
        message.contains("`year` operation not supported for dtype `str`"),
        "{message}"
    );
    assert!(
        !message.contains("Resolved plan until failure"),
        "{message}"
    );
}

/// Dates and durations are literals, because that is how they are written.
///
/// `2026-08-12` used to lex as `2026 - 8 - 12` and evaluate, silently, to
/// 2006 — the worst kind of wrong, since nothing about it looks wrong. It
/// is now one token, and so is `30d`, which together make a relative
/// filter something you write rather than compute.
#[test]
fn dates_and_durations_are_literals_the_way_people_write_them() {
    use chrono::NaiveDate;

    let mut store = demo_store();
    let frame_id = frame_named(store.document(), "Transactions").id.clone();

    for (name, formula) in [
        ("Literal", "2026-08-12"),
        ("Long form", "2026-08-12 - 30days"),
        ("Minus a month", "2026-03-31 - 1mo"),
        ("Plus a week", "2026-08-12 + 1w"),
        ("Shifted column", "`Sold on` + 10d"),
    ] {
        store
            .apply(Operation::AddComputedColumn {
                frame_id: frame_id.clone(),
                name: name.into(),
                formula: formula.into(),
                after_column_id: None,
            })
            .unwrap();
    }

    let view = store.view();
    let frame = view.document.frame(&frame_id).unwrap();
    let computed = &view.computed_frames[&frame_id];
    let first = |name: &str| {
        let column = frame
            .columns
            .iter()
            .find(|column| column.name == name)
            .unwrap();
        computed.rows[&frame.rows[0].id][&column.id]
            .typed_value
            .clone()
    };
    let date =
        |year, month, day| ScalarValue::Date(NaiveDate::from_ymd_opt(year, month, day).unwrap());

    assert_eq!(first("Literal"), date(2026, 8, 12));
    assert_eq!(first("Long form"), date(2026, 7, 13), "30days is 30d");
    // The calendar answer, not a fixed span: a month before the 31st of
    // March is the end of February, whatever length February happens to be.
    assert_eq!(first("Minus a month"), date(2026, 2, 28));
    assert_eq!(first("Plus a week"), date(2026, 8, 19));
    // The first transaction is 2026-07-02.
    assert_eq!(first("Shifted column"), date(2026, 7, 12));

    // Spaces still mean arithmetic, so nothing that used to be a sum has
    // quietly become a date.
    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Spaced".into(),
            formula: "2026 - 08 - 12".into(),
            after_column_id: None,
        })
        .unwrap();
    let view = store.view();
    let frame = view.document.frame(&frame_id).unwrap();
    let column = frame
        .columns
        .iter()
        .find(|column| column.name == "Spaced")
        .unwrap();
    assert_eq!(
        view.computed_frames[&frame_id].rows[&frame.rows[0].id][&column.id].typed_value,
        ScalarValue::Number(2006.0)
    );
}

/// `today()` is read when the frame is read, not when the filter is saved.
#[test]
fn today_is_read_at_query_time_and_takes_durations() {
    let mut store = demo_store();
    let frame_id = frame_named(store.document(), "Transactions").id.clone();

    for (name, formula) in [("Today", "today()"), ("A month ago", "today() - 1mo")] {
        store
            .apply(Operation::AddComputedColumn {
                frame_id: frame_id.clone(),
                name: name.into(),
                formula: formula.into(),
                after_column_id: None,
            })
            .unwrap();
    }

    let view = store.view();
    let frame = view.document.frame(&frame_id).unwrap();
    let computed = &view.computed_frames[&frame_id];
    let first = |name: &str| {
        let column = frame
            .columns
            .iter()
            .find(|column| column.name == name)
            .unwrap();
        computed.rows[&frame.rows[0].id][&column.id]
            .typed_value
            .clone()
    };

    let today = chrono::Local::now().date_naive();
    assert_eq!(first("Today"), ScalarValue::Date(today));
    let ScalarValue::Date(month_ago) = first("A month ago") else {
        panic!("today() - 1mo has to still be a date");
    };
    assert!(month_ago < today && today.signed_duration_since(month_ago).num_days() <= 31);
}

/// The Excel habit: an integer against a date is a count of days.
///
/// `today() + 1` is tomorrow, and the count may be a column, so the offset
/// has to be assembled row by row rather than fixed when the formula is
/// saved. Every other calendar unit already has a duration spelling, which
/// is what makes the bare integer unambiguous.
#[test]
fn an_integer_against_a_date_counts_days() {
    let mut store = demo_store();
    let frame_id = frame_named(store.document(), "Transactions").id.clone();

    for (name, formula) in [
        ("Tomorrow", "today() + 1"),
        ("Last week", "today() - 7"),
        ("Follow-up", "`Sold on` + `Units`"),
        ("Offset method", "`Sold on`.dt.offset_by(`Units`)"),
    ] {
        store
            .apply(Operation::AddComputedColumn {
                frame_id: frame_id.clone(),
                name: name.into(),
                formula: formula.into(),
                after_column_id: None,
            })
            .unwrap();
    }

    let view = store.view();
    let frame = view.document.frame(&frame_id).unwrap();
    let computed = &view.computed_frames[&frame_id];
    let first = |name: &str| {
        let column = frame
            .columns
            .iter()
            .find(|column| column.name == name)
            .unwrap();
        computed.rows[&frame.rows[0].id][&column.id]
            .typed_value
            .clone()
    };

    let today = chrono::Local::now().date_naive();
    assert_eq!(
        first("Tomorrow"),
        ScalarValue::Date(today + chrono::Duration::days(1))
    );
    assert_eq!(
        first("Last week"),
        ScalarValue::Date(today - chrono::Duration::days(7))
    );
    let ScalarValue::Date(sold_on) = first("Sold on") else {
        panic!("Sold on has to be a date");
    };
    let ScalarValue::Number(units) = first("Units") else {
        panic!("Units has to be a number");
    };
    for name in ["Follow-up", "Offset method"] {
        assert_eq!(
            first(name),
            ScalarValue::Date(sold_on + chrono::Duration::days(units as i64)),
            "{name} should move Sold on by Units days"
        );
    }
}

/// Calendar parts are spreadsheet integers, not machine-width ones.
///
/// Polars hands `.dt.day()` back as eight bits, and eight-bit arithmetic
/// wraps: `38 * day` answered 116 in a real document — no error, just a
/// wrong number wearing a plausible face. The compiler widens every part
/// so no formula ever multiplies inside a byte.
#[test]
fn date_parts_do_not_wrap_under_multiplication() {
    let mut store = demo_store();
    let frame_id = frame_named(store.document(), "Transactions").id.clone();
    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Scaled day".into(),
            formula: "38 * `Sold on`.dt.day()".into(),
            after_column_id: None,
        })
        .unwrap();

    let view = store.view();
    let frame = view.document.frame(&frame_id).unwrap();
    let computed = &view.computed_frames[&frame_id];
    let day_column = frame
        .columns
        .iter()
        .find(|column| column.name == "Scaled day")
        .unwrap();
    let sold_on = frame
        .columns
        .iter()
        .find(|column| column.name == "Sold on")
        .unwrap();
    for row in &frame.rows {
        let ScalarValue::Date(date) = computed.rows[&row.id][&sold_on.id].typed_value else {
            continue;
        };
        use chrono::Datelike;
        assert_eq!(
            computed.rows[&row.id][&day_column.id].typed_value,
            ScalarValue::Number((38 * date.day()) as f64),
            "38 × day must be real multiplication, not eight-bit wrap"
        );
    }
}

/// A duration on its own is not an answer, and says so.
#[test]
fn a_duration_has_to_be_attached_to_a_date() {
    let mut store = demo_store();
    let frame_id = frame_named(store.document(), "Transactions").id.clone();

    for (formula, expected) in [
        ("30d", "length of time"),
        ("30d - today()", "Write the date first"),
        ("2026-02-30", "not a real date"),
    ] {
        let failure = store.apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Broken".into(),
            formula: formula.into(),
            after_column_id: None,
        });
        let message = match failure {
            Err(CoreError::Formula(message)) => message,
            other => panic!("‘{formula}’ should have been refused, got {other:?}"),
        };
        assert!(
            message.contains(expected),
            "‘{formula}’ should have said ‘{expected}’, said: {message}"
        );
    }
}

/// The two commonest filters there are: a range, and a set.
///
/// Both are hand-written bindings rather than generated ones, because both
/// carry an argument the generator will not guess at — a `ClosedInterval`
/// and a bare `bool`. The defaults chosen here are what the words mean to
/// a person: between includes its ends, and a null is in no set.
#[test]
fn ranges_and_sets_filter_the_way_the_words_read() {
    let mut store = demo_store();
    let frame_id = frame_named(store.document(), "Transactions").id.clone();

    for (name, formula) in [
        ("In July", "`Sold on`.is_between(2026-07-05, 2026-07-13)"),
        (
            "In July, open",
            "`Sold on`.is_between(2026-07-05, 2026-07-13, closed=\"none\")",
        ),
        ("Small order", "`Units`.is_in([1, 2, 3])"),
        (
            "Either",
            "`Units`.is_in([1, 2]) | `Sold on`.is_between(2026-07-18, today())",
        ),
    ] {
        store
            .apply(Operation::AddComputedColumn {
                frame_id: frame_id.clone(),
                name: name.into(),
                formula: formula.into(),
                after_column_id: None,
            })
            .unwrap();
    }

    let view = store.view();
    let frame = view.document.frame(&frame_id).unwrap();
    let computed = &view.computed_frames[&frame_id];
    let values = |name: &str| {
        let column = frame
            .columns
            .iter()
            .find(|column| column.name == name)
            .unwrap();
        frame
            .rows
            .iter()
            .map(|row| computed.rows[&row.id][&column.id].typed_value.clone())
            .collect::<Vec<_>>()
    };
    let flags = |bits: [bool; 6]| bits.map(ScalarValue::Boolean).to_vec();

    // Sold on: 07-02, 07-05, 07-09, 07-13, 07-18, 07-22.
    assert_eq!(
        values("In July"),
        flags([false, true, true, true, false, false])
    );
    assert_eq!(
        values("In July, open"),
        flags([false, false, true, false, false, false]),
        "closed=none drops both ends"
    );
    // Units: 3, 5, 2, 7, 4, 1.
    assert_eq!(
        values("Small order"),
        flags([true, false, true, false, false, true])
    );
    assert_eq!(
        values("Either"),
        flags([false, false, true, false, true, true])
    );

    for (formula, expected) in [
        (
            "`Sold on`.is_between(2026-07-05, 2026-07-13, closed=\"halfway\")",
            "both, left, right, or none",
        ),
        ("`Units`.is_between(1)", "expects 2 arguments"),
    ] {
        let failure = store.apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Broken".into(),
            formula: formula.into(),
            after_column_id: None,
        });
        let message = match failure {
            Err(CoreError::Formula(message)) => message,
            other => panic!("‘{formula}’ should have been refused, got {other:?}"),
        };
        assert!(
            message.contains(expected),
            "‘{formula}’ should have said ‘{expected}’, said: {message}"
        );
    }
}

/// Being told a real type is not a type is worse than being told nothing.
///
/// `.cast("categorical")` is the natural thing to reach for when a column
/// should be treated as a set of named values, and the generic refusal — a
/// list of four other types — reads as "there is no such thing here", which
/// is false. A category is a type; it just carries its allowed values with
/// it, so it is declared on the column rather than computed by an
/// expression. The message has to say where to go, and it has to mention
/// that conditional formatting, which is what usually prompts the ask,
/// already sorts ordinary text into named values without it.
#[test]
fn casting_to_a_category_says_where_categories_come_from() {
    let mut store = demo_store();
    let frame_id = frame_named(store.document(), "Products").id.clone();
    for named in ["categorical", "category", "enum"] {
        let failure = store
            .apply(Operation::AddComputedColumn {
                frame_id: frame_id.clone(),
                name: "Kind".into(),
                formula: format!("`Category`.cast(\"{named}\")"),
                after_column_id: None,
            })
            .unwrap_err()
            .to_string();
        assert!(
            failure.contains("setting its type") && failure.contains("allowed values"),
            "‘{named}’ was refused with: {failure}"
        );
        // Not the generic list, which is the message that reads as a denial
        // that categories exist.
        assert!(!failure.contains("is not a type this can convert to"));
    }
    // A type that genuinely is not one still gets the plain list.
    let nonsense = store
        .apply(Operation::AddComputedColumn {
            frame_id,
            name: "Kind".into(),
            formula: "`Category`.cast(\"colour\")".into(),
            after_column_id: None,
        })
        .unwrap_err()
        .to_string();
    assert!(
        nonsense.contains("is not a type this can convert to"),
        "{nonsense}"
    );
}

/// The connectives written as words mean what `&` and `|` mean.
///
/// The smoke test watched an author try `(a == b) and (c == d)`, get
/// "Unexpected text at end of formula", and burn a scratch block working
/// out that `&` was the spelling. The words are unambiguous, so they lex
/// to the same operators instead of to an error that names no fix.
#[test]
fn and_or_written_as_words_are_the_boolean_operators() {
    let mut store = demo_store();
    let frame_id = frame_named(store.document(), "Transactions").id.clone();
    for (name, formula, expected) in [
        (
            "Both",
            "when((1 == 1) and (2 == 2)).then(8).otherwise(0)",
            8.0,
        ),
        (
            "Either",
            "when((1 == 2) or (2 == 2)).then(3).otherwise(0)",
            3.0,
        ),
        (
            "Neither",
            "when((1 == 2) and (2 == 2)).then(8).otherwise(0)",
            0.0,
        ),
    ] {
        store
            .apply(Operation::AddComputedColumn {
                frame_id: frame_id.clone(),
                name: name.into(),
                formula: formula.into(),
                after_column_id: None,
            })
            .unwrap();
        let view = store.view();
        let frame = view.document.frame(&frame_id).unwrap();
        let column = frame
            .columns
            .iter()
            .find(|column| column.name == name)
            .unwrap();
        assert_eq!(
            view.computed_frames[&frame_id].rows[&frame.rows[0].id][&column.id].typed_value,
            ScalarValue::Number(expected),
            "{formula}"
        );
    }
}

/// `!= null` means "is present", because that is what it says.
///
/// Left to Polars, equality against null is null for every row — a filter
/// reads that as false, so `` `Hours` != null `` matched nothing while two
/// entries sat right there. SQL solved this by demanding IS NULL and
/// smoke-test agents fell into the gap; a spreadsheet formula should just
/// mean the obvious thing.
#[test]
fn comparing_against_null_asks_about_presence() {
    let mut store = demo_store();
    let frame_id = frame_named(store.document(), "Transactions").id.clone();
    let note_id = {
        store
            .apply(Operation::AddColumn {
                frame_id: frame_id.clone(),
                name: "Note".into(),
                data_type: DataType::String,
                after_column_id: None,
            })
            .unwrap();
        let frame = frame_named(store.document(), "Transactions");
        let column = frame.columns.last().unwrap().id.clone();
        let row = frame.rows[0].id.clone();
        store
            .apply(Operation::SetCell {
                frame_id: frame_id.clone(),
                row_id: row,
                column_id: column.clone(),
                raw: "checked".into(),
            })
            .unwrap();
        column
    };
    let _ = note_id;

    for (name, formula, expected) in [
        ("Present", "when(`Note` != null).then(1).otherwise(0)", 1.0),
        ("Missing", "when(`Note` == null).then(1).otherwise(0)", 0.0),
    ] {
        store
            .apply(Operation::AddComputedColumn {
                frame_id: frame_id.clone(),
                name: name.into(),
                formula: formula.into(),
                after_column_id: None,
            })
            .unwrap();
        let view = store.view();
        let frame = view.document.frame(&frame_id).unwrap();
        let column = frame
            .columns
            .iter()
            .find(|column| column.name == name)
            .unwrap();
        assert_eq!(
            view.computed_frames[&frame_id].rows[&frame.rows[0].id][&column.id].typed_value,
            ScalarValue::Number(expected),
            "{formula} on a filled cell"
        );
        let empty_row = &frame.rows[1].id;
        assert_eq!(
            view.computed_frames[&frame_id].rows[empty_row][&column.id].typed_value,
            ScalarValue::Number(1.0 - expected),
            "{formula} on an empty cell"
        );
    }
}
