//! Several operations as one edit.
//!
//! The rest of the operation machinery already treats an edit as a list —
//! a history entry holds `Vec`s, and an inverse is often several operations —
//! but the boundary into it took one operation at a time, so a gesture that
//! needed two (paste the block, then grow the card to show it) became two
//! events, two revisions, and two undos. A batch is that boundary admitting a
//! list. It is one event, so a peer receives all of it or none; one revision,
//! because nothing could observe the state between its parts; and one undo
//! entry, because it was one thing somebody did.
//!
//! Preparation and inversion both have to see the document as each part
//! finds it, not as the batch found it: the second part of a batch may name a
//! row the first one made, and the inverse of the second has to read what
//! the first one left. So both walk the parts in order against a document
//! that is moved along as they go.

use crate::*;

impl Document {
    pub(crate) fn prepare_batch(
        &self,
        operations: Vec<Operation>,
    ) -> Result<ReplicatedOperation, CoreError> {
        if operations.is_empty() {
            return Err(CoreError::InvalidOperation(
                "A batch needs at least one operation".into(),
            ));
        }
        if operations
            .iter()
            .any(|operation| matches!(operation, Operation::Batch { .. }))
        {
            return Err(CoreError::InvalidOperation(
                "A batch cannot contain another batch".into(),
            ));
        }
        let count = operations.len();
        // Only a part that has another after it needs the document moved on,
        // so a batch of one costs what the operation alone does.
        let mut scratch = (count > 1).then(|| self.clone());
        let mut prepared = Vec::with_capacity(count);
        for (index, operation) in operations.into_iter().enumerate() {
            let document = scratch.as_ref().unwrap_or(self);
            let replicated = document.prepare_operation(operation)?;
            if index + 1 < count
                && let Some(scratch) = scratch.as_mut()
            {
                scratch.apply_replicated(replicated.clone())?;
            }
            prepared.push(replicated);
        }
        Ok(ReplicatedOperation::Batch {
            operations: prepared,
        })
    }

    /// Each part checks the document's invariants as it lands, so a batch
    /// can never pass through a state no single edit could reach; the
    /// caller restores the whole document if one fails.
    pub(crate) fn apply_batch(
        &mut self,
        operations: Vec<ReplicatedOperation>,
    ) -> Result<(), CoreError> {
        for operation in operations {
            self.apply_replicated(operation)?;
        }
        Ok(())
    }

    /// Applies `operation` and returns what undoes it, reading each part's
    /// inverse at the one moment it can be read: just before that part
    /// applies.
    ///
    /// Leaves the document part-way on failure; the caller holds the copy
    /// it restores from, as it does for a single operation.
    pub(crate) fn apply_and_invert(
        &mut self,
        operation: ReplicatedOperation,
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        let ReplicatedOperation::Batch { operations } = operation else {
            let inverse = self.invert(&operation)?;
            self.apply_replicated(operation)?;
            return Ok(inverse);
        };
        let mut inverses = Vec::with_capacity(operations.len());
        for operation in operations {
            inverses.push(self.apply_and_invert(operation)?);
        }
        // Undone last part first: each inverse was read against the state
        // the parts before it left, so that is the state it expects.
        Ok(inverses.into_iter().rev().flatten().collect())
    }

    /// The inverse of a batch without applying it here, for a caller that
    /// asks `invert` directly. Works on a copy, which is the price of
    /// reading each part's inverse against the state the earlier parts
    /// leave; history avoids it through [`Document::apply_and_invert`].
    pub(crate) fn invert_batch(
        &self,
        operations: &[ReplicatedOperation],
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        self.clone().apply_and_invert(ReplicatedOperation::Batch {
            operations: operations.to_vec(),
        })
    }
}
