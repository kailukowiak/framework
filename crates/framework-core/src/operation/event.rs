use crate::Id;
use crate::operation::kinds::ReplicatedOperation;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const OPERATION_EVENT_VERSION: u32 = 1;

pub type VersionVector = BTreeMap<Id, u64>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct EventId {
    pub writer_id: Id,
    pub sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OperationEvent {
    pub format_version: u32,
    pub document_id: Id,
    pub event_id: EventId,
    pub dependencies: VersionVector,
    pub operation: ReplicatedOperation,
}

impl OperationEvent {
    pub fn new(
        document_id: Id,
        writer_id: Id,
        sequence: u64,
        dependencies: VersionVector,
        operation: ReplicatedOperation,
    ) -> Self {
        Self {
            format_version: OPERATION_EVENT_VERSION,
            document_id,
            event_id: EventId {
                writer_id,
                sequence,
            },
            dependencies,
            operation,
        }
    }
}
