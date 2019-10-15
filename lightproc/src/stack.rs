use std::sync::atomic::AtomicUsize;

pub(crate) struct ProcStack {
    pub(crate) pid: AtomicUsize,

    pub(crate) after_start: unsafe fn(*const ()),

    pub(crate) after_complete: unsafe fn(*const ()),
}
