use crossbeam::channel::{unbounded, Sender};
use futures::executor;
use lazy_static::lazy_static;
use lightproc::prelude::*;
use std::future::Future;
use std::thread;
use lightproc::proc_state::EmptyProcState;
use std::sync::{Arc, Mutex};

#[derive(Copy, Clone)]
pub struct GlobalState {
    pub amount: usize
}

fn spawn_on_thread<F, R>(future: F, gs: Arc<Mutex<GlobalState>>) -> RecoverableHandle<R>
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

    let stack = ProcStack::default()
    .with_pid(1)
    .with_state(gs)
    .with_before_start(|s: &mut Arc<Mutex<GlobalState>>| {
        println!("Before start");
        s.clone().lock().unwrap().amount += 1;
    })
    .with_after_complete(|s: &mut Arc<Mutex<GlobalState>>| {
        println!("After complete");
        s.clone().lock().unwrap().amount += 2;
    })
    .with_after_panic(|s: &mut Arc<Mutex<GlobalState>>| {
        println!("After panic");
        s.clone().lock().unwrap().amount += 3;
    });

    let schedule = |t| QUEUE.send(t).unwrap();
    let (proc, handle) = LightProc::recoverable(
        future,
        schedule,
        stack,
    );

    proc.schedule();

    handle
}

fn main() {
    let mut gs = Arc::new(Mutex::new(GlobalState { amount: 0 }));
    let handle = spawn_on_thread(async {
        panic!("Panic here!");
    }, gs.clone());

    executor::block_on(handle);

    // 0 at the start
    // +1 before the start
    // +2 after panic occurs and completion triggers
    // +3 after panic triggers
    assert_eq!(gs.clone().lock().unwrap().amount, 6);
}
