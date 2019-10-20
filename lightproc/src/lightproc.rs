use std::{alloc, mem, ptr};
use std::future::Future;
use std::marker::PhantomData as marker;
use std::ptr::NonNull;

use crate::proc_layout::ProcLayout;

use crate::layout_helpers::extend;

use std::alloc::Layout;
use crate::raw_proc::RawProc;
use crate::proc_handle::ProcHandle;
use crate::stack::ProcStack;
use crate::proc_data::ProcData;
use crate::align_proc::AlignProc;
use std::sync::atomic::Ordering;


#[derive(Debug)]
pub struct LightProc {
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
        let raw_proc = RawProc::<F, R, S>::allocate(
            future, schedule, stack
        );
        let proc = LightProc { raw_proc };
        let handle = ProcHandle { raw_proc, _private: marker };
        (proc, handle)
    }

    pub fn schedule(self) {
        let ptr = self.raw_proc.as_ptr();
        let header = ptr as *const ProcData;
        unsafe {
            ((*header).vtable.schedule)(ptr);
        }
    }

    pub fn run(self) {
        let ptr = self.raw_proc.as_ptr();
        let header = ptr as *const ProcData;
        mem::forget(self);

        unsafe {
            ((*header).vtable.run)(ptr);
        }
    }

    pub fn cancel(&self) {
        let ptr = self.raw_proc.as_ptr();
        let header = ptr as *const ProcData;

        unsafe {
            (*header).cancel();
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
        let header = ptr as *const ProcData;

        unsafe {
            // Cancel the task.
            (*header).cancel();

            // Drop the future.
            ((*header).vtable.drop_future)(ptr);

            // Drop the task reference.
            ((*header).vtable.decrement)(ptr);
        }
    }
}
