//!
//! Pool of threads to run heavy processes
//!
//! We spawn futures onto the pool with [spawn_blocking] method of global run queue or
//! with corresponding [Worker]'s spawn method.

use crate::thread_manager::{DynamicPoolManager, DynamicRunner};
use crossbeam_channel::{unbounded, Receiver, Sender};
use lazy_static::lazy_static;
use lightproc::lightproc::LightProc;
use lightproc::proc_stack::ProcStack;
use lightproc::recoverable_handle::RecoverableHandle;
use once_cell::sync::{Lazy, OnceCell};
use std::future::Future;
use std::iter::Iterator;
use std::sync::Arc;
use std::time::Duration;
use std::{env, thread};
use tracing::trace;

/// If low watermark isn't configured this is the default scaler value.
/// This value is used for the heuristics of the scaler
const DEFAULT_LOW_WATERMARK: u64 = 2;

const THREAD_RECV_TIMEOUT: Duration = Duration::from_millis(100);

/// Spawns a blocking task.
///
/// The task will be spawned onto a thread pool specifically dedicated to blocking tasks.
pub fn spawn_blocking<F, R>(future: F, stack: ProcStack) -> RecoverableHandle<R>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let (task, handle) = LightProc::recoverable(future, schedule, stack);
    task.schedule();
    handle
}

struct BlockingRunner {
    // We keep a handle to the tokio runtime here to make sure
    // it will never be dropped while the DynamicPoolManager is alive,
    // In case we need to spin up some threads.
    #[cfg(feature = "runtime-tokio")]
    runtime_handle: tokio::runtime::Handle,
}

impl DynamicRunner for BlockingRunner {
    fn run_static(&self, park_timeout: Duration) -> ! {
        loop {
            while let Ok(task) = POOL.receiver.recv_timeout(THREAD_RECV_TIMEOUT) {
                trace!("static thread: running task");
                self.run(task);
            }

            trace!("static: empty queue, parking with timeout");
            thread::park_timeout(park_timeout);
        }
    }
    fn run_dynamic(&self, parker: &dyn Fn()) -> ! {
        loop {
            while let Ok(task) = POOL.receiver.recv_timeout(THREAD_RECV_TIMEOUT) {
                trace!("dynamic thread: running task");
                self.run(task);
            }
            trace!(
                "dynamic thread: parking - {:?}",
                std::thread::current().id()
            );
            parker();
        }
    }
    fn run_standalone(&self) {
        while let Ok(task) = POOL.receiver.recv_timeout(THREAD_RECV_TIMEOUT) {
            self.run(task);
        }
        trace!("standalone thread: quitting.");
    }
}

impl BlockingRunner {
    fn run(&self, task: LightProc) {
        #[cfg(feature = "runtime-tokio")]
        {
            self.runtime_handle.spawn_blocking(|| task.run());
        }
        #[cfg(not(feature = "runtime-tokio"))]
        {
            task.run();
        }
    }
}

/// Pool interface between the scheduler and thread pool
struct Pool {
    sender: Sender<LightProc>,
    receiver: Receiver<LightProc>,
}

static DYNAMIC_POOL_MANAGER: OnceCell<DynamicPoolManager> = OnceCell::new();

static POOL: Lazy<Pool> = Lazy::new(|| {
    let runner = Arc::new(BlockingRunner {
        // We use current() here instead of try_current()
        // because we want bastion to crash as soon as possible
        // if there is no available runtime.
        #[cfg(feature = "runtime-tokio")]
        runtime_handle: tokio::runtime::Handle::current(),
    });

    DYNAMIC_POOL_MANAGER
        .set(DynamicPoolManager::new(*low_watermark() as usize, runner))
        .expect("couldn't create dynamic pool manager");
    DYNAMIC_POOL_MANAGER
        .get()
        .expect("couldn't get static pool manager")
        .initialize();

    let (sender, receiver) = unbounded();
    Pool { sender, receiver }
});

/// Enqueues work, attempting to send to the thread pool in a
/// nonblocking way and spinning up needed amount of threads
/// based on the previous statistics without relying on
/// if there is not a thread ready to accept the work or not.
fn schedule(t: LightProc) {
    if let Err(err) = POOL.sender.try_send(t) {
        // We were not able to send to the channel without
        // blocking.
        POOL.sender.send(err.into_inner()).unwrap();
    }

    // Add up for every incoming scheduled task
    DYNAMIC_POOL_MANAGER.get().unwrap().increment_frequency();
}

///
/// Low watermark value, defines the bare minimum of the pool.
/// Spawns initial thread set.
/// Can be configurable with env var `BASTION_BLOCKING_THREADS` at runtime.
#[inline]
fn low_watermark() -> &'static u64 {
    lazy_static! {
        static ref LOW_WATERMARK: u64 = {
            env::var_os("BASTION_BLOCKING_THREADS")
                .map(|x| x.to_str().unwrap().parse::<u64>().unwrap())
                .unwrap_or(DEFAULT_LOW_WATERMARK)
        };
    }

    &*LOW_WATERMARK
}
