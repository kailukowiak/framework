//! Header mapping is resolved at edit time. Read the data plan, never the
//! displayed page: a large or live mapping must not silently rename only the
//! headers whose entries happen to be visible. Only matching keys are collected.
use crate::*;
use polars::prelude as pl;
use std::collections::{HashMap, HashSet};

impl Document {
    pub(super) fn prepare_mapping_rename(
        &self,
        frame_id: Id,
        mapping_frame_id: Id,
        key_column_id: Id,
        value_column_id: Id,
    ) -> Result<ReplicatedOperation, CoreError> {
        let target = self.frame(&frame_id)?;
        let mapping = self.frame(&mapping_frame_id)?;
        if key_column_id == value_column_id
            || !mapping
                .unique_keys
                .iter()
                .any(|key| key.column_ids == [key_column_id.clone()])
        {
            return Err(CoreError::InvalidOperation(
                "Choose a mapping frame with a unique key column".into(),
            ));
        }
        for id in [&key_column_id, &value_column_id] {
            if !mapping.columns.iter().any(|column| &column.id == id) {
                return Err(CoreError::ColumnNotFound);
            }
        }
        let key = pl::col(&key_column_id).cast(pl::DataType::String);
        let predicate = target
            .columns
            .iter()
            .fold(pl::lit(false), |predicate, column| {
                predicate.or(key.clone().eq(pl::lit(column.name.clone())))
            });
        let data = self
            .materialize_frame_lazy(&mapping_frame_id, Layer::Data, &mut HashSet::new())
            .map_err(CoreError::InvalidOperation)?
            .filter(predicate)
            .select([key, pl::col(&value_column_id).cast(pl::DataType::String)])
            .collect()
            .map_err(|error| CoreError::InvalidOperation(error.to_string()))?;
        let keys = data
            .column(&key_column_id)
            .and_then(|column| column.str())
            .map_err(|error| CoreError::InvalidOperation(error.to_string()))?;
        let values = data
            .column(&value_column_id)
            .and_then(|column| column.str())
            .map_err(|error| CoreError::InvalidOperation(error.to_string()))?;
        let mut replacements = HashMap::new();
        for index in 0..data.height() {
            let Some(key) = keys.get(index) else { continue };
            let value = values.get(index);
            let value = value
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    CoreError::InvalidOperation(format!(
                        "Replacement for ‘{key}’ is blank; column names cannot be missing"
                    ))
                })?;
            if replacements.insert(key, value).is_some() {
                return Err(CoreError::InvalidOperation(format!(
                    "Mapping key ‘{key}’ is duplicated"
                )));
            }
        }
        let names = target
            .columns
            .iter()
            .filter_map(|column| {
                replacements
                    .get(column.name.as_str())
                    .map(|name| (column.id.clone(), (*name).to_owned()))
            })
            .collect();
        self.prepare_file_operation(Operation::RenameColumns { frame_id, names })
    }
}
