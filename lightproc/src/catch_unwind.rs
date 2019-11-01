use pin_utils::unsafe_pinned;
use std::any::Any;
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe, UnwindSafe};
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug)]
pub(crate) struct CatchUnwind<F>
where
    F: Future,
{
    future: F,
}

impl<F> CatchUnwind<F>
where
    F: Future + UnwindSafe,
{
    unsafe_pinned!(future: F);

    pub(crate) fn new(future: F) -> CatchUnwind<F> {
        CatchUnwind { future }
    }
}

impl<F> Future for CatchUnwind<F>
where
    F: Future + UnwindSafe,
{
    type Output = Result<F::Output, Box<dyn Any + Send>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        catch_unwind(AssertUnwindSafe(|| self.future().poll(cx)))?.map(Ok)
    }
}
