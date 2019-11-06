use super::distributor::Distributor;

use super::run_queue::{Injector, Stealer, Worker};
use super::sleepers::Sleepers;
use super::worker;
use lazy_static::*;
use lightproc::prelude::*;
use std::future::Future;

use crate::load_balancer;

pub fn spawn<F, T>(future: F, stack: ProcStack) -> RecoverableHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    self::get().spawn(future, stack)
}

pub struct Pool {
    pub injector: Injector<LightProc>,
    pub stealers: Vec<Stealer<LightProc>>,
    pub sleepers: Sleepers,
}

impl Pool {
    /// Error recovery for the fallen threads
    pub fn recover_async_thread() {
        // FIXME: Do recovery for fallen worker threads
        unimplemented!()
    }

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
