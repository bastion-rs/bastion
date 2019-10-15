use crate::proc_data::ProcData;
use std::future::Future;
use crate::lightproc::LightProc;
use std::ptr::NonNull;
use std::sync::atomic::AtomicUsize;
use std::cell::UnsafeCell;
use std::task::RawWakerVTable;
use std::alloc::{Layout, alloc};
use crate::proc_layout::ProcLayout;

/// Raw pointers to the fields of a task.
pub struct RawProc<F, R, S, T> {
    pub(crate) pdata: *const ProcData,

    pub(crate) schedule: *const S,

    pub(crate) stack: *mut T,

    pub(crate) future: *mut F,

    pub(crate) output: *mut R,
}

impl<F, R, S, T> Copy for RawProc<F, R, S, T> {}

impl<F, R, S, T> Clone for RawProc<F, R, S, T> {
    fn clone(&self) -> Self {
        Self {
            pdata: self.pdata,
            schedule: self.schedule,
            stack: self.stack,
            future: self.future,
            output: self.output,
        }
    }
}

impl<F, R, S, T> RawProc<F, R, S, T>
    where
        F: Future<Output = R> + Send + 'static,
        R: Send + 'static,
        S: Fn(LightProc<T>) + Send + Sync + 'static,
        T: Send + 'static,
{

}
