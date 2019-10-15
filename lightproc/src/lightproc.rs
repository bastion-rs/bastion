use std::ptr::NonNull;
use std::marker::PhantomData as marker;
use std::future::Future;
use crate::proc_handle::ProcHandle;
use crate::raw_proc::RawProc;
use crate::stack::ProcStack;
use crate::proc_layout::ProcLayout;
use std::alloc;
use std::alloc::Layout;
use crate::layout_helpers::extend;

pub struct LightProc<T> {
    pub(crate) raw_proc: NonNull<()>,

    pub(crate) proc_layout: ProcLayout,
    pub(crate) _private: marker<T>
}

unsafe impl<T> Send for LightProc<T> {}
unsafe impl<T> Sync for LightProc<T> {}

impl<T> LightProc<T> {
    pub fn new() -> LightProc<T> {
        let proc_layout = ProcLayout::default();

        unsafe {
            LightProc {
                raw_proc: NonNull::dangling(),
                proc_layout,
                _private: marker
            }
        }
    }

    pub fn with_future<F, R>(mut self, f: F)
    where
        F: Future<Output = R> + Send + 'static
    {
        let fut_mem = Layout::new::<F>();
        let (new_layout, offset_t) =
            extend(self.proc_layout.layout, fut_mem);
        self.proc_layout.layout = new_layout;

    }
}
