use crate::Id;
use crate::error::CoreError;
use polars::prelude as pl;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Identifies a distinct sorted presentation of a paged frame: the frame,
/// and a fingerprint of everything that decides the rows it produces —
/// upstream lineage, its own snapshot, and its own display filter and sort
/// (see [`Document::frame_fingerprint`]). Any change to those produces a
/// different key, which is how the cache "invalidates": stale keys are
/// simply never looked up again and are pruned lazily.
///
/// Display state needs no signature of its own here, and that is the point
/// of the unification — filter and sort are fields of the frame, so the one
/// fingerprint already covers them instead of two hand-maintained strings
/// that had to be kept in step with the model by hand.
///
/// The fingerprint is lineage-scoped rather than a document revision on
/// purpose: an edit elsewhere in the document — renaming another frame,
/// moving a card, undoing either — leaves this key unchanged, so the cached
/// frame survives instead of being recomputed for no reason.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SortedPageCacheKey {
    pub(crate) frame_id: Id,
    pub(crate) fingerprint: u64,
}

/// A materialized, already filtered-and-sorted frame for one
/// [`SortedPageCacheKey`], computed once and reused for every page fetch
/// against that ordering. `total_rows` is the row count after filtering
/// (before pagination), matching what `FramePage::total_rows` reports.
pub(crate) struct SortedPageCacheEntry {
    pub(crate) frame: pl::DataFrame,
    pub(crate) total_rows: usize,
}

/// Derived-state cache for paged, sorted frame reads. Never serialized and
/// never part of the document model — it exists purely to avoid re-sorting
/// the whole frame on every 1,000-row page fetch (see `get_frame_page`).
/// Bounded to a small number of entries; entries whose revision no longer
/// matches the live document are pruned whenever the cache is written to, so
/// a sort/filter/undo/redo change can't leave stale multi-megabyte frames
/// around indefinitely.
#[derive(Default)]
pub(crate) struct SortedPageCache {
    pub(crate) entries: Mutex<HashMap<SortedPageCacheKey, Arc<SortedPageCacheEntry>>>,
    /// Incremented every time a full sort+filter is actually computed
    /// (i.e. on a cache miss). Exposed for tests so cache reuse can be
    /// asserted without timing.
    pub(crate) computations: AtomicUsize,
}

/// Cache entries are derived, disposable state: cloning a `Store` (e.g. the
/// pre-merge snapshot in `merge_into`) should not carry over sorted frames
/// from another instance, so a clone just starts with an empty cache.
impl Clone for SortedPageCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl std::fmt::Debug for SortedPageCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SortedPageCache")
            .field("entries", &self.entries.lock().map(|guard| guard.len()))
            .finish()
    }
}

const SORTED_PAGE_CACHE_MAX_ENTRIES: usize = 8;

impl SortedPageCache {
    /// Returns the cached entry for `key` if present, otherwise computes it
    /// with `compute`, stores it, and returns it. Also prunes entries for
    /// the same frame that a newer fingerprint has superseded, so the cache
    /// can't grow unbounded across a long editing session.
    pub(crate) fn get_or_compute(
        &self,
        key: SortedPageCacheKey,
        compute: impl FnOnce() -> Result<SortedPageCacheEntry, CoreError>,
    ) -> Result<Arc<SortedPageCacheEntry>, CoreError> {
        let mut guard = self.entries.lock().expect("sorted page cache poisoned");
        if let Some(entry) = guard.get(&key) {
            return Ok(entry.clone());
        }
        // Only this frame's own superseded entries are unreachable: their
        // fingerprint can never be produced again. Every other frame's
        // entries stay, which is the point of fingerprinting by lineage --
        // an edit over here must not throw away a sort computed over there.
        guard.retain(|cached_key, _| {
            cached_key.frame_id != key.frame_id || cached_key.fingerprint == key.fingerprint
        });
        drop(guard);

        self.computations.fetch_add(1, Ordering::Relaxed);
        let entry = Arc::new(compute()?);

        let mut guard = self.entries.lock().expect("sorted page cache poisoned");
        if guard.len() >= SORTED_PAGE_CACHE_MAX_ENTRIES
            && let Some(evict_key) = guard.keys().next().cloned()
        {
            guard.remove(&evict_key);
        }
        guard.insert(key, entry.clone());
        Ok(entry)
    }

    pub(crate) fn computations(&self) -> usize {
        self.computations.load(Ordering::Relaxed)
    }
}
