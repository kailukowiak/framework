//! Formatting a rectangle of cells.
//!
//! A frame's direct formats were one per target — the frame, a column, a
//! row, a cell — and each replaced whatever that target had. That is the
//! right shape for a single cell and the wrong one for a selection: bold
//! over two thousand cells would be two thousand records and two thousand
//! undos. A range is one record over a set of rows and a set of columns.
//!
//! What makes it subtle is that a range overlaps things. A cell inside it
//! may already have a format of its own, and cell formats outrank ranges, so
//! a range that simply sat beneath them would leave the bold on a cell
//! somebody had bolded before and then un-bolded as part of a selection.
//! So a range is a patch rather than a replacement: it means only the
//! properties it sets, and setting it clears those properties from every
//! cell format it covers. Among ranges, later ones win, which is why they
//! are kept in the order they were made. The effect is the rule a person
//! expects from a spreadsheet — the most recent format of a cell wins,
//! whichever gesture made it — without a record per cell.
//!
//! **Clear is the exception that has to cut.** A patch can leave a partly
//! covered earlier range alone, because the new range sits after it and
//! wins the overlap property by property. Clear pushes nothing — an empty
//! range would win nothing — so an earlier range only partly inside would
//! keep painting the cleared cells, and "Clear formatting" on two cells of
//! a bolded block would visibly do nothing. So a clear splits such a range
//! into the parts it missed (see `CoveredCells::uncovered_parts`), in the range's own
//! place in the list, so its order against later ranges is unchanged.
//!
//! A rectangle clear removes cell and range formats only. A row, column or
//! frame format is a broader declaration than the selection — "this column
//! is currency-red" — and a clear over a few of its cells is not a decision
//! about the whole of it; the cleared cells fall back to it. Excel has no
//! such layer to keep, which is why it can simply wipe the cells. The
//! inspector's "Clear formatting" is offered by the same rule
//! (`rectangleHasFormatting` in `src/lib/frameStyleRanges.ts`), so it is
//! never offered for formatting it would not touch.
//!
//! **The edit is recorded as splices, not as the list it leaves.** Undo
//! has to reverse exactly what this edit did to the list and nothing else:
//! between the edit and its undo, a collaborator may have formatted a
//! column of the same frame, and putting the whole list back would erase
//! that. A [`FrameStyleSplice`] names one run of entries and what replaced
//! it, so the inverse is the same splices turned round, and anything added
//! or changed since is left where it is.

use crate::Id;
use crate::model::frame::{FrameCellStyle, FrameDisplay, FrameStyle, FrameStyleTarget};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// The cells of a range, for asking what it covers.
pub(crate) struct CoveredCells<'a> {
    rows: HashSet<&'a str>,
    columns: HashSet<&'a str>,
}

/// How much of an earlier range a new one reaches.
enum Overlap {
    None,
    Partial,
    Whole,
}

impl<'a> CoveredCells<'a> {
    pub(crate) fn new(row_ids: &'a [Id], column_ids: &'a [Id]) -> Self {
        Self {
            rows: row_ids.iter().map(String::as_str).collect(),
            columns: column_ids.iter().map(String::as_str).collect(),
        }
    }

    fn contains(&self, row_id: &str, column_id: &str) -> bool {
        self.rows.contains(row_id) && self.columns.contains(column_id)
    }

    fn overlap(&self, row_ids: &[Id], column_ids: &[Id]) -> Overlap {
        let rows_in = row_ids
            .iter()
            .filter(|row_id| self.rows.contains(row_id.as_str()))
            .count();
        let columns_in = column_ids
            .iter()
            .filter(|column_id| self.columns.contains(column_id.as_str()))
            .count();
        if rows_in == 0 || columns_in == 0 {
            Overlap::None
        } else if rows_in == row_ids.len() && columns_in == column_ids.len() {
            Overlap::Whole
        } else {
            Overlap::Partial
        }
    }

    /// What is left of a partly covered range once these cells are taken
    /// out of it, as at most two rectangles: its rows that were missed, at
    /// its full width, then its rows that were hit, at the columns that
    /// were missed. Between them they are every cell of the range outside
    /// this one and none inside, and ids keep the range's own order.
    fn uncovered_parts(&self, row_ids: &[Id], column_ids: &[Id]) -> Vec<FrameStyleTarget> {
        let (rows_hit, rows_missed): (Vec<Id>, Vec<Id>) = row_ids
            .iter()
            .cloned()
            .partition(|row_id| self.rows.contains(row_id.as_str()));
        let columns_missed: Vec<Id> = column_ids
            .iter()
            .filter(|column_id| !self.columns.contains(column_id.as_str()))
            .cloned()
            .collect();
        let mut parts = Vec::new();
        if !rows_missed.is_empty() {
            parts.push(FrameStyleTarget::Range {
                row_ids: rows_missed,
                column_ids: column_ids.to_vec(),
            });
        }
        if !rows_hit.is_empty() && !columns_missed.is_empty() {
            parts.push(FrameStyleTarget::Range {
                row_ids: rows_hit,
                column_ids: columns_missed,
            });
        }
        parts
    }
}

impl FrameCellStyle {
    /// Forgets every property `newer` sets, keeping the rest.
    fn yield_to(&mut self, newer: &FrameCellStyle) {
        fn clear<T>(property: &mut Option<T>, newer: &Option<T>) {
            if newer.is_some() {
                *property = None;
            }
        }
        clear(&mut self.bold, &newer.bold);
        clear(&mut self.italic, &newer.italic);
        clear(&mut self.underline, &newer.underline);
        clear(&mut self.text_color, &newer.text_color);
        clear(&mut self.fill_color, &newer.fill_color);
        clear(&mut self.alignment, &newer.alignment);
        clear(&mut self.line_style, &newer.line_style);
    }
}

/// One run of a frame's direct formats replaced by another.
///
/// `index` is where the run starts in the list as it stands before the
/// splices that carry it are applied; a run to insert removes nothing.
/// Entries are named by value as well as by place because the place can
/// move: a peer's edit may have removed something earlier in the list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FrameStyleSplice {
    pub index: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove: Vec<FrameStyle>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub insert: Vec<FrameStyle>,
}

/// The splices that put the list back as it was before `splices` — which
/// must be resolved against that list — were applied.
///
/// Each splice's place shifts by how much the ones before it grew or
/// shrank the list; what it removed it inserts, and the other way round.
pub(crate) fn invert_style_splices(splices: &[FrameStyleSplice]) -> Vec<FrameStyleSplice> {
    let mut shift = 0isize;
    splices
        .iter()
        .map(|splice| {
            let index = (splice.index as isize + shift) as usize;
            shift += splice.insert.len() as isize - splice.remove.len() as isize;
            FrameStyleSplice {
                index,
                remove: splice.insert.clone(),
                insert: splice.remove.clone(),
            }
        })
        .collect()
}

impl FrameDisplay {
    /// The splices that adding a range format on top makes: the cell
    /// formats inside it, and the earlier ranges wholly inside it, give up
    /// the properties it sets, and any left with nothing are removed; the
    /// range itself goes on the end, unless it sets nothing.
    ///
    /// An earlier range only partly inside keeps its properties under a
    /// patch — it still paints the cells outside — and loses to the new one
    /// by order where they overlap. An empty style is "clear formats":
    /// every cell format and wholly covered range beneath goes, and a range
    /// partly beneath is cut down to the cells outside (see the module
    /// note), so the selection falls back to its rows', its columns' and
    /// the frame's formats.
    pub(crate) fn range_style_splices(
        &self,
        target: &FrameStyleTarget,
        covered: &CoveredCells,
        style: &FrameCellStyle,
    ) -> Vec<FrameStyleSplice> {
        let clear_everything = style.is_empty();
        let mut splices = Vec::new();
        for (index, entry) in self.styles.iter().enumerate() {
            let replacement = match &entry.target {
                FrameStyleTarget::Cell { row_id, column_id }
                    if covered.contains(row_id, column_id) =>
                {
                    yielded(entry, style)
                }
                FrameStyleTarget::Range {
                    row_ids,
                    column_ids,
                } => match covered.overlap(row_ids, column_ids) {
                    Overlap::Whole => yielded(entry, style),
                    Overlap::Partial if clear_everything => covered
                        .uncovered_parts(row_ids, column_ids)
                        .into_iter()
                        .map(|target| FrameStyle {
                            target,
                            style: entry.style.clone(),
                        })
                        .collect(),
                    Overlap::Partial | Overlap::None => continue,
                },
                _ => continue,
            };
            if replacement.as_slice() != std::slice::from_ref(entry) {
                splices.push(FrameStyleSplice {
                    index,
                    remove: vec![entry.clone()],
                    insert: replacement,
                });
            }
        }
        if !clear_everything {
            splices.push(FrameStyleSplice {
                index: self.styles.len(),
                remove: Vec::new(),
                insert: vec![FrameStyle {
                    target: target.clone(),
                    style: style.clone(),
                }],
            });
        }
        splices
    }

    /// Where each splice lands in the list as it stands now, with the ones
    /// that no longer can dropped.
    ///
    /// A run is looked for at its recorded place first and anywhere else
    /// second, since a peer's removal may have shifted it. A run that is
    /// nowhere — a peer has changed or removed an entry it names since —
    /// is dropped, and so is an insertion of a single-target format whose
    /// target a peer has formatted since: in both cases restoring ours
    /// would overwrite a later edit that was not this one's to take back.
    /// Splices come back in list order, ties in the order given, which is
    /// what [`invert_style_splices`] and [`Self::apply_style_splices`] need.
    pub(crate) fn resolve_style_splices(
        &self,
        splices: &[FrameStyleSplice],
    ) -> Vec<FrameStyleSplice> {
        let styles = &self.styles;
        let mut claimed = vec![false; styles.len()];
        let mut resolved = Vec::new();
        for splice in splices {
            let width = splice.remove.len();
            let fits = |start: usize| {
                start + width <= styles.len()
                    && styles[start..start + width] == splice.remove[..]
                    && !claimed[start..start + width].iter().any(|taken| *taken)
            };
            let index = if width == 0 {
                Some(splice.index.min(styles.len()))
            } else if fits(splice.index) {
                Some(splice.index)
            } else {
                (0..styles.len()).find(|start| fits(*start))
            };
            let Some(index) = index else { continue };
            let still_ours = splice.insert.iter().all(|inserted| {
                matches!(inserted.target, FrameStyleTarget::Range { .. })
                    || splice
                        .remove
                        .iter()
                        .any(|removed| removed.target == inserted.target)
                    || !styles
                        .iter()
                        .any(|existing| existing.target == inserted.target)
            });
            if !still_ours {
                continue;
            }
            claimed[index..index + width].fill(true);
            resolved.push(FrameStyleSplice {
                index,
                ..splice.clone()
            });
        }
        resolved.sort_by_key(|splice| splice.index);
        // Runs found by search can interleave with insertions placed by
        // index; one landing inside another's run has no single place to
        // go, and applying both would cut the wrong entries.
        let mut end = 0;
        resolved.retain(|splice| {
            let fits = splice.index >= end;
            if fits {
                end = splice.index + splice.remove.len();
            }
            fits
        });
        resolved
    }

    /// Applies splices `resolve_style_splices` returned, last first, so
    /// each one's place is still the place it was resolved at.
    pub(crate) fn apply_style_splices(&mut self, resolved: Vec<FrameStyleSplice>) {
        for splice in resolved.into_iter().rev() {
            let end = splice.index + splice.remove.len();
            self.styles.splice(splice.index..end, splice.insert);
        }
    }
}

/// An entry beneath a new range, after it gives up what the range sets:
/// nothing at all for a clear, or for a format left with no property.
fn yielded(entry: &FrameStyle, style: &FrameCellStyle) -> Vec<FrameStyle> {
    if style.is_empty() {
        return Vec::new();
    }
    let mut kept = entry.clone();
    kept.style.yield_to(style);
    if kept.style.is_empty() {
        Vec::new()
    } else {
        vec![kept]
    }
}
