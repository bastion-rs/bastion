use std::cell::Cell;
use std::ptr;

use super::pool;
use lightproc::prelude::*;
use super::run_queue::Worker;

pub fn current() -> ProcStack {
    get_proc_stack(|proc| proc.clone())
        .expect("`proc::current()` called outside the context of the proc")
}

thread_local! {
    static STACK: Cell<*const ProcStack> = Cell::new(ptr::null_mut());
}

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
    let res = STACK.try_with(|st| unsafe {
        st.get().as_ref().map(f)
    });

    match res {
        Ok(Some(val)) => Some(val),
        Ok(None) | Err(_) => None,
    }
}

thread_local! {
    static IS_WORKER: Cell<bool> = Cell::new(false);
    static QUEUE: Cell<Option<Worker<LightProc>>> = Cell::new(None);
}

pub(crate) fn is_worker() -> bool {
    IS_WORKER.with(|is_worker| is_worker.get())
}

fn get_queue<F: FnOnce(&Worker<LightProc>) -> T, T>(f: F) -> T {
    QUEUE.with(|queue| {
        let q = queue.take().unwrap();
        let ret = f(&q);
        queue.set(Some(q));
        ret
    })
}

pub(crate) fn schedule(proc: LightProc) {
    if is_worker() {
        get_queue(|q| q.push(proc));
    } else {
        pool::get().injector.push(proc);
    }
    pool::get().sleepers.notify_one();
}

pub(crate) fn main_loop(worker: Worker<LightProc>) {
    IS_WORKER.with(|is_worker| is_worker.set(true));
    QUEUE.with(|queue| queue.set(Some(worker)));

    loop {
        match get_queue(|q| pool::get().fetch_proc(q)) {
            Some(proc) => set_stack(proc.stack(), || proc.run()),
            None => pool::get().sleepers.wait(),
        }
    }
}
