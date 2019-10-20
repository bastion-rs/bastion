use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::mem;
use std::ptr::NonNull;

use crate::proc_data::ProcData;
use crate::raw_proc::RawProc;
use crate::proc_handle::ProcHandle;
use crate::stack::*;


pub struct LightProc {
    /// A pointer to the heap-allocated task.
    pub(crate) raw_proc: NonNull<()>,
}

unsafe impl Send for LightProc {}
unsafe impl Sync for LightProc {}

impl LightProc {
    pub fn build<F, R, S>(future: F, schedule: S, stack: ProcStack) -> (LightProc, ProcHandle<R>)
        where
            F: Future<Output = R> + Send + 'static,
            R: Send + 'static,
            S: Fn(LightProc) + Send + Sync + 'static
    {
        let raw_task = RawProc::<F, R, S>::allocate(stack, future, schedule);
        let task = LightProc {
            raw_proc: raw_task
        };
        let handle = ProcHandle {
            raw_proc: raw_task,
            _marker: PhantomData,
        };
        (task, handle)
    }

    pub fn schedule(self) {
        let ptr = self.raw_proc.as_ptr();
        let pdata = ptr as *const ProcData;
        mem::forget(self);

        unsafe {
            ((*pdata).vtable.schedule)(ptr);
        }
    }

    pub fn run(self) {
        let ptr = self.raw_proc.as_ptr();
        let pdata = ptr as *const ProcData;
        mem::forget(self);

        unsafe {
            ((*pdata).vtable.run)(ptr);
        }
    }

    pub fn cancel(&self) {
        let ptr = self.raw_proc.as_ptr();
        let pdata = ptr as *const ProcData;

        unsafe {
            (*pdata).cancel();
        }
    }

    pub fn stack(&self) -> &ProcStack {
        let offset = ProcData::offset_stack();
        let ptr = self.raw_proc.as_ptr();

        unsafe {
            let raw = (ptr as *mut u8).add(offset) as *const ProcStack;
            &*raw
        }
    }
}

impl Drop for LightProc {
    fn drop(&mut self) {
        let ptr = self.raw_proc.as_ptr();
        let pdata = ptr as *const ProcData;

        unsafe {
            // Cancel the task.
            (*pdata).cancel();

            // Drop the future.
            ((*pdata).vtable.drop_future)(ptr);

            // Drop the task reference.
            ((*pdata).vtable.decrement)(ptr);
        }
    }
}

impl fmt::Debug for LightProc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr = self.raw_proc.as_ptr();
        let pdata = ptr as *const ProcData;

        f.debug_struct("Task")
            .field("pdata", unsafe { &(*pdata) })
            .field("stack", self.stack())
            .finish()
    }
}
