use framework_core::{Document, Operation, ScalarValue, Store};
use std::path::Path;

pub(super) fn write(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut store = Store::new(Document::blank("Parameters"));
    store.apply(Operation::AddBlock {
        name: "Inputs".into(),
        x: 0.0,
        y: 0.0,
    })?;
    store.apply(Operation::SetBlockSource {
        block_id: super::block_id(&store, "Inputs"),
        source: "growth = slider(0,1,0.1,value=0.2)\nregion = dropdown([\"North\",\"South\"])\ncutoff = date_input(date(2026,12,31))\nanswer = growth * 100".into(),
        editing: None,
    })?;
    super::write_view(&store, &output.join("parameters.json"))?;
    let inputs = store.view().parameter_inputs;
    store.apply(Operation::SetParameterValue { object_id: inputs[0].id.clone(), value: ScalarValue::Number(0.4) })?;
    store.apply(Operation::SetParameterValue { object_id: inputs[2].id.clone(), value: ScalarValue::Date(chrono::NaiveDate::from_ymd_opt(2027,1,1).unwrap()) })?;
    super::write_view(&store, &output.join("parameters-selected.json"))
}
