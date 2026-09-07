//! Generate the dictionary interaction fixture through public operations.
use framework_core::{Document, Operation, Store};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut store = Store::new(Document::blank("Dictionary examples"));
    store.apply(Operation::AddDictionary {
        name: "Category fixes".into(),
        x: 0.0,
        y: 0.0,
    })?;
    store.apply(Operation::AddFrame {
        name: "Sales".into(),
        grid: vec![
            vec!["Category".into(), "Amount".into()],
            vec!["Travel".into(), "10".into()],
        ],
        x: 360.0,
        y: 0.0,
    })?;
    std::fs::write(
        "src/test/fixtures/dictionaries.json",
        serde_json::to_string_pretty(&store.view())?,
    )?;
    Ok(())
}
