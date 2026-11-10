//! How a frame's columns are laid out on screen, and keeping a frame's
//! display layer honest as its rows and columns come and go.
//!
//! Widths and wrapping are view state in exactly the sense freezing and
//! sorting are: they say how somebody is looking at a frame, nothing about
//! what is in it. That is why they are accepted on every kind of frame —
//! a derived frame, a live import, a frozen snapshot — none of which take an
//! edit to their data, and why nothing in the engine reads them. Auto-fit is
//! measured where the text is drawn, in the browser; the document only
//! remembers the answer, so a fitted grid reopens fitted without measuring
//! again and a peer sees the same widths.

use crate::Id;
use crate::model::frame::{FrameDisplay, FrameStyleTarget};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use ts_rs::TS;

/// The narrowest a column is stored at, in CSS pixels. Narrower than this
/// a header cannot show even the sort arrow, so a drag past it means "as
/// narrow as it goes" rather than a mistake worth refusing.
pub const MIN_COLUMN_WIDTH: f64 = 24.0;

/// The widest a column is stored at. Wider than any screen; it exists so a
/// width that arrives from a runaway measurement cannot make a grid that
/// scrolls for a kilometre.
pub const MAX_COLUMN_WIDTH: f64 = 2000.0;

/// One column's stored layout. Absent from the map means automatic and
/// unwrapped, and an entry that would say exactly that is removed rather
/// than kept, so a document that was resized and reset is the document it
/// was before.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameColumnLayout {
    /// Width in CSS pixels; `None` lets the table size the column.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub width: Option<f64>,
    /// Whether long text breaks onto further lines inside the cell instead
    /// of being clipped at the column's edge.
    #[serde(default, skip_serializing_if = "is_false")]
    pub wrap: bool,
}

impl FrameColumnLayout {
    pub fn is_automatic(&self) -> bool {
        self.width.is_none() && !self.wrap
    }
}

/// A column's whole layout, as one entry of a layout edit.
///
/// It replaces what the column had rather than patching it, so there is no
/// third state to spell: `width: None` is automatic, not "unchanged". A
/// resize sends the wrap the grid already shows, and a wrap toggle sends the
/// width — the client has both in hand, and a field that meant "leave this
/// alone" would make the inverse depend on which fields were sent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameColumnLayoutUpdate {
    pub column_id: Id,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub width: Option<f64>,
    #[serde(default)]
    pub wrap: bool,
}

impl FrameColumnLayoutUpdate {
    pub(crate) fn layout(&self) -> FrameColumnLayout {
        FrameColumnLayout {
            width: self.width,
            wrap: self.wrap,
        }
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl FrameDisplay {
    /// Drops everything this display layer keeps about columns that are no
    /// longer the frame's: their layout, and any direct format anchored to
    /// them. A range format loses only the gone columns, and goes only once
    /// it has none left.
    ///
    /// Nothing would break if they stayed — the grid looks entries up by
    /// column id and would never find these — but a document that quietly
    /// accumulates formats for columns deleted months ago is one whose
    /// display layer nobody can read, and undo of the deletion restores the
    /// whole frame, entries included, so forgetting costs nothing.
    pub(crate) fn forget_columns(&mut self, gone: impl Fn(&str) -> bool) {
        self.column_layout.retain(|column_id, _| !gone(column_id));
        self.styles.retain_mut(|entry| match &mut entry.target {
            FrameStyleTarget::Column { column_id } | FrameStyleTarget::Cell { column_id, .. } => {
                !gone(column_id)
            }
            FrameStyleTarget::Range { column_ids, .. } => {
                column_ids.retain(|column_id| !gone(column_id));
                !column_ids.is_empty()
            }
            FrameStyleTarget::Frame | FrameStyleTarget::Row { .. } => true,
        });
    }

    /// The same for rows, by id: a format anchored to a row that went away
    /// has nothing left to paint.
    pub(crate) fn forget_rows(&mut self, gone: &HashSet<Id>) {
        self.styles.retain_mut(|entry| match &mut entry.target {
            FrameStyleTarget::Row { row_id } | FrameStyleTarget::Cell { row_id, .. } => {
                !gone.contains(row_id)
            }
            FrameStyleTarget::Range { row_ids, .. } => {
                row_ids.retain(|row_id| !gone.contains(row_id));
                !row_ids.is_empty()
            }
            FrameStyleTarget::Frame | FrameStyleTarget::Column { .. } => true,
        });
    }
}
