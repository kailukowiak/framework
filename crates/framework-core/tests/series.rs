use crate::common::*;
use framework_core::*;
use std::fs;

fn orders_store() -> (Store, String) {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Orders".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Orders".into(),
            grid: vec![
                vec!["Currency".into(), "Amount".into()],
                vec!["USD".into(), "100".into()],
                vec!["JPY".into(), "20".into()],
                vec!["CAD".into(), "5".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Orders").id.clone();
    (store, frame_id)
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

fn series_named<'a>(store: &'a Store, name: &str) -> &'a SeriesObject {
    store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Series(series) if series.name == name => Some(series),
            _ => None,
        })
        .unwrap()
}

/// A list is a thing on the canvas, named, and read by name from a formula
/// — the answer to selecting a range in a spreadsheet and calling it one.
#[test]
fn a_named_list_can_be_written_down_and_read_from_a_formula() {
    let (mut store, frame_id) = orders_store();
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddSeries {
            name: "Domestic".into(),
            values: "USD, CAD".into(),
            x: 400.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    assert_eq!(series_named(&store, "Domestic").values, vec!["USD", "CAD"]);
    assert_eq!(series_named(&store, "Domestic").data_type, DataType::String);

    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "At home".into(),
            formula: "`Currency`.is_in(`Domestic`)".into(),
            after_column_id: None,
        })
        .unwrap();
    assert_eq!(
        column_values(&store, &frame_id, "At home"),
        vec!["true", "false", "true"]
    );

    // And it reads back as the name that was written.
    let column = frame_named(store.document(), "Orders")
        .columns
        .iter()
        .find(|column| column.name == "At home")
        .unwrap()
        .id
        .clone();
    assert_eq!(
        store.view().computed_frames[&frame_id].formulas[&column],
        "`Currency`.is_in(`Holder`.`Domestic`)"
    );
}

/// A list is a list however many values it holds, because that is what it
/// was declared as. Using one where a single value belongs is refused
/// rather than quietly broadcast or zipped.
#[test]
fn a_list_has_to_go_where_a_list_goes() {
    let (mut store, frame_id) = orders_store();
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddSeries {
            name: "Rates".into(),
            values: "[1.0, 0.0067, 0.74]".into(),
            x: 400.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    assert_eq!(series_named(&store, "Rates").data_type, DataType::Number);

    let error = store
        .apply(Operation::AddComputedColumn {
            frame_id,
            name: "Converted".into(),
            formula: "`Amount` * `Rates`".into(),
            after_column_id: None,
        })
        .unwrap_err()
        .to_string();
    // Refused because pairing a list with a column matches them by
    // position, which is the reason worth giving — and both fixes are
    // named, including the one this formula probably wanted.
    assert!(error.contains("by position"), "{error}");
    assert!(error.contains("is_in"), "{error}");
}

/// Nobody types a list from nothing — it is copied out of a spreadsheet, a
/// Python session, an R session, or a message. All of those are read.
#[test]
fn a_list_is_read_out_of_whatever_shape_it_was_copied_from() {
    let cases = [
        ("[1, 2, 3]", vec!["1", "2", "3"]),
        ("array([1, 2, 3])", vec!["1", "2", "3"]),
        ("c(1, 2, 3)", vec!["1", "2", "3"]),
        ("np.array([1, 2, 3])", vec!["1", "2", "3"]),
        ("{1, 2, 3}", vec!["1", "2", "3"]),
        ("1, 2, 3", vec!["1", "2", "3"]),
        ("USD\nJPY\nCAD", vec!["USD", "JPY", "CAD"]),
        ("USD\tJPY\tCAD", vec!["USD", "JPY", "CAD"]),
        ("[\"a\", \"b\"]", vec!["a", "b"]),
        ("['a', 'b']", vec!["a", "b"]),
        // A value with a comma in it survives, because splitting is
        // quote-aware. Getting this wrong would be a wrong answer.
        ("[\"a, b\", \"c\"]", vec!["a, b", "c"]),
        // Trailing separators and blank lines are noise, not values.
        ("[1, 2, 3,]", vec!["1", "2", "3"]),
        ("USD\n\nCAD\n", vec!["USD", "CAD"]),
    ];
    for (index, (text, expected)) in cases.iter().enumerate() {
        let mut store = Store::new(Document {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Lists".into(),
            revision: 0,
            objects: Vec::new(),
            views: Vec::new(),
            frozen_values: Default::default(),
        });
        let holder = a_container(&mut store);
        store
            .apply(Operation::AddSeries {
                name: format!("List {index}"),
                values: (*text).into(),
                x: 0.0,
                y: 0.0,
                container_id: Some(holder.clone()),
            })
            .unwrap();
        assert_eq!(
            &series_named(&store, &format!("List {index}")).values,
            expected,
            "reading {text:?}"
        );
    }
}

/// A list that already exists in a file need not be retyped: Polars reads
/// it, and the column's own type comes with it rather than being guessed
/// back out of printed text.
#[test]
fn a_list_can_be_read_out_of_a_column_of_a_file() {
    let directory = temporary_test_directory("series-import");
    let source = directory.join("codes.csv");
    fs::write(&source, "Code,Weight\n01234,1\n00567,2\n").unwrap();

    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Codes".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    let holder = a_container(&mut store);
    store
        .apply(Operation::ImportSeriesFromFile {
            name: "Weights".into(),
            path: source.to_string_lossy().into(),
            column: Some("Weight".into()),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    let weights = series_named(&store, "Weights");
    assert_eq!(weights.values, vec!["1", "2"]);
    assert_eq!(weights.data_type, DataType::Integer);

    // Naming no column takes the first one.
    store
        .apply(Operation::ImportSeriesFromFile {
            name: "Codes".into(),
            path: source.to_string_lossy().into(),
            column: None,
            x: 0.0,
            y: 300.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    assert_eq!(series_named(&store, "Codes").values.len(), 2);

    let error = store
        .apply(Operation::ImportSeriesFromFile {
            name: "Missing".into(),
            path: source.to_string_lossy().into(),
            column: Some("Nope".into()),
            x: 0.0,
            y: 600.0,
            container_id: Some(holder.clone()),
        })
        .unwrap_err()
        .to_string();
    assert!(error.contains("not a column of that file"), "{error}");
}

/// Retyping a list is allowed where every value survives the reading, and
/// refused where one would not — calling a list of names numbers would turn
/// each of them into a null without saying so.
#[test]
fn a_list_can_be_retyped_only_where_every_value_still_reads() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Codes".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddSeries {
            name: "Postcodes".into(),
            values: "01234, 00567".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    let id = series_named(&store, "Postcodes").id.clone();
    assert_eq!(
        series_named(&store, "Postcodes").data_type,
        DataType::Integer
    );

    store
        .apply(Operation::SetSeriesType {
            object_id: id.clone(),
            data_type: DataType::String,
        })
        .unwrap();
    assert_eq!(
        series_named(&store, "Postcodes").data_type,
        DataType::String
    );
    assert_eq!(
        series_named(&store, "Postcodes").values,
        vec!["01234", "00567"],
        "retyping does not touch what was written"
    );

    store
        .apply(Operation::SetSeries {
            object_id: id.clone(),
            values: "alpha, beta".into(),
        })
        .unwrap();
    let error = store
        .apply(Operation::SetSeriesType {
            object_id: id,
            data_type: DataType::Number,
        })
        .unwrap_err()
        .to_string();
    assert!(error.contains("‘alpha’ is not a number"), "{error}");
}

/// A list a formula reads cannot be deleted out from under it, the same way
/// a value cannot — and undo puts back what a rewrite replaced.
#[test]
fn a_list_is_held_in_place_by_what_reads_it_and_survives_undo() {
    let (mut store, frame_id) = orders_store();
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddSeries {
            name: "Domestic".into(),
            values: "USD, CAD".into(),
            x: 400.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    let series_id = series_named(&store, "Domestic").id.clone();
    store
        .apply(Operation::AddComputedColumn {
            frame_id,
            name: "At home".into(),
            formula: "`Currency`.is_in(`Domestic`)".into(),
            after_column_id: None,
        })
        .unwrap();

    assert!(matches!(
        store.apply(Operation::DeleteObject {
            object_id: series_id.clone(),
        }),
        Err(CoreError::ReferencedByFormula(_))
    ));

    store
        .apply(Operation::SetSeries {
            object_id: series_id,
            values: "GBP".into(),
        })
        .unwrap();
    assert_eq!(series_named(&store, "Domestic").values, vec!["GBP"]);
    store.undo();
    assert_eq!(series_named(&store, "Domestic").values, vec!["USD", "CAD"]);
}
