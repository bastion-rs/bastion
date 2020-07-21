//!
//! Handle for recoverable process
use crate::proc_data::ProcData;
use crate::proc_handle::ProcHandle;
use crate::proc_stack::ProcStack;
use crate::state::State;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::thread;

/// Recoverable handle which encapsulates a standard Proc Handle and contain all panics inside.
///
/// Execution of `after_panic` will be immediate on polling the [RecoverableHandle]'s future.
pub struct RecoverableHandle<R>(pub(crate) ProcHandle<thread::Result<R>>);

impl<R> RecoverableHandle<R> {
    /// Cancels the proc.
    ///
    /// If the proc has already completed, calling this method will have no effect.
    ///
    /// When a proc is cancelled, its future cannot be polled again and will be dropped instead.
    pub fn cancel(&self) {
        self.0.cancel()
    }

    /// Returns a reference to the stack stored inside the proc.
    pub fn stack(&self) -> &ProcStack {
        self.0.stack()
    }

    /// Returns a state of the ProcHandle.
    pub fn state(&self) -> State {
        self.0.state()
    }
}

impl<R> Future for RecoverableHandle<R> {
    type Output = Option<R>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        match Pin::new(&mut self.0).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Ok(val))) => Poll::Ready(Some(val)),
            Poll::Ready(Some(Err(_))) => {
                if let Some(after_panic_cb) = self.0.stack().after_panic.clone() {
                    (*after_panic_cb.clone())(self.0.stack().state.clone());
                }

                Poll::Ready(None)
            }
        }
    }
}

impl<R> Debug for RecoverableHandle<R> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        let ptr = self.0.raw_proc.as_ptr();
        let pdata = ptr as *const ProcData;

        fmt.debug_struct("ProcHandle")
            .field("pdata", unsafe { &(*pdata) })
            .field("stack", self.stack())
            .finish()
    }
}
