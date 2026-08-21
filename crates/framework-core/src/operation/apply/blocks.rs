//! Applying already-resolved `ReplicatedOperation`s in this family.
//!
//! Formula blocks: the ordered scratchpad and the lines in it.

use crate::*;

/// Lines whose calculation no longer means what it meant before this edit.
///
/// Source text is deliberately not the comparison. Renaming a frame or a
/// referenced line rewrites that text while the parsed expression keeps the
/// same stable ids, and a recorded answer is still the answer to that same
/// calculation. Changing the expression, losing it to a parse error, or
/// removing the line is different: keeping the frozen value would put an old
/// answer beside a new question.
pub(crate) fn changed_block_line_ids(current: &[BlockLine], next: &[BlockLine]) -> Vec<Id> {
    current
        .iter()
        .filter(|line| {
            next.iter()
                .find(|candidate| candidate.id == line.id)
                .is_none_or(|candidate| candidate.formula != line.formula)
        })
        .map(|line| line.id.clone())
        .collect()
}

impl Document {
    pub(crate) fn apply_set_block_lines(
        &mut self,
        block_id: Id,
        lines: Vec<BlockLine>,
    ) -> Result<(), CoreError> {
        // Line ids are document-unique, the way object ids are: a formula
        // anywhere may hold one, so two lines answering to the same id would
        // make a reference mean two things.
        for line in &lines {
            if let Some((owner, _)) = self.block_line(&line.id)
                && owner.id != block_id
            {
                return Err(CoreError::InvalidOperation(
                    "a line ID already belongs to another block".into(),
                ));
            }
        }
        let changed = changed_block_line_ids(&self.block(&block_id)?.lines, &lines);
        for line_id in changed {
            self.frozen_values.remove(&line_id);
        }
        self.block_mut(&block_id)?.lines = lines;
        Ok(())
    }
}
