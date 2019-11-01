use crate::proc_handle::ProcHandle;
use crate::proc_stack::ProcStack;
use std::future::Future;
use std::panic::resume_unwind;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::thread;

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
}

impl<R> Future for RecoverableHandle<R> {
    type Output = Option<R>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.0).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Ok(val))) => Poll::Ready(Some(val)),
            Poll::Ready(Some(Err(err))) => {
                if let Some(after_panic_cb) = self.0.stack().after_panic.clone() {
                    (*after_panic_cb.clone())();
                }

                resume_unwind(err)
            }
        }
    }
}
