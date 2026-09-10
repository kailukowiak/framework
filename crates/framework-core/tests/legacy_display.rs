use crate::common::*;
use framework_core::*;

#[test]
fn promoting_a_display_layer_moves_its_filter_and_sort_into_the_wrangle_chain() {
    let mut store = demo_store();
    add_roster(&mut store);
    let frame = frame_named(store.document(), "Roster").clone();
    let score_id = frame.columns[1].id.clone();
    let name_id = frame.columns[0].id.clone();

    store
        .apply(Operation::SetFrameDisplayFilter {
            frame_id: frame.id.clone(),
            filters: vec!["`Name` != \"Abe\"".into()],
            filter_match_all: true,
        })
        .unwrap();
    store
        .apply(Operation::SetFrameDisplaySort {
            frame_id: frame.id.clone(),
            keys: vec![DerivedSort {
                column_id: score_id.clone(),
                descending: true,
            }],
        })
        .unwrap();

    // Simulate a saved legacy display layer. New header commands author the
    // pipeline directly, but opening old notebooks must remain supported.
    let mut legacy = store.document().clone();
    for object in &mut legacy.objects {
        if let DataObject::Frame(input) = object
            && input.id == frame.id
        {
            input.display.steps = std::mem::take(&mut input.steps);
        }
    }
    store = Store::new(legacy);

    // While they are still display steps a frame derived from Roster does
    // not see them.
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: frame.id.clone(),
            name: "Downstream".into(),
            x: 600.0,
            y: 0.0,
        })
        .unwrap();
    let downstream = frame_named(store.document(), "Downstream").clone();
    assert_eq!(
        store
            .get_frame_page(&downstream.id, 0, 100)
            .unwrap()
            .total_rows,
        4,
        "a display filter does not reach the frame derived from it"
    );

    store
        .apply(Operation::PromoteDisplayToSteps {
            frame_id: frame.id.clone(),
        })
        .unwrap();

    let promoted = frame_named(store.document(), "Roster").clone();
    assert!(
        promoted.display.steps.is_empty(),
        "promotion empties the display layer"
    );
    assert_eq!(promoted.steps.len(), 2);
    assert!(
        matches!(&promoted.steps[1], FrameStep::Sort { keys } if keys[0].column_id == score_id
            && keys[0].descending)
    );
    assert!(matches!(promoted.steps[0], FrameStep::Filter { .. }));

    let page = store.get_frame_page(&frame.id, 0, 100).unwrap();
    let names = page
        .rows
        .iter()
        .map(|row| row[page.columns.iter().position(|c| c.id == name_id).unwrap()].clone())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["Cara", "Dee", "Bo"]);
    assert_eq!(
        store
            .get_frame_page(&downstream.id, 0, 100)
            .unwrap()
            .total_rows,
        3,
        "once promoted, the filter is lineage and the derived frame sees it"
    );
}

fn add_roster(store: &mut Store) {
    store
        .apply(Operation::AddFrame {
            name: "Roster".into(),
            grid: vec![
                vec![
                    "Name".into(),
                    "Score".into(),
                    "Active".into(),
                    "Joined".into(),
                ],
                vec![
                    "Dee".into(),
                    "20".into(),
                    "true".into(),
                    "2024-02-05".into(),
                ],
                vec![
                    "Cara".into(),
                    "50".into(),
                    "true".into(),
                    "2024-01-10".into(),
                ],
                vec!["Abe".into(), "".into(), "false".into(), "2024-03-01".into()],
                vec!["Bo".into(), "20".into(), "".into(), "".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
}
