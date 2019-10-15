use std::alloc;
use std::future::Future;
use std::marker::PhantomData as marker;
use std::ptr::NonNull;

use crate::proc_layout::ProcLayout;

use crate::layout_helpers::extend;
use crate::raw_proc::RawProc;
use std::alloc::Layout;

pub struct LightProc<T> {
    pub(crate) raw_proc: NonNull<()>,

    pub(crate) proc_layout: ProcLayout,
    pub(crate) _private: marker<T>,
}

unsafe impl<T> Send for LightProc<T> {}
unsafe impl<T> Sync for LightProc<T> {}

impl<T> LightProc<T> {
    pub fn new() -> LightProc<T> {
        let proc_layout = ProcLayout::default();

        unsafe {
            LightProc {
                raw_proc: NonNull::new(alloc::alloc(proc_layout.layout) as *mut ()).unwrap(),
                proc_layout,
                _private: marker,
            }
        }
    }

    pub fn with_future<F, R>(mut self, f: F)
    where
        F: Future<Output = R> + Send + 'static,
    {
        let fut_mem = Layout::new::<F>();
        let (new_layout, offset_f) = extend(self.proc_layout.layout, fut_mem);
        self.proc_layout.layout = new_layout;
        self.proc_layout.offset_table.insert("future", offset_f);

        unsafe {
            alloc::realloc(
                self.raw_proc.as_ptr() as *mut u8,
                self.proc_layout.layout,
                self.proc_layout.layout.size(),
            );
        }

        //        RawProc::from_ptr(self.raw_proc.as_ptr(), self.proc_layout);
    }
}
