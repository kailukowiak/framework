//! Finding a value inside a frame nobody has in memory.
//!
//! The canvas already holds every cell of a small frame, so the interface can
//! search those itself without asking anything. A paged frame is the case that
//! has no answer at all: its rows exist only as pages read out of a parquet or
//! a connector, and "which row says `Fenwick`" is a question no page in hand
//! can answer. This module asks it of the plan instead, which is the only
//! thing that has seen every row.
//!
//! Deliberately one mode: a case-insensitive substring over every column, cast
//! to text. Not a regex, not a per-column type-aware comparison — Find is the
//! gesture someone reaches for *before* they know what they are looking at,
//! and a mode picker on it is a question asked of somebody who came here
//! precisely because they could not answer it.

use crate::*;
use polars::prelude as pl;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The display position each candidate row occupies, carried past the search
/// filter so a hit can say where to scroll. Distinct from the data-layer
/// `__framework_row` a page read already carries: that one names the *stored*
/// row behind a cell, and a display sort makes the two disagree.
const DISPLAY_INDEX: &str = "__framework_display_row";

/// Longest snippet a hit carries back. A cell holding a paragraph of imported
/// notes should not put that paragraph through the wire fifty times over; the
/// palette shows one line of it either way.
const SNIPPET_LIMIT: usize = 160;

/// One matching cell in a frame read through pages.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RowHit {
    /// Where the row sits in the frame *as displayed* — what the grid scrolls
    /// to. Not the stored ordinal, which a display sort reorders away from.
    ///
    /// Typed for TypeScript rather than left to ts-rs, which renders a `u64`
    /// as `bigint`: serde_json writes this as a plain JSON number, the same
    /// reasoning `DocumentView::revision` carries.
    #[ts(type = "number")]
    pub row_index: u64,
    /// The row's identity, spelled by exactly the code a page read spells it
    /// with, so a hit and the page the grid loads for it agree about which row
    /// this is. For a frame that owns its rows that is the stored row's id;
    /// for one read out of a file it is positional, which is all such a row
    /// has ever had. `None` only when the plan gave back fewer identities than
    /// rows, which nothing currently does.
    pub row_id: Option<Id>,
    pub column_id: Id,
    /// The matching cell's text, as displayed by `.cast(String)`.
    pub snippet: String,
}

impl Document {
    /// Every cell of `frame_id` whose text contains `query`, case-insensitively,
    /// up to `limit` hits.
    ///
    /// The plan is built exactly the way a page read builds it — same wrangle
    /// chain, same display filter and sort — so a hit's position is a position
    /// in the frame somebody is actually looking at. The search runs above the
    /// slice a page would take, because the whole point is the rows that are
    /// not on screen.
    pub(crate) fn search_frame_rows(
        &self,
        frame_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<RowHit>, CoreError> {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let frame = self.frame(frame_id)?;
        if frame.columns.is_empty() {
            return Ok(Vec::new());
        }

        // Cast once, in the plan, and search the cast column: a hit has to be
        // reported against the text somebody would read in the cell, so the
        // comparison and the snippet must come from the same string rather
        // than from a value formatted twice by two different code paths.
        let as_text = |column: &Column| pl::col(column.id.clone()).cast(pl::DataType::String);
        let matches = |column: &Column| {
            as_text(column)
                .str()
                .to_lowercase()
                .str()
                .contains_literal(pl::lit(needle.clone()))
                .fill_null(pl::lit(false))
        };
        let predicate = frame
            .columns
            .iter()
            .map(matches)
            .reduce(|left, right| left.or(right))
            .expect("frame has at least one column");

        let mut projection = vec![
            pl::col(DISPLAY_INDEX),
            pl::col(crate::engine::plan::ROW_INDEX),
        ];
        projection.extend(frame.columns.iter().map(as_text));

        // Each surviving row matched in at least one column, so `limit` rows
        // can only ever be too many hits, never too few — the truncation
        // below is what makes the count exact.
        let data_frame = self
            .indexed_frame_page_plan(frame)?
            .with_row_index(DISPLAY_INDEX, None)
            .select(projection)
            .filter(predicate)
            .slice(0, limit as u32)
            .collect()
            .map_err(|error| CoreError::Import(error.to_string()))?;

        let row_ids = self.page_row_ids(frame, &data_frame, 0);
        let positions = data_frame
            .column(DISPLAY_INDEX)
            .ok()
            .and_then(|column| column.as_materialized_series().u32().ok().cloned());
        let texts: Vec<(Id, Option<pl::StringChunked>)> = frame
            .columns
            .iter()
            .map(|column| {
                let values = data_frame
                    .column(&column.id)
                    .ok()
                    .and_then(|found| found.as_materialized_series().str().ok().cloned());
                (column.id.clone(), values)
            })
            .collect();

        let mut hits = Vec::new();
        for index in 0..data_frame.height() {
            for (column_id, values) in &texts {
                let Some(text) = values.as_ref().and_then(|values| values.get(index)) else {
                    continue;
                };
                if !text.to_lowercase().contains(&needle) {
                    continue;
                }
                hits.push(RowHit {
                    row_index: positions
                        .as_ref()
                        .and_then(|positions| positions.get(index))
                        .map(u64::from)
                        .unwrap_or(index as u64),
                    row_id: row_ids.get(index).cloned(),
                    column_id: column_id.clone(),
                    snippet: snippet(text),
                });
                if hits.len() == limit {
                    return Ok(hits);
                }
            }
        }
        Ok(hits)
    }
}

fn snippet(text: &str) -> String {
    if text.chars().count() <= SNIPPET_LIMIT {
        return text.to_string();
    }
    text.chars().take(SNIPPET_LIMIT).collect::<String>() + "…"
}
