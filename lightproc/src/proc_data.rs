use std::alloc::Layout;
use std::cell::Cell;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::Waker;

use crossbeam_utils::Backoff;

use crate::state::*;
use crate::proc_stack::*;
use crate::proc_vtable::TaskVTable;
use crate::layout_helpers::extend;

/// The pdata of a task.
///
/// This pdata is stored right at the beginning of every heap-allocated task.
pub(crate) struct ProcData {
    /// Current state of the task.
    ///
    /// Contains flags representing the current state and the reference count.
    pub(crate) state: AtomicUsize,

    /// The task that is blocked on the `JoinHandle`.
    ///
    /// This waker needs to be woken once the task completes or is closed.
    pub(crate) awaiter: Cell<Option<Waker>>,

    /// The virtual table.
    ///
    /// In addition to the actual waker virtual table, it also contains pointers to several other
    /// methods necessary for bookkeeping the heap-allocated task.
    pub(crate) vtable: &'static TaskVTable,
}

impl ProcData {
    /// Cancels the task.
    ///
    /// This method will only mark the task as closed and will notify the awaiter, but it won't
    /// reschedule the task if it's not completed.
    pub(crate) fn cancel(&self) {
        let mut state = self.state.load(Ordering::Acquire);

        loop {
            // If the task has been completed or closed, it can't be cancelled.
            if state & (COMPLETED | CLOSED) != 0 {
                break;
            }

            // Mark the task as closed.
            match self.state.compare_exchange_weak(
                state,
                state | CLOSED,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    // Notify the awaiter that the task has been closed.
                    if state & AWAITER != 0 {
                        self.notify();
                    }

                    break;
                }
                Err(s) => state = s,
            }
        }
    }

    /// Notifies the task blocked on the task.
    ///
    /// If there is a registered waker, it will be removed from the pdata and woken.
    #[inline]
    pub(crate) fn notify(&self) {
        if let Some(waker) = self.swap_awaiter(None) {
            // We need a safeguard against panics because waking can panic.
            waker.wake();
        }
    }

    /// Notifies the task blocked on the task unless its waker matches `current`.
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
        let layout_t = Layout::new::<ProcStack>();
        let (_, offset_t) = extend(layout_pdata, layout_t);
        offset_t
    }
}

impl fmt::Debug for ProcData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.state.load(Ordering::SeqCst);

        f.debug_struct("ProcData")
            .field("scheduled", &(state & SCHEDULED != 0))
            .field("running", &(state & RUNNING != 0))
            .field("completed", &(state & COMPLETED != 0))
            .field("closed", &(state & CLOSED != 0))
            .field("awaiter", &(state & AWAITER != 0))
            .field("handle", &(state & HANDLE != 0))
            .field("ref_count", &(state / REFERENCE))
            .finish()
    }
}
