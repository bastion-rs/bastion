//! A function that runs a future to completion on a dedicated thread.

use std::future::Future;
use std::sync::Arc;
use std::thread;

use crossbeam::channel;
use futures::executor;
use lightproc::prelude::*;
use std::sync::atomic::AtomicUsize;

fn spawn_on_thread<F, R>(fut: F) -> ProcHandle<R>
    where
        F: Future<Output = R> + Send + 'static,
        R: Send + 'static,
{
    let (sender, receiver) = channel::unbounded();
    let sender = Arc::new(sender);
    let s = Arc::downgrade(&sender);

    let future = async move {
        let _ = sender;
        fut.await
    };

    let schedule = move |t| s.upgrade().unwrap().send(t).unwrap();
    let (proc, handle) = LightProc::build(
        future,
        schedule,
        ProcStack {
            pid: AtomicUsize::new(1),
            after_complete: Some(Arc::new(|| {
                println!("After complete");
            })),
            before_start: Some(Arc::new(|| {
                println!("Before start");
            }))
        }
    );

    proc.schedule();

    thread::spawn(move || {
        for proc in receiver {
            proc.run();
        }
    });

    handle
}

fn main() {
    executor::block_on(spawn_on_thread(async {
        println!("Hello, world!");
    }));
}
