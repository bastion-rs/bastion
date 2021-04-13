//!
//! Handle for tasks which don't need to unwind panics inside
//! the given futures.
use crate::proc_data::ProcData;
use crate::proc_stack::ProcStack;
use crate::state::*;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::marker::{PhantomData, Unpin};
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::atomic::Ordering;
use std::task::{Context, Poll};

/// A handle that awaits the result of a proc.
///
/// This type is a future that resolves to an `Option<R>` where:
///
/// * `None` indicates the proc has panicked or was cancelled
/// * `Some(res)` indicates the proc has completed with `res`
pub struct ProcHandle<R> {
    /// A raw proc pointer.
    pub(crate) raw_proc: NonNull<()>,

    /// A marker capturing the generic type `R`.
    pub(crate) _marker: PhantomData<R>,
}

unsafe impl<R> Send for ProcHandle<R> {}
unsafe impl<R> Sync for ProcHandle<R> {}

impl<R> Unpin for ProcHandle<R> {}

impl<R> ProcHandle<R> {
    /// Cancels the proc.
    ///
    /// If the proc has already completed, calling this method will have no effect.
    ///
    /// When a proc is cancelled, its future cannot be polled again and will be dropped instead.
    pub fn cancel(&self) {
        let ptr = self.raw_proc.as_ptr();
        let pdata = ptr as *const ProcData;

        unsafe {
            let mut state = (*pdata).state.load(Ordering::Acquire);

            loop {
                // If the proc has been completed or closed, it can't be cancelled.
                if state & (COMPLETED | CLOSED) != 0 {
                    break;
                }

                // If the proc is not scheduled nor running, we'll need to schedule it.
                let new = if state & (SCHEDULED | RUNNING) == 0 {
                    (state | SCHEDULED | CLOSED) + REFERENCE
                } else {
                    state | CLOSED
                };

                // Mark the proc as closed.
                match (*pdata).state.compare_exchange_weak(
                    state,
                    new,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        // If the proc is not scheduled nor running, schedule it so that its future
                        // gets dropped by the executor.
                        if state & (SCHEDULED | RUNNING) == 0 {
                            ((*pdata).vtable.schedule)(ptr);
                        }

                        // Notify the awaiter that the proc has been closed.
                        if state & AWAITER != 0 {
                            (*pdata).notify();
                        }

                        break;
                    }
                    Err(s) => state = s,
                }
            }
        }
    }

    /// Returns a reference to the stack stored inside the proc.
    pub fn stack(&self) -> &ProcStack {
        let offset = ProcData::offset_stack();
        let ptr = self.raw_proc.as_ptr();

        unsafe {
            let raw = (ptr as *mut u8).add(offset) as *const ProcStack;
            &*raw
        }
    }

    /// Returns current state of the handle.
    pub fn state(&self) -> State {
        let ptr = self.raw_proc.as_ptr();
        let pdata = ptr as *const ProcData;
        let raw_state = unsafe { (*pdata).state.load(Ordering::SeqCst) };
        State::new(raw_state)
    }
}

impl<R> Future for ProcHandle<R> {
    type Output = Option<R>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let ptr = self.raw_proc.as_ptr();
        let pdata = ptr as *const ProcData;

        unsafe {
            let mut state = (*pdata).state.load(Ordering::Acquire);

            loop {
                // If the proc has been closed, notify the awaiter and return `None`.
                if state & CLOSED != 0 {
                    // Even though the awaiter is most likely the current proc, it could also be
                    // another proc.
                    (*pdata).notify_unless(cx.waker());
                    return Poll::Ready(None);
                }

                // If the proc is not completed, register the current proc.
                if state & COMPLETED == 0 {
                    // Replace the waker with one associated with the current proc. We need a
                    // safeguard against panics because dropping the previous waker can panic.
                    (*pdata).swap_awaiter(Some(cx.waker().clone()));

                    // Reload the state after registering. It is possible that the proc became
                    // completed or closed just before registration so we need to check for that.
                    state = (*pdata).state.load(Ordering::Acquire);

                    // If the proc has been closed, notify the awaiter and return `None`.
                    if state & CLOSED != 0 {
                        // Even though the awaiter is most likely the current proc, it could also
                        // be another proc.
                        (*pdata).notify_unless(cx.waker());
                        return Poll::Ready(None);
                    }

                    // If the proc is still not completed, we're blocked on it.
                    if state & COMPLETED == 0 {
                        return Poll::Pending;
                    }
                }

                // Since the proc is now completed, mark it as closed in order to grab its output.
                match (*pdata).state.compare_exchange(
                    state,
                    state | CLOSED,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        // Notify the awaiter. Even though the awaiter is most likely the current
                        // proc, it could also be another proc.
                        if state & AWAITER != 0 {
                            (*pdata).notify_unless(cx.waker());
                        }

                        // Take the output from the proc.
                        let output = ((*pdata).vtable.get_output)(ptr) as *mut R;
                        return Poll::Ready(Some(output.read()));
                    }
                    Err(s) => state = s,
                }
            }
        }
    }
}

impl<R> Debug for ProcHandle<R> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        let ptr = self.raw_proc.as_ptr();
        let pdata = ptr as *const ProcData;

        fmt.debug_struct("ProcHandle")
            .field("pdata", unsafe { &(*pdata) })
            .field("stack", self.stack())
            .finish()
    }
}

impl<R> Drop for ProcHandle<R> {
    fn drop(&mut self) {
        let ptr = self.raw_proc.as_ptr();
        let pdata = ptr as *const ProcData;

        // A place where the output will be stored in case it needs to be dropped.
        let mut output = None;

        unsafe {
            // Optimistically assume the `ProcHandle` is being dropped just after creating the
            // proc. This is a common case so if the handle is not used, the overhead of it is only
            // one compare-exchange operation.
            if let Err(mut state) = (*pdata).state.compare_exchange_weak(
                SCHEDULED | HANDLE | REFERENCE,
                SCHEDULED | REFERENCE,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                loop {
                    // If the proc has been completed but not yet closed, that means its output
                    // must be dropped.
                    if state & COMPLETED != 0 && state & CLOSED == 0 {
                        // Mark the proc as closed in order to grab its output.
                        match (*pdata).state.compare_exchange_weak(
                            state,
                            state | CLOSED,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        ) {
                            Ok(_) => {
                                // Read the output.
                                output = Some((((*pdata).vtable.get_output)(ptr) as *mut R).read());

                                // Update the state variable because we're continuing the loop.
                                state |= CLOSED;
                            }
                            Err(s) => state = s,
                        }
                    } else {
                        // If this is the last reference to the proc and it's not closed, then
                        // close it and schedule one more time so that its future gets dropped by
                        // the executor.
                        let new = if state & (!(REFERENCE - 1) | CLOSED) == 0 {
                            SCHEDULED | CLOSED | REFERENCE
                        } else {
                            state & !HANDLE
                        };

                        // Unset the handle flag.
                        match (*pdata).state.compare_exchange_weak(
                            state,
                            new,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        ) {
                            Ok(_) => {
                                // If this is the last reference to the proc, we need to either
                                // schedule dropping its future or destroy it.
                                if state & !(REFERENCE - 1) == 0 {
                                    if state & CLOSED == 0 {
                                        ((*pdata).vtable.schedule)(ptr);
                                    } else {
                                        ((*pdata).vtable.destroy)(ptr);
                                    }
                                }

                                break;
                            }
                            Err(s) => state = s,
                        }
                    }
                }
            }
        }

        // Drop the output if it was taken out of the proc.
        drop(output);
    }
}
