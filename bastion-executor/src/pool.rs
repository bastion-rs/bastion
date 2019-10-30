use super::distributor::Distributor;

use super::load_balancer::LoadBalancer;
use super::run_queue::{Injector, Stealer, Worker};
use super::sleepers::Sleepers;
use super::worker;
use lazy_static::*;
use lightproc::prelude::*;
use std::future::Future;

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
        unimplemented!()
    }

    pub fn fetch_proc(&self, local: &Worker<LightProc>) -> Option<LightProc> {
        // Pop only from the local queue with full trust
        local.pop()
    }

    pub fn spawn<F, T>(&self, future: F, stack: ProcStack) -> RecoverableHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        // Log this `spawn` operation.
        let child_id = stack.get_pid() as u64;
        let parent_id = worker::get_proc_stack(|t| t.get_pid() as u64).unwrap_or(0);

        dbg!(parent_id);
        dbg!(child_id);

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
            let (stealers, workers) = distributor.assign();

            LoadBalancer().start(workers);

            Pool {
                injector: Injector::new(),
                stealers,
                sleepers: Sleepers::new(),
            }
        };
    }
    &*POOL
}
