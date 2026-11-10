//! The complete machine-facing mutation contract.
//!
//! `Operation` is already the serialized boundary shared by the desktop,
//! history and collaboration. Repeating its variants in an MCP-only catalog
//! would recreate the drift this adapter exists to avoid, so this module asks
//! the same `ts-rs` implementation that generates the frontend binding for a
//! declaration and recursively includes every named type it references.

use framework_core::Operation;
use std::{any::TypeId, collections::HashSet};
use ts_rs::{Config, TS, TypeVisitor};

struct DeclarationCollector<'a> {
    config: &'a Config,
    seen: HashSet<TypeId>,
    declarations: Vec<String>,
}

impl TypeVisitor for DeclarationCollector<'_> {
    fn visit<T: TS + 'static + ?Sized>(&mut self) {
        if T::output_path().is_none() || !self.seen.insert(TypeId::of::<T>()) {
            return;
        }
        self.declarations.push(T::decl(self.config));
        T::visit_dependencies(self);
    }
}

/// A self-contained TypeScript description of the JSON accepted by
/// `apply_operation`, generated from the canonical Rust enum and its inputs.
pub fn operation_typescript() -> String {
    let config = Config::default();
    // `Operation` is seen before the walk starts because a batch holds
    // operations: without this the walk would reach the union through its
    // own `batch` variant and declare it twice.
    let mut collector = DeclarationCollector {
        config: &config,
        seen: HashSet::from([TypeId::of::<Operation>()]),
        declarations: Vec::new(),
    };
    Operation::visit_dependencies(&mut collector);
    collector.declarations.sort();
    collector.declarations.push(Operation::decl(&config));
    collector.declarations.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_follows_simple_and_nested_operation_variants() {
        let catalog = operation_typescript();
        assert!(catalog.contains(r#""type": "renameColumn""#));
        assert!(catalog.contains(r#""type": "renameColumns""#));
        assert!(catalog.contains(r#""type": "importFrameFromFile""#));
        assert!(catalog.contains(r#""type": "importFrameFromArtifact""#));
        assert!(catalog.contains(r#""type": "foldRowPatches""#));
        assert!(catalog.contains(r#""type": "setFramePipeline""#));
        assert!(catalog.contains("type FrameStepInput ="));
        assert!(catalog.contains(r#""kind": "expand""#));
        assert!(catalog.contains("type DataArtifact ="));
        // Scenarios reach an agent through the generic surface alone, so
        // the catalog naming them is the whole of their discoverability.
        for scenario_operation in [
            r#""type": "addScenario""#,
            r#""type": "removeScenario""#,
            r#""type": "renameScenario""#,
            r#""type": "setScenarioValue""#,
            r#""type": "activateScenario""#,
        ] {
            assert!(catalog.contains(scenario_operation), "{scenario_operation}");
        }
    }

    /// The grid-shape edits — a row placed above another, several rows
    /// deleted as one, a column placed before another — reach an agent only
    /// through this catalog, so it has to spell each of them.
    #[test]
    fn catalog_spells_the_grid_shape_edits() {
        let catalog = operation_typescript();
        assert!(catalog.contains(r#""type": "deleteRows""#));
        assert!(catalog.contains("rowIds: Array<string>"));
        assert!(catalog.contains("beforeRowId?: string | null"));
        assert!(catalog.contains("beforeColumnId?: string | null"));
    }

    /// `addColumn` can carry the new column's first values, keyed by row id,
    /// which is the only way a batch can fill a column whose id the engine
    /// has not minted yet.
    #[test]
    fn catalog_spells_add_column_values() {
        let catalog = operation_typescript();
        let add_column = catalog
            .split(r#""type": "addColumn""#)
            .nth(1)
            .and_then(|rest| rest.split(r#""type": "deleteColumn""#).next())
            .expect("addColumn is in the catalog");
        assert!(add_column.contains("values?: { [key in string]: string }"));
    }

    /// Column widths, range formats and batches reach an agent only
    /// through this catalog. A batch holds operations, so the union is
    /// declared once and referenced from inside itself.
    #[test]
    fn catalog_spells_layout_range_formats_and_batches() {
        let catalog = operation_typescript();
        assert!(catalog.contains(r#""type": "setFrameDisplayColumnLayout""#));
        assert!(catalog.contains("type FrameColumnLayoutUpdate ="));
        assert!(catalog.contains(r#""kind": "range""#));
        assert!(catalog.contains(r#""type": "batch""#));
        assert!(catalog.contains("operations: Array<Operation>"));
        assert_eq!(catalog.matches("type Operation =").count(), 1);
    }

    #[test]
    fn models_use_the_canonical_nested_operation_contract() {
        let catalog = operation_typescript();
        for operation in [
            "addModel",
            "setModelSpec",
            "fitModel",
            "importModel",
            "importOnnxModel",
            "addModelPredictions",
            "addModelSummary",
            "addStatisticalAnalysis",
        ] {
            assert!(
                catalog.contains(&format!(r#""type": "{operation}""#)),
                "{operation}"
            );
        }
        for declaration in [
            "type ModelSpec =",
            "type Method =",
            "type CovarianceMethod =",
            "type IterationRange =",
            "type StatsRequest =",
        ] {
            assert!(catalog.contains(declaration), "{declaration}");
        }
    }
}
