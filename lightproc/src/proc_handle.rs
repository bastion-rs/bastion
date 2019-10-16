use std::marker::PhantomData as marker;
use std::ptr::NonNull;

pub struct ProcHandle<R, T> {
    pub(crate) raw_proc: NonNull<()>,
    pub(crate) _private: marker<(R, T)>,
}

unsafe impl<R, T> Send for ProcHandle<R, T> {}
unsafe impl<R, T> Sync for ProcHandle<R, T> {}

impl<R, T> Unpin for ProcHandle<R, T> {}

impl<R, T> ProcHandle<R, T> {
    pub fn cancel(&self) {
        let _ptr = self.raw_proc.as_ptr();
        unimplemented!()
    }
}
