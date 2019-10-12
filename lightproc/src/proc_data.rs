use std::task::Waker;
use std::cell::UnsafeCell;
use proc_vtable::ProcVTable;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) struct ProcData {
    pub(crate) state: AtomicUsize,

    pub(crate) awaiter: UnsafeCell<Option<Waker>>,

    pub(crate) vtable: &'static ProcVTable,
}
