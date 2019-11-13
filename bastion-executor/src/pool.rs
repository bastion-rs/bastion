//!
//! Pool of threads to run lightweight processes
//!
//! Pool management and tracking belongs here.
//! We spawn futures onto the pool with [spawn] method of global run queue or
//! with corresponding [Worker]'s spawn method.
use crate::distributor::Distributor;
use crate::run_queue::{Injector, Stealer};
use crate::sleepers::Sleepers;
use crate::worker;
use lazy_static::lazy_static;
use lightproc::prelude::*;
use std::future::Future;

///
/// Spawn a process (which contains future + process stack) onto the executor from the global level.
///
/// # Example
/// ```rust
/// use bastion_executor::prelude::*;
/// use lightproc::prelude::*;
///
/// let pid = 1;
/// let stack = ProcStack::default().with_pid(pid);
///
/// let handle = spawn(
///     async {
///         panic!("test");
///     },
///     stack.clone(),
/// );
///
/// run(
///     async {
///         handle.await;
///     },
///     stack.clone(),
/// );
/// ```
pub fn spawn<F, T>(future: F, stack: ProcStack) -> RecoverableHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    self::get().spawn(future, stack)
}

///
/// Pool that global run queue, stealers of the workers, and parked threads.
#[derive(Debug)]
pub struct Pool {
    ///
    /// Global run queue implementation
    pub injector: Injector<LightProc>,
    ///
    /// Stealers of the workers
    pub stealers: Vec<Stealer<LightProc>>,
    ///
    /// Container of parked threads
    pub sleepers: Sleepers,
}

impl Pool {
    /// Error recovery for the fallen threads
    pub fn recover_async_thread() {
        // FIXME: Do recovery for fallen worker threads
        unimplemented!()
    }

    ///
    /// Spawn a process (which contains future + process stack) onto the executor via [Pool] interface.
    pub fn spawn<F, T>(&self, future: F, stack: ProcStack) -> RecoverableHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        // Log this `spawn` operation.
        let _child_id = stack.get_pid() as u64;
        let _parent_id = worker::get_proc_stack(|t| t.get_pid() as u64).unwrap_or(0);

        let (task, handle) = LightProc::recoverable(future, worker::schedule, stack);
        task.schedule();
        handle
    }
}

///
/// Acquire the static Pool reference
#[inline]
pub fn get() -> &'static Pool {
    lazy_static! {
        static ref POOL: Pool = {
            let distributor = Distributor::new();
            let stealers = distributor.assign();

            Pool {
                injector: Injector::new(),
                stealers,
                sleepers: Sleepers::new(),
            }
        };
    }
    &*POOL
}
