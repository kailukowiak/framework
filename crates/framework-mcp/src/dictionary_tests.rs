    #[test]
    fn generic_operation_surface_tracks_and_applies_the_canonical_enum() {
        let (server, path) = test_server();
        let catalog = server.describe_operations().unwrap().0;
        assert!(catalog.type_script.contains("targetId?: string"));
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
    fn matrix_input_bindings_use_the_canonical_operation_surface() {
        let (server, path) = test_server();
        for operation in [
            serde_json::json!({"type":"addVariable","name":"growth","formula":"slider(0,1,0.1,0.2)","x":0,"y":0}),
            serde_json::json!({"type":"addCalculationMatrix","name":"Sensitivity","x":0,"y":0}),
        ] {
            server.apply_operation(Parameters(ApplyOperationArgs { operation, expected_revision: None })).unwrap();
        }
        let (input, matrix) = {
            let session = server.lock().unwrap();
            let objects = &session.store.document().objects;
            (objects.iter().find(|o| o.name() == "growth").unwrap().id().to_string(),
             objects.iter().find(|o| o.name() == "Sensitivity").unwrap().id().to_string())
        };
        server.apply_operation(Parameters(ApplyOperationArgs {
            operation: serde_json::json!({"type":"setCalculationMatrix","objectId":matrix,"rows":[{"name":"r","formula":"[0.1,0.3]","targetId":input}],"columns":[],"body":"`growth` * 100"}), expected_revision: None,
        })).unwrap();
        let session = server.lock().unwrap();
        let view = session.store.view();
        assert_eq!(view.computed_calculation_matrices[&matrix].cells[1][0].value, Some(30.0));
        assert_eq!(view.computed_results[&input].cell.value, Some(0.2));
        drop(session);
        if path.exists() { std::fs::remove_file(path).unwrap(); }
    }

    #[test]
    fn parameter_values_use_the_canonical_typed_operation() {
        let (server, path) = test_server();
        let catalog = server.describe_operations().unwrap().0.type_script;
        assert!(catalog.contains(r#""type": "setParameterValue""#));
        assert!(catalog.contains("ScalarValue"));
        server.apply_operation(Parameters(ApplyOperationArgs {
            operation: serde_json::json!({ "type":"addVariable", "name":"growth", "formula":"slider(0,1,0.1)", "x":0, "y":0 }),
            expected_revision: Some(0),
        })).unwrap();
        let object_id = server.lock().unwrap().store.view().parameter_inputs[0].id.clone();
        server.apply_operation(Parameters(ApplyOperationArgs {
            operation: serde_json::json!({ "type":"setParameterValue", "objectId":object_id, "value":{"type":"number","value":0.25} }),
            expected_revision: Some(1),
        })).unwrap();
        assert_eq!(server.lock().unwrap().store.view().computed_results[&object_id].cell.value, Some(0.25));
        assert!(server.apply_operation(Parameters(ApplyOperationArgs {
            operation: serde_json::json!({ "type":"setParameterValue", "objectId":object_id, "value":{"type":"string","value":"bad"} }),
            expected_revision: Some(2),
        })).is_err());
        assert_eq!(server.inspect_document().unwrap().0.revision, 2);
        if path.exists() { std::fs::remove_file(path).unwrap(); }
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

    #[test]
    fn mapping_rename_is_discoverable_and_uses_the_canonical_operation() {
        let (server, path) = test_server();
        assert!(server.describe_operations().unwrap().0.type_script.contains(r#""type": "renameColumnsUsingMapping""#));
        for operation in [
            serde_json::json!({"type":"addFrame","name":"Data","grid":[["Old"],["42"]],"x":0,"y":0}),
            serde_json::json!({"type":"addDictionary","name":"Names","x":0,"y":0}),
        ] {
            server.apply_operation(Parameters(ApplyOperationArgs { operation, expected_revision: None })).unwrap();
        }
        let (target, mapping) = {
            let session = server.lock().unwrap();
            let find = |name: &str| session.store.document().objects.iter().find_map(|object| match object {
                DataObject::Frame(frame) if frame.name == name => Some(frame.clone()), _ => None,
            }).unwrap();
            (find("Data"), find("Names"))
        };
        server.apply_operation(Parameters(ApplyOperationArgs {
            operation: serde_json::json!({"type":"pasteCells","frameId":mapping.id,"rowId":mapping.rows[0].id,"columnId":mapping.columns[0].id,"grid":[["Old","New"]]}), expected_revision: None,
        })).unwrap();
        server.apply_operation(Parameters(ApplyOperationArgs {
            operation: serde_json::json!({"type":"renameColumnsUsingMapping","frameId":target.id,"mappingFrameId":mapping.id,"keyColumnId":mapping.columns[0].id,"valueColumnId":mapping.columns[1].id}), expected_revision: None,
        })).unwrap();
        let session = server.lock().unwrap();
        let frame = session.store.document().frame(&target.id).unwrap();
        assert_eq!(frame.columns[0].name, "New");
        assert_eq!(frame.columns[0].id, target.columns[0].id);
        drop(session);
        if path.exists() { std::fs::remove_file(path).unwrap(); }
    }

    #[test]
    fn period_declaration_is_discoverable_and_uses_the_canonical_operation() {
        let (server, path) = test_server();
        assert!(server.describe_operations().unwrap().0.type_script.contains(r#""type": "setFramePeriod""#));
        server.apply_operation(Parameters(ApplyOperationArgs {
            operation: serde_json::json!({"type":"addFrame","name":"Actuals","grid":[["Month","Revenue"],["2025-01-01","100"],["2025-02-01","104"]],"x":0,"y":0}), expected_revision: None,
        })).unwrap();
        let frame = {
            let session = server.lock().unwrap();
            session.store.document().objects.iter().find_map(|object| match object {
                DataObject::Frame(frame) if frame.name == "Actuals" => Some(frame.clone()), _ => None,
            }).unwrap()
        };
        server.apply_operation(Parameters(ApplyOperationArgs {
            operation: serde_json::json!({"type":"setFramePeriod","frameId":frame.id,"period":{"columnId":frame.columns[0].id,"partitionColumnIds":[]}}), expected_revision: None,
        })).unwrap();
        let session = server.lock().unwrap();
        let declared = session.store.document().frame(&frame.id).unwrap();
        assert_eq!(declared.period.as_ref().unwrap().column_id, frame.columns[0].id);
        drop(session);
        if path.exists() { std::fs::remove_file(path).unwrap(); }
    }
