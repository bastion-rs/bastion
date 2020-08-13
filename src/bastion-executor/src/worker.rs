//!
//! SMP parallelism based cache affine worker implementation
//!
//! This worker implementation relies on worker run queue statistics which are hold in the pinned global memory
//! where workload distribution calculated and amended to their own local queues.
use crate::load_balancer::{self, LoadBalancer, SmpStats};
use crate::pool::{self, Pool};
use crate::run_queue::{Steal, Worker};
use lightproc::prelude::*;
use once_cell::sync::OnceCell;
use std::cell::{Cell, UnsafeCell};
use std::ptr;

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

static LOAD_BALANCER: once_cell::sync::OnceCell<LoadBalancer> = OnceCell::new();

pub(crate) fn load_balancer() -> &'static LoadBalancer {
    LOAD_BALANCER.get_or_init(LoadBalancer::default)
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
    // Maybe we now have work to do
    if let Some(proc) = local.pop() {
        return Some(proc);
    }

    let load_mean = load_balancer::stats().mean();

    // First try to get procs from global queue
    pool.injector
        .steal_batch_and_pop(&local)
        .or_else(|| {
            if let Some((core_id, load)) = load_balancer::stats()
                .get_sorted_load()
                .iter()
                // We don't wanna steal from ourselves
                // and we don't wanna steal from cores that are idle
                .filter(|(core_id, load)| *load > 0 && *core_id != affinity)
                .collect::<Vec<_>>()
                .first()
            {
                if load_mean > 0 {
                    return pool
                        .stealers
                        .get(*core_id)
                        .unwrap()
                        .steal_batch_and_pop_with_amount(&local, load_mean);
                } else {
                    return pool
                        .stealers
                        .get(*core_id)
                        .unwrap()
                        .steal_batch_and_pop_with_amount(&local, *load);
                }
            }
            Steal::Empty
        })
        .success()
}

pub(crate) fn update_stats(affinity: usize, local: &Worker<LightProc>) {
    load_balancer::stats().store_load(affinity, local.worker_run_queue_size());
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
