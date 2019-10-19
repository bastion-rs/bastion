use crate::system::RUNTIME;
use futures::channel::oneshot::{self, Receiver};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(super) struct Proc<T> {
    recver: Receiver<T>,
}

impl<T> Proc<T>
where
    T: Send + 'static,
{
    pub(super) fn spawn<F>(fut: F) -> Self
    where
        F: Future<Output = T> + Send + 'static,
    {
        let (sender, recver) = oneshot::channel();

        RUNTIME.spawn(async {
            // FIXME: Err(Error)
            sender.send(fut.await).ok();
        });

        Proc { recver }
    }
}

impl<T> Future for Proc<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        // FIXME: panics?
        Pin::new(&mut self.get_mut().recver)
            .poll(ctx)
            .map(|res| res.unwrap())
    }
}
