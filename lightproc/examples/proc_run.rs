//! A function that runs a future to completion on a dedicated thread.

use std::future::Future;
use std::sync::Arc;
use std::thread;

use crossbeam::channel;
use futures::executor;
use std::sync::atomic::AtomicUsize;
use lightproc::proc_handle::ProcHandle;
use lightproc::stack::ProcStack;
use lightproc::lightproc::LightProc;
use std::time::Duration;

/// Spawns a future on a new dedicated thread.
///
/// The returned handle can be used to await the output of the future.
fn spawn_on_thread<F, R>(fut: F) -> ProcHandle<R>
    where
        F: Future<Output = R> + Send + 'static,
        R: Send + 'static,
{
    // Create a channel that holds the task when it is scheduled for running.
    let (sender, receiver) = channel::unbounded();
    let sender = Arc::new(sender);
    let s = Arc::downgrade(&sender);

    // Wrap the future into one that disconnects the channel on completion.
    let future = async move {
        // When the inner future completes, the sender gets dropped and disconnects the channel.
        println!("exec");
        let _sender = sender;
        println!("exec1");
        fut.await
    };

    // Create a task that is scheduled by sending itself into the channel.
    let schedule = move |t| s.upgrade().unwrap().send(t).unwrap();
    let (proc, handle) = LightProc::build(
        future,
        schedule,
        ProcStack::default()
    );

    // Schedule the task by sending it into the channel.
    proc.schedule();

    // Spawn a thread running the task to completion.
    thread::spawn(move || {
        // Keep taking the task from the channel and running it until completion.
        for proc in receiver {
            println!("ad");
            proc.run();
            println!("ad2");
        }
    });

    handle
}

fn main() {
    executor::block_on(spawn_on_thread(async {
        println!("Hello, world!");
    }));

    thread::sleep(Duration::new(5, 0));
}
