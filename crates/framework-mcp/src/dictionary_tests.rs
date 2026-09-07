    #[test]
    fn generic_operation_surface_tracks_and_applies_the_canonical_enum() {
        let (server, path) = test_server();
        let catalog = server.describe_operations().unwrap().0;
        assert!(catalog.type_script.contains(r#""type": "renameDocument""#));
        assert!(
            catalog
                .type_script
                .contains(r#""type": "setFramePipeline""#)
        );

        let receipt = server
            .apply_operation(Parameters(ApplyOperationArgs {
                operation: serde_json::json!({
                    "type": "renameDocument",
                    "name": "Named through the operation API"
                }),
                expected_revision: Some(0),
            }))
            .unwrap()
            .0;
        assert_eq!(receipt.revision, 1);
        assert_eq!(
            server.inspect_document().unwrap().0.name,
            "Named through the operation API"
        );
        if path.exists() {
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn dictionaries_are_available_through_the_canonical_operation_catalog() {
        let (server, path) = test_server();
        assert!(server.describe_operations().unwrap().0.type_script.contains(r#""type": "addDictionary""#));
        server.apply_operation(Parameters(ApplyOperationArgs {
            operation: serde_json::json!({ "type": "addDictionary", "name": "Corrections", "x": 0, "y": 0 }), expected_revision: Some(0),
        })).unwrap();
        let snapshot = server.get_frame(Parameters(GetFrameArgs { frame: "Corrections".into(), limit: Some(10) })).unwrap().0;
        assert_eq!(snapshot.total_row_count, 1);
        let session = server.lock().unwrap();
        let frame = session.store.document().objects.iter().find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == "Corrections" => Some(frame), _ => None,
        }).unwrap();
        assert_eq!(frame.unique_keys[0].column_ids, vec![frame.columns[0].id.clone()]);
        drop(session);
        if path.exists() { std::fs::remove_file(path).unwrap(); }
    }
