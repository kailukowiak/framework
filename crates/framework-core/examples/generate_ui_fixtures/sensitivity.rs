use framework_core::*;
use std::path::Path;

pub(super) fn write(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut store = Store::new(Document::blank("Sensitivity"));
    store.apply(Operation::AddVariable {
        name: "growth".into(),
        formula: "slider(0,1,0.1,0.2)".into(),
        x: 0.0,
        y: 0.0,
    })?;
    let target_id = store.document().objects[0].id().to_string();
    store.apply(Operation::AddCalculationMatrix {
        name: "Sensitivity".into(),
        x: 0.0,
        y: 0.0,
    })?;
    let object_id = store.document().objects[1].id().to_string();
    store.apply(Operation::SetCalculationMatrix {
        object_id,
        rows: vec![CalculationMatrixFormulaInput {
            id: None,
            name: "Rate".into(),
            formula: "[0.1,0.3]".into(),
            target_id: Some(target_id),
        }],
        columns: vec![],
        body: "100 * (1 + `growth`)".into(),
    })?;
    super::write_view(&store, &output.join("matrix-sensitivity.json"))
}
