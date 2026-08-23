//! Full-data diagnostics for choosing a join relationship.
//!
//! A join is authored from the canvas, where an imported or live frame may
//! have only one page of rows in memory. Match and uniqueness claims cannot
//! be inferred from that page: they are properties of the plan. This module
//! keeps the scan on the engine side and out of `DocumentView`, so opening a
//! small lookup prompt is what pays for it rather than every canvas edit.

use crate::*;
use polars::prelude as pl;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct JoinDiagnostics {
    pub primary_rows: usize,
    pub matched_rows: usize,
    pub unmatched_rows: usize,
    pub lookup_rows: usize,
    /// Lookup rows missing at least one piece of the proposed key.
    pub lookup_null_key_rows: usize,
    /// Distinct non-null key values that occur more than once.
    pub lookup_duplicate_key_values: usize,
}

impl Store {
    pub fn get_join_diagnostics(
        &self,
        primary_frame_id: &str,
        lookup_frame_id: &str,
        primary_key_column_ids: &[Id],
        lookup_key_column_ids: &[Id],
    ) -> Result<JoinDiagnostics, CoreError> {
        self.document.join_diagnostics(
            primary_frame_id,
            lookup_frame_id,
            primary_key_column_ids,
            lookup_key_column_ids,
        )
    }
}

impl Document {
    fn join_diagnostics(
        &self,
        primary_frame_id: &str,
        lookup_frame_id: &str,
        primary_key_column_ids: &[Id],
        lookup_key_column_ids: &[Id],
    ) -> Result<JoinDiagnostics, CoreError> {
        if primary_frame_id == lookup_frame_id {
            return Err(CoreError::InvalidOperation(
                "Choose two different frames to inspect a join".into(),
            ));
        }
        if primary_key_column_ids.is_empty()
            || primary_key_column_ids.len() != lookup_key_column_ids.len()
        {
            return Err(CoreError::InvalidOperation(
                "Choose the same number of key columns on both sides".into(),
            ));
        }
        let primary_frame = self.frame(primary_frame_id)?;
        let lookup_frame = self.frame(lookup_frame_id)?;
        for (primary_id, lookup_id) in primary_key_column_ids.iter().zip(lookup_key_column_ids) {
            let primary = primary_frame
                .columns
                .iter()
                .find(|column| column.id == *primary_id)
                .ok_or(CoreError::ColumnNotFound)?;
            let lookup = lookup_frame
                .columns
                .iter()
                .find(|column| column.id == *lookup_id)
                .ok_or(CoreError::ColumnNotFound)?;
            if !join_types_compatible(primary.data_type, lookup.data_type) {
                return Err(CoreError::InvalidOperation(
                    "Join columns must have compatible types".into(),
                ));
            }
        }

        let primary = self
            .materialize_frame_lazy(primary_frame_id, Layer::Data, &mut HashSet::new())
            .map_err(CoreError::Import)?;
        let lookup = self
            .materialize_frame_lazy(lookup_frame_id, Layer::Data, &mut HashSet::new())
            .map_err(CoreError::Import)?;
        let (primary, lookup) = crate::engine::plan::match_key_types(
            primary,
            lookup,
            primary_key_column_ids,
            lookup_key_column_ids,
        )
        .map_err(CoreError::Import)?;

        let primary_rows = lazy_row_count(primary.clone())?;
        let lookup_rows = lazy_row_count(lookup.clone())?;
        let lookup_has_null = key_has_null(lookup_key_column_ids);
        let lookup_null_key_rows = lazy_row_count(lookup.clone().filter(lookup_has_null.clone()))?;
        let usable_lookup = lookup.filter(lookup_has_null.not());

        // A semi join counts primary rows, not result pairs, so duplicate
        // lookup keys cannot inflate the match preview while they are being
        // reported separately below.
        let matched = primary.join(
            usable_lookup.clone(),
            primary_key_column_ids
                .iter()
                .map(pl::col)
                .collect::<Vec<_>>(),
            lookup_key_column_ids
                .iter()
                .map(pl::col)
                .collect::<Vec<_>>(),
            pl::JoinArgs::new(pl::JoinType::Semi),
        );
        let matched_rows = lazy_row_count(matched)?;

        const KEY_COUNT: &str = "__framework_join_key_count";
        let duplicate_keys = usable_lookup
            .group_by(
                lookup_key_column_ids
                    .iter()
                    .map(pl::col)
                    .collect::<Vec<_>>(),
            )
            .agg([pl::len().alias(KEY_COUNT)])
            .filter(pl::col(KEY_COUNT).gt(pl::lit(1u32)));
        let lookup_duplicate_key_values = lazy_row_count(duplicate_keys)?;

        Ok(JoinDiagnostics {
            primary_rows,
            matched_rows,
            unmatched_rows: primary_rows.saturating_sub(matched_rows),
            lookup_rows,
            lookup_null_key_rows,
            lookup_duplicate_key_values,
        })
    }
}

fn key_has_null(column_ids: &[Id]) -> pl::Expr {
    column_ids
        .iter()
        .map(|column_id| pl::col(column_id).is_null())
        .reduce(pl::Expr::or)
        .expect("join diagnostics checked for at least one key")
}
