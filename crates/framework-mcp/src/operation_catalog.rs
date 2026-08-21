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
    let mut collector = DeclarationCollector {
        config: &config,
        seen: HashSet::new(),
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
        assert!(catalog.contains(r#""type": "setFramePipeline""#));
        assert!(catalog.contains("type FrameStepInput ="));
        assert!(catalog.contains(r#""kind": "expand""#));
        assert!(catalog.contains("type DataArtifact ="));
    }
}
