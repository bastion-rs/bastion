//!
//! SMP parallelism based cache affine worker implementation
//!
//! This worker implementation relies on worker run queue statistics which are hold in the pinned global memory
//! where workload distribution calculated and amended to their own local queues.
use crate::load_balancer::{self, SmpStats};
use crate::pool::{self, Pool};
use crate::run_queue::{Steal, Worker};
use lightproc::prelude::*;
use std::cell::{Cell, UnsafeCell};
use std::time::{Duration, Instant};
use std::{iter, ptr};
use tracing::{trace};

/// The timeout we'll use when parking before an other Steal attempt
pub const THREAD_PARK_TIMEOUT: Duration = Duration::from_millis(1);

/// The amount of time we'll try to steal tasks before we start parking.
const STEAL_ATTEMPTS_TIMEOUT: Duration = Duration::from_millis(100);

///
/// Get the current process's stack
pub fn current() -> ProcStack {
    get_proc_stack(|proc| proc.clone())
        .expect("`proc::current()` called outside the context of the proc")
}

thread_local! {
    static STACK: Cell<*const ProcStack> = Cell::new(ptr::null_mut());
}

///
/// Set the current process's stack during the run of the future.
pub(crate) fn set_stack<F, R>(stack: *const ProcStack, f: F) -> R
where
    F: FnOnce() -> R,
{
    struct ResetStack<'a>(&'a Cell<*const ProcStack>);

    impl Drop for ResetStack<'_> {
        fn drop(&mut self) {
            self.0.set(ptr::null());
        }
    }

    STACK.with(|st| {
        st.set(stack);
        let _guard = ResetStack(st);

        f()
    })
}

pub(crate) fn get_proc_stack<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&ProcStack) -> R,
{
    let res = STACK.try_with(|st| unsafe { st.get().as_ref().map(f) });

    match res {
        Ok(Some(val)) => Some(val),
        Ok(None) | Err(_) => None,
    }
}

thread_local! {
    static QUEUE: UnsafeCell<Option<Worker<LightProc>>> = UnsafeCell::new(None);
}

pub(crate) fn schedule(proc: LightProc) {
    QUEUE.with(|queue| {
        let local = unsafe { (*queue.get()).as_ref() };

        match local {
            None => pool::get().injector.push(proc),
            Some(q) => q.push(proc),
        }
    });

    pool::get().sleepers.notify_one();
}

///
/// Fetch the process from the run queue.
/// Does the work of work-stealing if process doesn't exist in the local run queue.
pub fn fetch_proc(affinity: usize) -> Option<LightProc> {
    let pool = pool::get();

    QUEUE.with(|queue| {
        let local = unsafe { (*queue.get()).as_ref().unwrap() };
        local.pop().or_else(|| affine_steal(pool, local, affinity))
    })
}

fn affine_steal(pool: &Pool, local: &Worker<LightProc>, affinity: usize) -> Option<LightProc> {
    let steal_start = Instant::now();
    iter::repeat_with(|| {
        // Maybe we now have work to do
        if let Some(proc) = local.pop() {
            return Steal::Success(proc);
        }

        // First try to get procs from global queue
        pool.injector.steal_batch_and_pop(&local).or_else(|| {
            let sorted_load = load_balancer::stats().get_sorted_load();
            let mean_load = load_balancer::stats().mean();
            sorted_load
                .iter()
                // We don't wanna steal from ourselves
                .filter(|(core_id, load)| *load > mean_load && *core_id != affinity)
                .collect::<Vec<_>>()
                .first()
                .map(|(core_id, load)| {
                    let items_to_steal = load / 2;
                    if items_to_steal > 0 {
                        pool.stealers
                            .get(*core_id)
                            .unwrap()
                            .steal_batch_and_pop_with_amount(&local, items_to_steal)
                    } else {
                        pool.stealers
                            .get(*core_id)
                            .unwrap()
                            .steal_batch_and_pop(&local)
                    }
                })
                .map(|steal| {
                    if steal.is_retry() {
                        trace!("{:?} - steal attempt failed", std::thread::current().id());
                    }
                    steal
                })
                .unwrap_or_else(|| {
                    if steal_start.elapsed() > STEAL_ATTEMPTS_TIMEOUT {
                        trace!("{:?} - no load to steal", std::thread::current().id());
                        std::thread::park_timeout(THREAD_PARK_TIMEOUT);
                    }
                    Steal::Retry
                })
        })
    })
    // Loop while no task was stolen and any steal operation needs to be retried.
    .find(|s| !s.is_retry())
    // Extract the stolen task, if there is one.
    .and_then(|s| s.success())
}

pub(crate) fn update_stats(affinity: usize, local: &Worker<LightProc>) {
    load_balancer::stats().store_load(affinity, local.worker_run_queue_size());
    load_balancer::update()
}

pub(crate) fn main_loop(affinity: usize, local: Worker<LightProc>) {
    QUEUE.with(|queue| unsafe { *queue.get() = Some(local) });

    loop {
        QUEUE.with(|queue| {
            let local = unsafe { (*queue.get()).as_ref().unwrap() };
            update_stats(affinity, local);
        });

        match fetch_proc(affinity) {
            Some(proc) => set_stack(proc.stack(), || proc.run()),
            None => pool::get().sleepers.wait(),
        }
    }
}
