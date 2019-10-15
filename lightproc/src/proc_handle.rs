use std::ptr::NonNull;
use std::marker::PhantomData as marker;

pub struct ProcHandle<R, T> {
    raw_proc: NonNull<()>,
    _private: marker<(R, T)>
}

unsafe impl<R, T> Send for ProcHandle<R, T> {}
unsafe impl<R, T> Sync for ProcHandle<R, T> {}

impl<R, T> Unpin for ProcHandle<R, T> {}

impl<R, T> ProcHandle<R, T> {
    pub fn cancel(&self) {
        let ptr = self.raw_proc.as_ptr();
        unimplemented!()
    }
}
