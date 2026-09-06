//! Debounced scheduling for the `.fw` snapshot write.
//!
//! Every operation used to call `Store::save` synchronously, which meant one
//! disk write per keystroke-equivalent edit. On a folder watched by Dropbox
//! or Google Drive that is not a performance nit -- it is one sync upload per
//! edit, and a burst of them (dragging a card, typing a formula character by
//! character) is exactly the shape that produces a conflicted copy when the
//! sync client uploads a half-written intermediate state at the same moment
//! a peer's own upload lands. The fix is not "write less data", it is "write
//! less often": coalesce a burst of operations into one write, performed
//! after the burst goes idle.
//!
//! This module holds only the timing decision -- whether a write is owed and
//! whether its debounce window has elapsed -- not the write itself or
//! anything about `DocumentSession`. Keeping it free of those types is what
//! lets the scheduling logic be unit tested without a `Store`, a `Mutex`, or
//! a Tauri runtime: see the tests below, which drive it with a clock that
//! advances by hand instead of sleeping in real time.

use std::time::{Duration, Instant};

/// How long an idle window must last before a scheduled write actually
/// touches disk. Long enough that ordinary interactive editing (typing a
/// formula, dragging a card, clicking through several cells) coalesces into
/// one write; short enough that a person who stops to look at the result
/// sees it land on disk within a couple of seconds, not minutes.
pub const SNAPSHOT_WRITE_DEBOUNCE: Duration = Duration::from_secs(2);

/// The debounce clock, abstracted so scheduling can be tested by advancing
/// time by hand rather than by sleeping in real time.
pub trait Clock {
    fn now(&self) -> Instant;
}

/// The real clock, used everywhere outside tests.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Whether a document session has an edit sitting in memory that has not yet
/// reached its `.fw` file, and -- if so -- when the debounce window says it
/// is time to write it.
///
/// `due_at` is `None` exactly when the snapshot on disk already matches the
/// store in memory. It becomes `Some` the moment an operation lands, is
/// pushed forward by every further operation before it elapses (that is the
/// coalescing), and is cleared only once a write actually succeeds -- a
/// failed write leaves the deadline in the past, so the next scheduling tick
/// treats it as immediately due again rather than silently giving up.
#[derive(Default)]
pub struct PendingWrite {
    due_at: Option<Instant>,
}

impl PendingWrite {
    /// True whenever the in-memory store may hold something the last
    /// successful write did not: either a debounce window is running, or a
    /// previous write attempt failed and left one due immediately. Explicit
    /// flush points (Save As, Open, Package, window blur, app quit, ...)
    /// use this to decide whether they have anything to do at all.
    pub fn is_pending(&self) -> bool {
        self.due_at.is_some()
    }

    /// Records that an operation landed and (re)starts the debounce window
    /// from `now`. Called after every operation, including undo and redo --
    /// they change the document exactly as any other operation does.
    pub fn schedule(&mut self, clock: &dyn Clock, after: Duration) {
        self.due_at = Some(clock.now() + after);
    }

    /// True once the debounce window has elapsed (or a prior write failed,
    /// leaving the deadline already in the past) and a periodic tick should
    /// perform the write on this session's behalf.
    pub fn is_due(&self, clock: &dyn Clock) -> bool {
        self.due_at.is_some_and(|due| clock.now() >= due)
    }

    /// Called once a write has actually reached disk: nothing is pending.
    pub fn clear(&mut self) {
        self.due_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// A clock a test moves by hand instead of sleeping in real time.
    struct ManualClock(Cell<Instant>);

    impl ManualClock {
        fn new() -> Self {
            Self(Cell::new(Instant::now()))
        }

        fn advance(&self, by: Duration) {
            self.0.set(self.0.get() + by);
        }
    }

    impl Clock for ManualClock {
        fn now(&self) -> Instant {
            self.0.get()
        }
    }

    #[test]
    fn a_freshly_scheduled_write_is_pending_but_not_yet_due() {
        let clock = ManualClock::new();
        let mut pending = PendingWrite::default();
        pending.schedule(&clock, SNAPSHOT_WRITE_DEBOUNCE);
        assert!(pending.is_pending());
        assert!(!pending.is_due(&clock));
    }

    #[test]
    fn a_write_becomes_due_once_the_debounce_window_elapses() {
        let clock = ManualClock::new();
        let mut pending = PendingWrite::default();
        pending.schedule(&clock, SNAPSHOT_WRITE_DEBOUNCE);
        clock.advance(SNAPSHOT_WRITE_DEBOUNCE - Duration::from_millis(1));
        assert!(!pending.is_due(&clock));
        clock.advance(Duration::from_millis(1));
        assert!(pending.is_due(&clock));
    }

    #[test]
    fn a_further_operation_before_the_window_elapses_pushes_the_deadline_out() {
        // This is the whole point: a burst of operations must coalesce into
        // one write, not one per operation. Each new operation resets the
        // idle clock rather than adding a second, independent deadline.
        let clock = ManualClock::new();
        let mut pending = PendingWrite::default();
        pending.schedule(&clock, SNAPSHOT_WRITE_DEBOUNCE);
        clock.advance(Duration::from_millis(1500));
        pending.schedule(&clock, SNAPSHOT_WRITE_DEBOUNCE); // another edit lands
        clock.advance(Duration::from_millis(600));
        assert!(
            !pending.is_due(&clock),
            "the second operation should have pushed the deadline out"
        );
        clock.advance(Duration::from_millis(1500));
        assert!(pending.is_due(&clock));
    }

    #[test]
    fn clearing_after_a_successful_flush_leaves_nothing_pending() {
        let clock = ManualClock::new();
        let mut pending = PendingWrite::default();
        pending.schedule(&clock, SNAPSHOT_WRITE_DEBOUNCE);
        pending.clear();
        assert!(!pending.is_pending());
        assert!(!pending.is_due(&clock));
    }

    #[test]
    fn a_never_scheduled_write_is_neither_pending_nor_due() {
        let clock = ManualClock::new();
        let pending = PendingWrite::default();
        assert!(!pending.is_pending());
        assert!(!pending.is_due(&clock));
    }
}
