use crossbeam::channel::{unbounded, Sender};
use futures::executor;
use lazy_static::lazy_static;
use lightproc::prelude::*;
use std::future::Future;
use std::thread;

fn spawn_on_thread<F, R>(future: F) -> RecoverableHandle<R>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    lazy_static! {
        // A channel that holds scheduled procs.
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
            .with_before_start(|s: EmptyProcState| {
                println!("Before start");
                s
            })
            .with_after_complete(|s: EmptyProcState| {
                println!("After complete");
                s
            })
            .with_after_panic(|s: EmptyProcState| {
                println!("After panic");
                s
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
