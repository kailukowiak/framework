use framework_core::*;

#[test]
fn integer_seed_promotes_before_fractional_steps_including_restarts() {
    for seed in ["100", "`Seed`"] {
        let mut store = Store::new(Document::blank("Precision"));
        store
            .apply(Operation::AddFrame {
                name: "Balances".into(),
                grid: vec![
                    vec!["Order".into(), "Group".into(), "Seed".into()],
                    vec!["1".into(), "A".into(), "100".into()],
                    vec!["2".into(), "B".into(), "100".into()],
                    vec!["3".into(), "A".into(), "100".into()],
                    vec!["4".into(), "B".into(), "100".into()],
                    vec!["5".into(), "A".into(), "100".into()],
                ],
                x: 0.0,
                y: 0.0,
            })
            .unwrap();
        let frame = store
            .document()
            .objects
            .iter()
            .find_map(|o| {
                if let DataObject::Frame(f) = o {
                    Some(f.clone())
                } else {
                    None
                }
            })
            .unwrap();
        let output = id();
        store
            .apply(Operation::SetFramePipeline {
                frame_id: frame.id.clone(),
                steps: vec![
                    FrameStepInput::Sort {
                        keys: vec![SortInput {
                            column_id: frame.columns[0].id.clone(),
                            descending: false,
                        }],
                    },
                    FrameStepInput::WithColumns {
                        columns: vec![ExistingFormulaInput {
                            output_column_id: output.clone(),
                            name: "Balance".into(),
                            formula: format!(
                                "recur({seed}, previous() * 1.005, restart_by=[`Group`])"
                            ),
                        }],
                    },
                ],
            })
            .unwrap();
        let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
        let index = page.columns.iter().position(|c| c.id == output).unwrap();
        for (row, expected) in page.rows.iter().zip([100.0, 100.0, 100.5, 100.5, 101.0025]) {
            assert!(
                (row[index].parse::<f64>().unwrap() - expected).abs() < 1e-6,
                "{seed}: {:?}",
                row
            );
        }
    }
}
