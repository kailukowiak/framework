//! Tidying the canvas: where every window goes when the user asks for the
//! mess to be cleaned up.
//!
//! The arrangement is a pure function of the document, which is what makes
//! it safe to prepare into a fully determined operation and replay on
//! another replica. It reads geometry (each window keeps its own size, and a
//! collapsed one only claims the height it draws) and lineage (what each
//! window shows, and what that was derived from), and nothing else — no
//! clock, no randomness, no current scroll position.
//!
//! Lineage decides the columns. A window sits one column right of the
//! furthest-left thing it reads, so sources are on the left and the results
//! computed from them march rightward. That is the same left-to-right story
//! the lineage cords already draw between cards, so tidying makes the cords
//! shorter and mostly parallel instead of crossing the canvas.

use crate::Id;
use crate::model::document::{CanvasView, DataObject, Document};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Where one window lands. The size is left alone deliberately: a window the
/// user made big is big because they wanted to see it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ViewPlacement {
    pub view_id: Id,
    pub x: f64,
    pub y: f64,
}

/// Where the first window's top-left corner goes.
const ORIGIN_X: f64 = 76.0;
const ORIGIN_Y: f64 = 96.0;
/// The gap between columns, and between stacked windows in one column.
const COLUMN_GAP: f64 = 64.0;
const ROW_GAP: f64 = 28.0;
/// What a collapsed card actually draws, matching the CSS.
const COLLAPSED_HEIGHT: f64 = 29.0;
/// A guard against a lineage that somehow loops. The core rejects circular
/// derivations, so reaching this means the document is already malformed and
/// the honest answer is "column zero" rather than a hang.
const MAX_DEPTH: usize = 64;

impl Document {
    /// Every window's tidied position, in the order they should be applied.
    ///
    /// Deterministic: the same document always yields the same placements, so
    /// tidying twice in a row is a no-op rather than a slow drift.
    pub fn tidy_layout(&self) -> Vec<ViewPlacement> {
        let depths = self.object_depths();
        let mut ordered: Vec<(Vec<usize>, &CanvasView)> = self
            .views
            .iter()
            .map(|view| (self.lineage_sort_key(view, &depths), view))
            .collect();
        // The sort key starts with the column, so this both groups the
        // windows into columns and orders each column so that children fall
        // under the parent they were derived from.
        ordered.sort_by(|(left, _), (right, _)| left.cmp(right));

        let mut placements = Vec::with_capacity(ordered.len());
        let mut column_x = ORIGIN_X;
        let mut cursor_y = ORIGIN_Y;
        let mut column_width: f64 = 0.0;
        let mut current_column = ordered.first().and_then(|(key, _)| key.first().copied());

        for (key, view) in ordered {
            let column = key.first().copied();
            if column != current_column {
                column_x += column_width + COLUMN_GAP;
                cursor_y = ORIGIN_Y;
                column_width = 0.0;
                current_column = column;
            }
            placements.push(ViewPlacement {
                view_id: view.id.clone(),
                x: column_x,
                y: cursor_y,
            });
            cursor_y += self.drawn_height(view) + ROW_GAP;
            column_width = column_width.max(view.width);
        }
        placements
    }

    /// The height a window actually occupies on the canvas. A collapsed card
    /// is a title bar, so stacking it against its stored height would
    /// leave exactly the hole collapsing it was meant to close.
    fn drawn_height(&self, view: &CanvasView) -> f64 {
        if view.collapsed {
            COLLAPSED_HEIGHT
        } else {
            view.height
        }
    }

    /// How far down the lineage each object sits: 0 for anything read from
    /// nothing else, one more than its furthest input otherwise.
    fn object_depths(&self) -> HashMap<&str, usize> {
        let mut depths = HashMap::with_capacity(self.objects.len());
        for object in &self.objects {
            self.resolve_depth(object.id(), &mut depths, 0);
        }
        depths
    }

    fn resolve_depth<'a>(
        &'a self,
        object_id: &'a str,
        depths: &mut HashMap<&'a str, usize>,
        guard: usize,
    ) -> usize {
        if let Some(depth) = depths.get(object_id) {
            return *depth;
        }
        if guard >= MAX_DEPTH {
            return 0;
        }
        let depth = match self.objects.iter().find(|object| object.id() == object_id) {
            Some(DataObject::Frame(frame)) => {
                // A join or a union reads a second frame, so the result
                // belongs to the right of both of its inputs, not just the
                // primary one. The lookup ids are looked back up in
                // `objects` because `lookup_frame_ids` hands back owned ids
                // that would not outlive the recursive call.
                let deepest_lookup = frame
                    .lookup_frame_ids()
                    .into_iter()
                    .filter_map(|lookup_id| {
                        self.objects
                            .iter()
                            .find(|object| object.id() == lookup_id)
                            .map(DataObject::id)
                    })
                    .map(|lookup_id| self.resolve_depth(lookup_id, depths, guard + 1))
                    .max();
                match (&frame.derivation, deepest_lookup) {
                    (Some(derivation), lookup) => {
                        let source =
                            self.resolve_depth(&derivation.source_frame_id, depths, guard + 1);
                        source.max(lookup.unwrap_or(0)) + 1
                    }
                    // A source frame whose own chain stacks another frame
                    // still sits to the right of what it reads.
                    (None, Some(lookup)) => lookup + 1,
                    (None, None) => 0,
                }
            }
            Some(DataObject::Plot(plot)) => {
                self.resolve_depth(&plot.source_frame_id, depths, guard + 1) + 1
            }
            _ => 0,
        };
        depths.insert(object_id, depth);
        depth
    }

    /// A window's place in the tidied order: its column, then the creation
    /// order of each ancestor from the root down, then its own.
    ///
    /// Comparing those ancestor paths lexicographically is what keeps a
    /// derived frame directly under the frame it came from, and keeps two
    /// branches of the same source together rather than interleaved.
    fn lineage_sort_key(&self, view: &CanvasView, depths: &HashMap<&str, usize>) -> Vec<usize> {
        // A card with tabs shows several objects; it belongs beside the
        // earliest input any of them reads, so the whole card stays left of
        // everything computed from it.
        let anchor = view
            .tabs()
            .iter()
            .min_by_key(|tab| depths.get(tab.as_str()).copied().unwrap_or(0))
            .cloned()
            .unwrap_or_else(|| view.object_id.clone());
        let mut key = vec![depths.get(anchor.as_str()).copied().unwrap_or(0)];
        key.extend(self.ancestry(&anchor));
        key
    }

    /// Each ancestor's index in `objects`, root first, ending with the object
    /// itself. Creation order is the only stable ordering a document carries
    /// that also tends to match the order the user built things in.
    fn ancestry(&self, object_id: &str) -> Vec<usize> {
        let mut path = Vec::new();
        let mut current = object_id.to_string();
        for _ in 0..MAX_DEPTH {
            let Some(index) = self
                .objects
                .iter()
                .position(|object| object.id() == current)
            else {
                break;
            };
            path.push(index);
            let parent = match &self.objects[index] {
                DataObject::Frame(frame) => frame
                    .derivation
                    .as_ref()
                    .map(|derivation| derivation.source_frame_id.clone()),
                DataObject::Plot(plot) => Some(plot.source_frame_id.clone()),
                _ => None,
            };
            match parent {
                Some(parent) => current = parent,
                None => break,
            }
        }
        path.reverse();
        path
    }
}
