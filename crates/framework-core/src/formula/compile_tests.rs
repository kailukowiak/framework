#[allow(unused_imports)]
use crate::test_support::*;
#[allow(unused_imports)]
use crate::*;
#[allow(unused_imports)]
use std::{fs, path::PathBuf};
#[allow(unused_imports)]
use uuid::Uuid;

#[test]
pub(crate) fn polars_methods_render_canonically_and_evaluate() {
    let mut store = demo_store();
    let frame_id = store
        .document
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
            name: "Quantity squared".into(),
            formula: "`Quantity`.pow(2).round(0)".into(),
            after_column_id: None,
        })
        .unwrap();
    let frame = view
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.id == frame_id => Some(frame),
            _ => None,
        })
        .unwrap();
    let column_id = frame
        .columns
        .iter()
        .find(|column| column.name == "Quantity squared")
        .unwrap()
        .id
        .clone();
    // Read back through `formulas`, which is where a rendered column
    // formula lives on a frame that owns its rows: the formula is on the
    // column itself rather than in a wrangle step, the same way the
    // demo's own Total column holds one. Rendering is the assertion --
    // an expression that parses and then writes itself back differently
    // is one somebody's document quietly rewrites every time it is read.
    let rendered = &view.computed_frames[&frame_id].formulas[&column_id];
    assert_eq!(rendered, "`Quantity`.pow(2).round(0)");
    let page = store.get_frame_page(&frame_id, 0, 10).unwrap();
    let output = page
        .columns
        .iter()
        .position(|column| column.id == column_id)
        .unwrap();
    let values = page
        .rows
        .iter()
        .map(|row| row[output].parse::<f64>().ok())
        .collect::<Vec<_>>();
    assert_eq!(values, vec![Some(9.0), Some(25.0), Some(4.0)]);

    let wrong_arity = store.apply(Operation::AddComputedColumn {
        frame_id,
        name: "Broken".into(),
        formula: "`Quantity`.sqrt(1, 2)".into(),
        after_column_id: None,
    });
    assert!(
        matches!(wrong_arity, Err(CoreError::Formula(message)) if message.contains("0 arguments"))
    );

    let catalog = formula_function_catalog();
    assert_eq!(
        catalog.len(),
        crate::formula::catalog::POLARS_FORMULA_FUNCTIONS.len()
            + 3 * crate::formula::catalog::financial::FUNCTIONS.len()
            + formula::generated_bindings::GENERATED_FORMULA_FUNCTIONS.len()
    );
    assert!(catalog.iter().any(|function| {
        function.id == "expr.pow" && function.aliases.contains(&"power".to_string())
    }));
    assert!(
        catalog
            .iter()
            .any(|function| function.id == "str.strip_chars")
    );
}
