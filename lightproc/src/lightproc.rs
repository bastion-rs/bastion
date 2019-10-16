use std::alloc;
use std::future::Future;
use std::marker::PhantomData as marker;
use std::ptr::NonNull;

use crate::proc_layout::ProcLayout;

use crate::layout_helpers::extend;

use std::alloc::Layout;
use crate::raw_proc::RawProc;
use crate::proc_handle::ProcHandle;
use crate::stack::ProcStack;

#[derive(Debug)]
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

    pub fn with_future<F, R>(mut self, f: F) -> Self
    where
        F: Future<Output = R> + Send + 'static,
        R: Send + 'static,
    {
        let fut_mem = Layout::new::<F>();
        let (new_layout, offset_f) = extend(self.proc_layout.layout, fut_mem);
        self.proc_layout.offset_table.insert("future", offset_f);

        self.reallocate(new_layout);

        let rawp =
            RawProc::<F, R, usize, usize>::from_ptr(
                self.raw_proc.as_ptr(), &self.proc_layout);

        unsafe {
            rawp.future.write(f);
        }

        self
    }

    pub fn with_schedule<S>(mut self, s: S) -> Self
        where
            S: Fn(LightProc<T>) + Send + Sync + 'static,
            T: Send + 'static,
    {
        let sched_mem = Layout::new::<S>();
        let (new_layout, offset_s) = extend(self.proc_layout.layout, sched_mem);
        self.proc_layout.offset_table.insert("schedule", offset_s);

        self.reallocate(new_layout);

        let rawp =
            RawProc::<usize, usize, S, T>::from_ptr(
                self.raw_proc.as_ptr(), &self.proc_layout);

        unsafe {
            (rawp.schedule as *mut S).write(s);
        }

        self
    }

    pub fn with_stack(mut self, st: T) -> Self
        where T: Send + 'static,
    {
        let stack_mem = Layout::new::<T>();
        let (new_layout, offset_st) = extend(self.proc_layout.layout, stack_mem);
        self.proc_layout.offset_table.insert("stack", offset_st);

        self.reallocate(new_layout);

        let rawp =
            RawProc::<usize, usize, usize, T>::from_ptr(
                self.raw_proc.as_ptr(), &self.proc_layout);

        unsafe {
            rawp.stack.write(st);
        }

        self
    }

    pub fn returning<R>(mut self) -> (LightProc<T>, ProcHandle<R, T>) {
        let raw_proc = self.raw_proc;
        let proc = LightProc {
            raw_proc,
            proc_layout: self.proc_layout,
            _private: marker,
        };
        let handle = ProcHandle {
            raw_proc,
            _private: marker,
        };
        (proc, handle)
    }

    fn reallocate(&mut self, added: Layout) {
        unsafe {
            let pointer = alloc::realloc(
                self.raw_proc.as_ptr() as *mut u8,
                self.proc_layout.layout,
                added.size(),
            );
            self.raw_proc = NonNull::new(pointer as *mut ()).unwrap()
        }

        self.proc_layout.layout = added;
    }
}
