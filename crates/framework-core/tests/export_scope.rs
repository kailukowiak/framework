use crate::common::*;
use framework_core::*;
use std::fs;

#[test]
fn export_scope_counts_and_rows_agree_with_pipeline_filters_and_sort() {
    let directory = temporary_test_directory("export-scope");
    let mut store = Store::new(Document::blank("Export scope"));
    store
        .apply(Operation::AddFrame {
            name: "Items".into(),
            grid: vec![
                vec!["Item".into(), "Amount".into()],
                vec!["Keep B".into(), "3".into()],
                vec!["Remove".into(), "99".into()],
                vec!["Keep A".into(), "2".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Items").clone();
    store
        .apply(Operation::SetFrameDisplayFilter {
            frame_id: frame.id.clone(),
            filters: vec!["`Item` != \"Remove\"".into()],
            filter_match_all: true,
        })
        .unwrap();
    store
        .apply(Operation::SetFrameDisplaySort {
            frame_id: frame.id.clone(),
            keys: vec![DerivedSort {
                column_id: frame.columns[0].id.clone(),
                descending: false,
            }],
        })
        .unwrap();
    let before = store.document().clone();
    assert_eq!(store.export_row_count(&frame.id, true).unwrap(), 2);
    assert_eq!(store.export_row_count(&frame.id, false).unwrap(), 2);
    let filtered = directory.join("view.xlsx");
    let entire = directory.join("all.xlsx");
    store
        .export_excel_scoped(std::slice::from_ref(&frame.id), &filtered, false, true)
        .unwrap();
    store
        .export_excel_scoped(std::slice::from_ref(&frame.id), &entire, false, false)
        .unwrap();
    assert_eq!(
        preview_excel_range(&filtered, "Items", "A1:B3", true, 10)
            .unwrap()
            .rows,
        vec![vec!["Keep A", "2"], vec!["Keep B", "3"]]
    );
    assert_eq!(
        preview_excel_range(&entire, "Items", "A1:B3", true, 10)
            .unwrap()
            .rows,
        vec![vec!["Keep A", "2"], vec!["Keep B", "3"]]
    );
    assert_eq!(store.document(), &before);
    fs::remove_dir_all(directory).unwrap();
}
