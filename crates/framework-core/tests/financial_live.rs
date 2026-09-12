use crate::common::frame_named;
use framework_core::*;

fn answers(store: &Store) -> Vec<f64> {
    store
        .view()
        .computed_blocks
        .values()
        .next()
        .unwrap()
        .lines
        .iter()
        .map(|line| {
            assert!(line.cell.error.is_none(), "{:?}", line.cell.error);
            line.cell.value.unwrap()
        })
        .collect()
}

#[test]
fn dated_values_follow_derived_flows_and_survive_history_and_reload() {
    let mut store = Store::new(Document::blank("Live valuation"));
    store
        .apply(Operation::AddFrame {
            name: "Flows".into(),
            grid: vec![
                vec!["Date".into(), "Amount".into()],
                vec!["2025-01-01".into(), "-100".into()],
                vec!["2026-01-01".into(), "110".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Flows").clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: frame.id.clone(),
            name: "Derived".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_named(store.document(), "Derived").id.clone(),
            steps: vec![calculation("Doubled", "`Amount` * 2")],
        })
        .unwrap();
    store
        .apply(Operation::AddBlock {
            name: "Valuation".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let block = store
        .document()
        .objects
        .iter()
        .find(|o| o.name() == "Valuation")
        .unwrap()
        .id()
        .to_string();
    store
        .apply(Operation::SetBlockSource {
            block_id: block,
            source: "npv = `Derived`.`Doubled`.finance.xnpv(0.1, `Derived`.`Date`)\nreturn = `Derived`.`Doubled`.finance.irr()\ndownstream = npv + 10"
                .into(),
            editing: None,
        })
        .unwrap();
    assert!(answers(&store)[0].abs() < 1e-9);
    assert!((answers(&store)[1] - 0.1).abs() < 1e-9);
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: frame.rows[1].id.clone(),
            column_id: frame.columns[1].id.clone(),
            raw: "121".into(),
        })
        .unwrap();
    assert!((answers(&store)[0] - 20.0).abs() < 1e-9);
    assert!((answers(&store)[1] - 0.21).abs() < 1e-9);
    assert!((answers(&store)[2] - 30.0).abs() < 1e-9);
    store.undo();
    assert!(answers(&store)[0].abs() < 1e-9);
    store.redo();
    let directory = std::env::temp_dir().join(format!("framework-finance-{}", id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("valuation.fw");
    store.save(&path).unwrap();
    let loaded = Store::load(&path).unwrap();
    assert!((answers(&loaded)[0] - 20.0).abs() < 1e-9);
    assert!((answers(&loaded)[1] - 0.21).abs() < 1e-9);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn financial_column_expressions_broadcast_and_reconcile_a_loan() {
    let mut store = Store::new(Document::blank("Loan"));
    store
        .apply(Operation::AddGeneratorFrame {
            name: "Loan".into(),
            formula: "sequence(1, 61)".into(),
            column_name: Some("Period".into()),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Loan").id.clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![
                calculation("Interest", "ipmt(0.005, `Period`, 60, -400000)"),
                calculation("Principal", "ppmt(0.005, `Period`, 60, -400000)"),
            ],
        })
        .unwrap();
    let page = store.get_frame_page(&frame_id, 0, 100).unwrap();
    let principal: f64 = page.rows.iter().map(|r| r[2].parse::<f64>().unwrap()).sum();
    let interest: f64 = page.rows.iter().map(|r| r[1].parse::<f64>().unwrap()).sum();
    // Independent amortisation checkpoints from the finance lesson.
    assert!((principal - 400000.0).abs() < 0.01);
    assert!((interest - 63987.2367).abs() < 0.01);
}

fn calculation(name: &str, formula: &str) -> FrameStepInput {
    FrameStepInput::WithColumns {
        columns: vec![ExistingFormulaInput {
            output_column_id: id(),
            name: name.into(),
            formula: formula.into(),
        }],
    }
}

#[test]
fn imported_flows_remain_referenceable_and_update_after_refresh() {
    let directory = std::env::temp_dir().join(format!("framework-finance-import-{}", id()));
    std::fs::create_dir_all(&directory).unwrap();
    let source = directory.join("flows.csv");
    let artifacts = directory.join("artifacts");
    std::fs::write(&source, "Date,Amount\n2025-01-01,-100\n2026-01-01,110\n").unwrap();
    let mut store = Store::new(Document::blank("Imported valuation"));
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Flows".into(),
            artifact: create_data_artifact(&source, &artifacts).unwrap(),
            connector: Some(ConnectorRecipe::File {
                source_path: source.display().to_string(),
            }),
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Flows").id.clone();
    assert!(!frame_named(store.document(), "Flows").owns_its_rows());
    store
        .apply(Operation::AddBlock {
            name: "Value".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let block_id = store
        .document()
        .objects
        .iter()
        .find(|o| o.name() == "Value")
        .unwrap()
        .id()
        .to_string();
    store
        .apply(Operation::SetBlockSource {
            block_id,
            // CSV artifacts retain text dates. Convert explicitly, as in a
            // Wrangle formula, rather than assuming an Excel date serial.
            source: "value = `Flows`.`Amount`.finance.xnpv(0.1, `Flows`.`Date`.str.to_date())\nreturn = `Flows`.`Amount`.finance.xirr(`Flows`.`Date`.str.to_date())".into(),
            editing: None,
        })
        .unwrap();
    assert!(answers(&store)[0].abs() < 1e-9);
    assert!((answers(&store)[1] - 0.1).abs() < 1e-9);
    std::fs::write(&source, "Date,Amount\n2025-01-01,-100\n2026-01-01,121\n").unwrap();
    store
        .apply(Operation::RefreshFrameArtifact {
            frame_id: frame_id.clone(),
            artifact: create_data_artifact(&source, &artifacts).unwrap(),
        })
        .unwrap();
    assert!((answers(&store)[0] - 10.0).abs() < 1e-9);
    assert!((answers(&store)[1] - 0.21).abs() < 1e-9);
    assert!(!store.view().computed_frames[&frame_id].editing.cells);
    std::fs::remove_dir_all(directory).unwrap();
}
