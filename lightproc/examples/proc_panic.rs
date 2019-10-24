use std::future::Future;

use std::thread;

use crossbeam::channel::{unbounded, Sender};
use futures::executor;
use lazy_static::lazy_static;
use lightproc::prelude::*;

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
        ProcStack::default()
            .with_pid(1)
            .with_before_start(|| {
                println!("Before start");
            })
            .with_after_complete(|| {
                println!("After complete");
            })
            .with_after_panic(|| {
                println!("After panic");
            }),
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
