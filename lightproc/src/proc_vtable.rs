use std::task::RawWakerVTable;

/// The vtable for a proc.
pub(crate) struct ProcVTable {
    /// The raw waker vtable.
    pub(crate) raw_waker: RawWakerVTable,

    /// Schedules the proc.
    pub(crate) schedule: unsafe fn(*const ()),

    /// Drops the future inside the proc.
    pub(crate) drop_future: unsafe fn(*const ()),

    /// Returns a pointer to the output stored after completion.
    pub(crate) get_output: unsafe fn(*const ()) -> *const (),

    /// Drops a waker or a proc.
    pub(crate) decrement: unsafe fn(ptr: *const ()),

    /// Destroys the proc.
    pub(crate) destroy: unsafe fn(*const ()),

    /// Runs the proc.
    pub(crate) run: unsafe fn(*const ()),
}
