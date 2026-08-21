use framework_core::{
    DataObject, Document, ExistingFormulaInput, FrameStepInput, Operation, SortInput, Store,
    column_id,
};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let output = workspace.join("tutorials/formula-clicks");

    // Build through public operations, the same mutation boundary MCP uses.
    // That keeps these files honest examples of what an agent can construct,
    // rather than fixtures with privileged model state no user can reproduce.
    let mut store = Store::new_tutorial(Document::blank("Formula clicks tutorial"));
    store.apply(Operation::AddFrame {
        name: "Monthly sales".into(),
        grid: vec![
            vec!["Month", "Region", "Revenue", "Cost"],
            vec!["2026-04", "West", "142000", "91000"],
            vec!["2026-01", "East", "118000", "76000"],
            vec!["2026-06", "East", "168000", "104000"],
            vec!["2026-03", "East", "136000", "85000"],
            vec!["2026-02", "West", "124000", "79000"],
            vec!["2026-05", "East", "151000", "96000"],
        ]
        .into_iter()
        .map(|row| row.into_iter().map(str::to_string).collect())
        .collect(),
        x: 80.0,
        y: 80.0,
    })?;
    store.apply(Operation::AddBlock {
        name: "Checks".into(),
        x: 80.0,
        y: 430.0,
    })?;

    let start = output.join("formula-clicks-start.fw");
    store.save(&start)?;

    let sales = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == "Monthly sales" => Some(frame.clone()),
            _ => None,
        })
        .expect("tutorial frame exists");
    let column = |name: &str| {
        sales
            .columns
            .iter()
            .find(|column| column.name == name)
            .expect("tutorial column exists")
            .id
            .clone()
    };
    let month_id = column("Month");
    let previous_id = column_id("Previous revenue");
    let change_id = column_id("Change");

    store.apply(Operation::SetFramePipeline {
        frame_id: sales.id.clone(),
        steps: vec![
            FrameStepInput::Sort {
                keys: vec![SortInput {
                    column_id: month_id,
                    descending: false,
                }],
            },
            FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: previous_id.clone(),
                    name: "Previous revenue".into(),
                    formula: "`Revenue`.shift(1)".into(),
                }],
            },
            FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: change_id,
                    name: "Change".into(),
                    formula: "`Revenue` - `Previous revenue`".into(),
                }],
            },
        ],
    })?;

    let sales_view_id = store
        .document()
        .views
        .iter()
        .find(|view| view.object_id == sales.id)
        .expect("tutorial frame view exists")
        .id
        .clone();
    store.apply(Operation::BranchFrame {
        view_id: sales_view_id.clone(),
        frame_id: sales.id.clone(),
    })?;
    let branch = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame)
                if frame.id != sales.id
                    && frame
                        .derivation
                        .as_ref()
                        .is_some_and(|derivation| derivation.source_frame_id == sales.id) =>
            {
                Some(frame.clone())
            }
            _ => None,
        })
        .expect("branched tutorial frame exists");
    store.apply(Operation::RenameObject {
        object_id: branch.id.clone(),
        name: "East only".into(),
    })?;

    let passthrough = branch
        .columns
        .iter()
        .map(|output| ExistingFormulaInput {
            output_column_id: output.id.clone(),
            name: output.name.clone(),
            formula: format!("`{}`", output.name.replace('`', "``")),
        })
        .collect();
    let selected = branch
        .columns
        .iter()
        .map(|column| column.id.clone())
        .collect();
    store.apply(Operation::SetFramePipeline {
        frame_id: branch.id,
        steps: vec![
            FrameStepInput::WithColumns {
                columns: passthrough,
            },
            FrameStepInput::Select {
                column_ids: selected,
            },
            FrameStepInput::Filter {
                predicates: vec!["`Region` == \"East\"".into()],
                match_all: true,
            },
        ],
    })?;

    let checks = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Block(block) if block.name == "Checks" => Some(block.id.clone()),
            _ => None,
        })
        .expect("tutorial block exists");
    store.apply(Operation::SetBlockSource {
        block_id: checks.clone(),
        source: "Total revenue = `Monthly sales`.`Revenue`.sum()\nLatest revenue = `Monthly sales`.`Revenue`.last()".into(),
        editing: None,
    })?;

    // Scratchwork is deliberately live. The finished tutorial keeps these
    // checks as formulas rather than recorded artifacts so editing Monthly
    // sales immediately changes the answers it teaches people to inspect.

    // The answer key should show the two calculated columns without making
    // somebody hunt for a horizontal scrollbar. This is presentation only;
    // the starting workbook keeps the compact default size used by the
    // tutorial's first screen.
    store.apply(Operation::ResizeView {
        view_id: sales_view_id,
        width: 920.0,
        height: 330.0,
    })?;
    let checks_view_id = store
        .document()
        .views
        .iter()
        .find(|view| view.object_id == checks)
        .expect("tutorial block view exists")
        .id
        .clone();
    store.apply(Operation::ResizeView {
        view_id: checks_view_id,
        width: 480.0,
        height: 220.0,
    })?;

    let finished = output.join("formula-clicks-finished.fw");
    store.save(&finished)?;

    // Loading and planning both documents catches format drift and formulas
    // whose names were never actually bound through the public operation path.
    let start_store = Store::load(&start)?;
    let finished_store = Store::load(&finished)?;
    assert_eq!(start_store.document().name, "Formula clicks tutorial");
    assert!(finished_store.get_frame_page(&sales.id, 0, 20).is_ok());
    println!("wrote {}", start.display());
    println!("wrote {}", finished.display());
    Ok(())
}
