use std::future::Future;
use std::sync::Arc;
use std::thread;

use crossbeam::channel::{unbounded, Sender};
use futures::{executor, FutureExt};
use lightproc::prelude::*;
use std::sync::atomic::AtomicUsize;
use lazy_static::lazy_static;
use std::panic::AssertUnwindSafe;
use lightproc::proc_ext::ProcFutureExt;
use lightproc::recoverable_handle::RecoverableHandle;


fn spawn_on_thread<F, R>(future: F) -> RecoverableHandle<R>
    where
        F: Future<Output = R> + Send + 'static,
        R: Send + 'static,
{
    lazy_static! {
        // A channel that holds scheduled tasks.
        static ref QUEUE: Sender<LightProc> = {
            let (sender, receiver) = unbounded::<LightProc>();

            // Start the executor thread.
            thread::spawn(move || {
                for proc in receiver {
                    proc.run();
                }
            });

            sender
        };
    }

    let schedule = |t| QUEUE.send(t).unwrap();
    let (proc, handle) = LightProc::recoverable(
        future,
        schedule,
        ProcStack {
            pid: AtomicUsize::new(1),
            after_complete: Some(Arc::new(|| {
                println!("After complete");
            })),
            before_start: Some(Arc::new(|| {
                println!("Before start");
            })),
            after_panic: Some(Arc::new(|| {
                println!("After panic");
            }))
        },
    );

    proc.schedule();

    handle
}

fn main() {
    let handle = spawn_on_thread(async {
        panic!("Panic here!");
    });

    executor::block_on(handle);
}
