use std::task::RawWakerVTable;

pub(crate) struct ProcVTable {
    /// The raw waker vtable.
    pub(crate) raw_waker: RawWakerVTable,

    pub(crate) schedule: unsafe fn(*const ()),
    //    // Callbacks
    //    pub(crate) callbacks: ProcCallbacks
}
