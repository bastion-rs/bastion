use crate::layout_helpers::extend;
use crate::proc_stack::ProcStack;
use crate::proc_vtable::ProcVTable;
use crate::state::*;
use crossbeam_utils::Backoff;
use std::alloc::Layout;
use std::cell::Cell;
use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::Waker;

/// The pdata of a proc.
///
/// This pdata is stored right at the beginning of every heap-allocated proc.
pub(crate) struct ProcData {
    /// Current state of the proc.
    ///
    /// Contains flags representing the current state and the reference count.
    pub(crate) state: AtomicUsize,

    /// The proc that is blocked on the `ProcHandle`.
    ///
    /// This waker needs to be woken once the proc completes or is closed.
    pub(crate) awaiter: Cell<Option<Waker>>,

    /// The virtual table.
    ///
    /// In addition to the actual waker virtual table, it also contains pointers to several other
    /// methods necessary for bookkeeping the heap-allocated proc.
    pub(crate) vtable: &'static ProcVTable,
}

impl ProcData {
    /// Cancels the proc.
    ///
    /// This method will only mark the proc as closed and will notify the awaiter, but it won't
    /// reschedule the proc if it's not completed.
    pub(crate) fn cancel(&self) {
        let mut state = self.state.load(Ordering::Acquire);

        loop {
            // If the proc has been completed or closed, it can't be cancelled.
            if state & (COMPLETED | CLOSED) != 0 {
                break;
            }

            // Mark the proc as closed.
            match self.state.compare_exchange_weak(
                state,
                state | CLOSED,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    // Notify the awaiter that the proc has been closed.
                    if state & AWAITER != 0 {
                        self.notify();
                    }

                    break;
                }
                Err(s) => state = s,
            }
        }
    }

    /// Notifies the proc blocked on the proc.
    ///
    /// If there is a registered waker, it will be removed from the pdata and woken.
    #[inline]
    pub(crate) fn notify(&self) {
        if let Some(waker) = self.swap_awaiter(None) {
            // We need a safeguard against panics because waking can panic.
            waker.wake();
        }
    }

    /// Notifies the proc blocked on the proc unless its waker matches `current`.
    ///
    /// If there is a registered waker, it will be removed from the pdata.
    #[inline]
    pub(crate) fn notify_unless(&self, current: &Waker) {
        if let Some(waker) = self.swap_awaiter(None) {
            if !waker.will_wake(current) {
                // We need a safeguard against panics because waking can panic.
                waker.wake();
            }
        }
    }

    /// Swaps the awaiter and returns the previous value.
    #[inline]
    pub(crate) fn swap_awaiter(&self, new: Option<Waker>) -> Option<Waker> {
        let new_is_none = new.is_none();

        // We're about to try acquiring the lock in a loop. If it's already being held by another
        // thread, we'll have to spin for a while so it's best to employ a backoff strategy.
        let backoff = Backoff::new();
        loop {
            // Acquire the lock. If we're storing an awaiter, then also set the awaiter flag.
            let state = if new_is_none {
                self.state.fetch_or(LOCKED, Ordering::Acquire)
            } else {
                self.state.fetch_or(LOCKED | AWAITER, Ordering::Acquire)
            };

            // If the lock was acquired, break from the loop.
            if state & LOCKED == 0 {
                break;
            }

            // Snooze for a little while because the lock is held by another thread.
            backoff.snooze();
        }

        // Replace the awaiter.
        let old = self.awaiter.replace(new);

        // Release the lock. If we've cleared the awaiter, then also unset the awaiter flag.
        if new_is_none {
            self.state.fetch_and(!LOCKED & !AWAITER, Ordering::Release);
        } else {
            self.state.fetch_and(!LOCKED, Ordering::Release);
        }

        old
    }

    #[inline]
    pub(crate) fn offset_stack() -> usize {
        let layout_pdata = Layout::new::<ProcData>();
        let layout_stack = Layout::new::<ProcStack>();
        let (_, offset_stack) = extend(layout_pdata, layout_stack);
        offset_stack
    }
}

impl Debug for ProcData {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        let state = self.state.load(Ordering::SeqCst);

        fmt.debug_struct("ProcData")
            .field("scheduled", &(state & SCHEDULED != 0))
            .field("running", &(state & RUNNING != 0))
            .field("completed", &(state & COMPLETED != 0))
            .field("closed", &(state & CLOSED != 0))
            .field("handle", &(state & HANDLE != 0))
            .field("awaiter", &(state & AWAITER != 0))
            .field("locked", &(state & LOCKED != 0))
            .field("ref_count", &(state / REFERENCE))
            .finish()
    }
}
