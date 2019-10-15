use crate::proc_vtable::ProcVTable;
use std::cell::UnsafeCell;
use std::sync::atomic::AtomicUsize;
use std::task::Waker;

pub(crate) struct ProcData {
    pub(crate) state: AtomicUsize,

    pub(crate) awaiter: UnsafeCell<Option<Waker>>,

    pub(crate) vtable: &'static ProcVTable,
}
