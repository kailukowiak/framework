//! Applying the edits that change a frame's shape rather than its values:
//! where a new column goes, which rows go together, and the one rule that
//! decides whether a frame's literal columns are the document's to change.

use crate::*;
use std::collections::HashSet;

/// Where a new column lands among the frame's visible columns.
///
/// Three places rather than an index because an index means nothing to a
/// replica whose columns moved meanwhile; a neighbour's id does.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ColumnPlace {
    End,
    After(Id),
    Before(Id),
}

impl ColumnPlace {
    /// `after` wins when both are set. Prepare refuses that combination from
    /// a person; a replica is applying an already-decided fact and should not
    /// fail over a field it cannot have been sent by this build.
    pub(crate) fn new(after: Option<Id>, before: Option<Id>) -> Self {
        match (after, before) {
            (Some(after), _) => Self::After(after),
            (None, Some(before)) => Self::Before(before),
            (None, None) => Self::End,
        }
    }

    /// The index in `columns` the new column takes.
    fn index_in(&self, columns: &[Column]) -> Result<usize, CoreError> {
        let position = |id: &str| {
            columns
                .iter()
                .position(|column| column.id == id)
                .ok_or(CoreError::ColumnNotFound)
        };
        Ok(match self {
            Self::End => columns.len(),
            Self::After(id) => position(id)? + 1,
            Self::Before(id) => position(id)?,
        })
    }
}

impl FrameObject {
    /// Whether this frame's literal columns are the document's own to add,
    /// delete and retype.
    ///
    /// True when the document holds the rows and its chain only filters,
    /// sorts, annotates, or adds calculated columns. A calculation reads the
    /// literal columns; it does not take them over, so treating a frame with
    /// one as "computed" — which is what the column lifecycle used to do —
    /// meant the first *Add calculated column* froze the table's own schema
    /// while its cells stayed editable. A reshape (select, summarize, join,
    /// pivot, …) is different: it fixes which columns the frame shows, and a
    /// literal column added underneath it would be invisible or would change
    /// what the reshape means. A frame read from a file or grown from a rule
    /// does not own its columns at all.
    pub(crate) fn owns_its_columns(&self) -> bool {
        self.owns_its_rows()
            && self.steps.iter().all(|step| {
                matches!(
                    step,
                    FrameStep::Filter { .. }
                        | FrameStep::WithColumns { .. }
                        | FrameStep::Sort { .. }
                        | FrameStep::Comment { .. }
                )
            })
    }

    /// The guard every column-lifecycle apply shares. A frame with no chain
    /// or derivation keeps the latitude it has always had (an imported
    /// column can still be retyped); a computed one is refused unless the
    /// computation only adds columns to rows the document owns.
    pub(crate) fn column_schema_is_read_only(&self) -> bool {
        self.is_computed() && !self.owns_its_columns()
    }

    /// Copies a literal column's visible definition onto its entry in the
    /// input schema, after a rename or a retype.
    ///
    /// The chain is rendered back to text against the input schema, so a
    /// rename that reached only `columns` left every chain formula spelling
    /// the old name — and the next save of that chain parsed a name the frame
    /// no longer has. Only for a frame that owns its columns; an imported
    /// frame's input entry keeps the file's own field.
    pub(crate) fn mirror_input_column(&mut self, column_id: &str) {
        if !self.owns_its_columns() {
            return;
        }
        let Some(shown) = self.columns.iter().find(|column| column.id == column_id) else {
            return;
        };
        let shown = shown.clone();
        if let Some(input) = self
            .base_columns
            .iter_mut()
            .find(|column| column.id == column_id)
        {
            *input = shown;
        }
    }

    /// Whether this frame's own chain reads `column_id` — in a formula, a
    /// filter, or as a sort key.
    ///
    /// Asked before deleting a literal column. A chain formula naming a
    /// column that has gone does not fail loudly; it fails on every read
    /// after. Refusing the delete and naming the step is the same answer the
    /// document already gives for another column's formula or another
    /// frame's chain reading it.
    pub(crate) fn own_chain_reads_column(&self, column_id: &str) -> bool {
        self.steps.iter().any(|step| {
            crate::engine::frame::step_expressions(step)
                .any(|expression| expression.references_column(column_id))
                || matches!(
                    step,
                    FrameStep::Sort { keys } if keys.iter().any(|key| key.column_id == column_id)
                )
        })
    }
}

impl Document {
    /// Inserts a literal column into the frame's visible schema and, when the
    /// frame has a chain, into the input schema the chain starts from.
    ///
    /// The two lists have to move together: the base scan reads
    /// `input_columns`, so a column only in `columns` would be asked of a plan
    /// that never produced it. In the input list the column goes after the
    /// last input column that precedes it on screen, which keeps the two
    /// orders agreeing about every column they share.
    pub(crate) fn insert_literal_column(
        &mut self,
        frame_id: &str,
        column: Column,
        place: &ColumnPlace,
    ) -> Result<(), CoreError> {
        let frame = self.frame_mut(frame_id)?;
        if frame
            .columns
            .iter()
            .chain(frame.base_columns.iter())
            .any(|existing| existing.id == column.id)
        {
            return Err(CoreError::InvalidOperation(
                "column ID already exists in the target frame".into(),
            ));
        }
        let insert_at = place.index_in(&frame.columns)?;
        if !frame.base_columns.is_empty() {
            let base_at = frame.columns[..insert_at]
                .iter()
                .rev()
                .find_map(|shown| {
                    frame
                        .base_columns
                        .iter()
                        .position(|base| base.id == shown.id)
                })
                .map_or(0, |index| index + 1);
            frame.base_columns.insert(base_at, column.clone());
        }
        let column_id = column.id.clone();
        frame.columns.insert(insert_at, column.clone());
        for row in &mut frame.rows {
            row.cells.insert(column_id.clone(), Cell::default());
        }
        // A scored frame's carried columns are placed after a named
        // neighbour, so "before the first" is told to it as the append it
        // has always done for a column with no neighbour.
        let after = insert_at
            .checked_sub(1)
            .and_then(|index| frame.columns.get(index))
            .map(|neighbour| neighbour.id.clone());
        self.carry_column_into_scored_frames(frame_id, &column, after.as_deref());
        Ok(())
    }

    /// Takes several of a frame's own rows out at once.
    ///
    /// Every id is checked before anything moves, so a replica that lost one
    /// of the rows to a peer refuses the whole edit rather than removing the
    /// rest — half a selection deleted is not the edit anybody made.
    pub(crate) fn apply_delete_rows(
        &mut self,
        frame_id: Id,
        row_ids: Vec<Id>,
    ) -> Result<(), CoreError> {
        let frame = self.frame(&frame_id)?;
        if !frame.preserves_own_row_identity() {
            return Err(CoreError::DerivedFrameReadOnly);
        }
        let going: HashSet<&str> = row_ids.iter().map(String::as_str).collect();
        if going
            .iter()
            .any(|row_id| !frame.rows.iter().any(|row| row.id == *row_id))
        {
            return Err(CoreError::RowNotFound);
        }
        let going: HashSet<Id> = going.into_iter().map(str::to_string).collect();
        let frame = self.frame_mut(&frame_id)?;
        frame.rows.retain(|row| !going.contains(&row.id));
        // The same tidying `apply_delete_row` does: a style anchored to a row
        // that went away has nothing left to paint.
        frame.display.forget_rows(&going);
        Ok(())
    }
}
