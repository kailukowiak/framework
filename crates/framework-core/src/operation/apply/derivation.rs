//! Applying already-resolved `ReplicatedOperation`s in this family.
//!
//! Derived frames — transformation chains, joins, unique keys, and the
//! parquet snapshots a frame can be materialized into.

use crate::*;
use std::collections::HashSet;

impl Document {
    pub(crate) fn apply_set_unique_keys(
        &mut self,
        frame_id: Id,
        unique_keys: Vec<UniqueKeyConstraint>,
    ) -> Result<(), CoreError> {
        let frame = self.frame_mut(&frame_id)?;
        frame.unique_keys = unique_keys;
        Ok(())
    }

    pub(crate) fn apply_set_frame_generator(
        &mut self,
        frame_id: Id,
        generator: FrameGenerator,
        columns: Vec<Column>,
    ) -> Result<(), CoreError> {
        let frame = self.frame_mut(&frame_id)?;
        if frame.generator.is_none() {
            return Err(CoreError::InvalidOperation(
                "Only a generated frame has a rule to replace".into(),
            ));
        }
        frame.generator = Some(generator);
        frame.columns = columns;
        // The rows a view may have cached under the old rule mean nothing
        // under the new one.
        frame.rows.clear();
        Ok(())
    }

    pub(crate) fn apply_set_frame_derivation(
        &mut self,
        frame_id: Id,
        name: String,
        columns: Vec<Column>,
        derivation: FrameDerivation,
    ) -> Result<(), CoreError> {
        let frame = self.frame(&frame_id)?;
        if frame.derivation.is_none() {
            return Err(CoreError::InvalidOperation(
                "Only a derived frame has a transformation to update".into(),
            ));
        }
        let next_ids = columns
            .iter()
            .map(|column| column.id.as_str())
            .collect::<HashSet<_>>();
        for removed in frame
            .columns
            .iter()
            .filter(|column| !next_ids.contains(column.id.as_str()))
        {
            let held_by = self
                .objects
                .iter()
                .find_map(|object| match object {
                    DataObject::Frame(candidate) => candidate
                        .wrangle_reads_foreign_column(&frame_id, &removed.id)
                        .then(|| as_named(&candidate.name)),
                    _ => None,
                })
                .or_else(|| {
                    frame
                        .display
                        .references_column(&removed.id)
                        .then(|| as_named(&frame.name))
                })
                .or_else(|| self.column_read_by(&frame_id, &removed.id));
            if let Some(reader) = held_by {
                return Err(CoreError::ReferencedByFormula(format!(
                    "{reader} reads {}, so these steps cannot stop producing it. \
                     Change what reads it first.",
                    as_named(&removed.name)
                )));
            }
        }
        let frame = self.frame_mut(&frame_id)?;
        frame.name = name;
        frame.columns = columns;
        frame.derivation = Some(derivation);
        frame.rows.clear();
        prune_entry_columns(frame);
        Ok(())
    }

    pub(crate) fn apply_set_frame_steps(
        &mut self,
        frame_id: Id,
        columns: Vec<Column>,
        base_columns: Vec<Column>,
        steps: Vec<FrameStep>,
    ) -> Result<(), CoreError> {
        let frame = self.frame(&frame_id)?;
        if frame.derivation.is_some() {
            return Err(CoreError::InvalidOperation(
                "A derived frame's chain is part of its derivation".into(),
            ));
        }
        // The stored rows and cells are untouched by a chain -- they
        // stay the frame's input -- so the only thing to protect is
        // a column another object reads by id disappearing from the
        // output.
        let next_ids = columns
            .iter()
            .map(|column| column.id.as_str())
            .collect::<HashSet<_>>();
        for removed in frame
            .columns
            .iter()
            .filter(|column| !next_ids.contains(column.id.as_str()))
        {
            let held_by = self
                .objects
                .iter()
                .find_map(|object| match object {
                    DataObject::Frame(candidate) => candidate
                        .wrangle_reads_foreign_column(&frame_id, &removed.id)
                        .then(|| as_named(&candidate.name)),
                    DataObject::Plot(plot) if plot.source_frame_id == frame_id => {
                        json_contains_string(&plot.spec, &removed.id).then(|| as_named(&plot.name))
                    }
                    _ => None,
                })
                .or_else(|| {
                    frame
                        .display
                        .references_column(&removed.id)
                        .then(|| as_named(&frame.name))
                })
                .or_else(|| self.column_read_by(&frame_id, &removed.id));
            if let Some(reader) = held_by {
                return Err(CoreError::ReferencedByFormula(format!(
                    "{reader} reads {}, so these steps cannot stop producing it. \
                     Change what reads it first.",
                    as_named(&removed.name)
                )));
            }
        }
        let frame = self.frame_mut(&frame_id)?;
        frame.columns = columns;
        frame.base_columns = base_columns;
        frame.steps = steps;
        prune_entry_columns(frame);
        Ok(())
    }

    pub(crate) fn apply_set_frame_materialization(
        &mut self,
        frame_id: Id,
        materialization: Option<Materialization>,
    ) -> Result<(), CoreError> {
        // The snapshot is what makes a frame readable from elsewhere, so
        // dropping one out from under a formula that reads it would break
        // that formula. Refreshing replaces the snapshot and is fine; only
        // going back to live is refused.
        if materialization.is_none()
            && let Some(reader) = self.frame_read_by(&frame_id)
        {
            return Err(CoreError::ReferencedByFormula(format!(
                "{reader} reads {}, and the snapshot is what makes it readable, so it \
                 cannot go back to live. Change the formula that reads it first.",
                as_named(&self.frame(&frame_id)?.name)
            )));
        }
        let frame = self.frame_mut(&frame_id)?;
        if materialization.is_some() && frame.derivation.is_none() {
            return Err(CoreError::InvalidOperation(
                "Only a derived frame can be cached to a snapshot".into(),
            ));
        }
        frame.materialization = materialization;
        Ok(())
    }
}

/// Drops entry columns whose own column or key columns a chain edit no
/// longer produces. Deterministic from the columns the operation carried,
/// so every replica prunes identically. Entries in a surviving column are
/// untouched — surviving a chain edit is what key-addressed storage is for.
fn prune_entry_columns(frame: &mut FrameObject) {
    let column_ids: HashSet<&str> = frame
        .columns
        .iter()
        .map(|column| column.id.as_str())
        .collect();
    frame.entry_columns.retain(|entry_column| {
        column_ids.contains(entry_column.column_id.as_str())
            && entry_column
                .key_column_ids
                .iter()
                .all(|key| column_ids.contains(key.as_str()))
    });
}
