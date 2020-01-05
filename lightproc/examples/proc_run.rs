use crossbeam::channel;
use futures::executor;
use lightproc::prelude::*;
use std::future::Future;
use std::sync::Arc;
use std::thread;

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
        ProcStack::default()
            .with_pid(1)
            .with_before_start(|s: EmptyProcState| {
                println!("Before start");
                s
            })
            .with_after_complete(|s: EmptyProcState| {
                println!("After complete");
                s
            }),
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
