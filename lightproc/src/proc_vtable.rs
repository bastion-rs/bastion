use std::task::RawWakerVTable;

/// The vtable for a task.
pub(crate) struct ProcVTable {
    /// The raw waker vtable.
    pub(crate) raw_waker: RawWakerVTable,

    /// Schedules the task.
    pub(crate) schedule: unsafe fn(*const ()),

    /// Drops the future inside the task.
    pub(crate) drop_future: unsafe fn(*const ()),

    /// Returns a pointer to the output stored after completion.
    pub(crate) get_output: unsafe fn(*const ()) -> *const (),

    /// Drops a waker or a task.
    pub(crate) decrement: unsafe fn(ptr: *const ()),

    /// Destroys the task.
    pub(crate) destroy: unsafe fn(*const ()),

    /// Runs the task.
    pub(crate) run: unsafe fn(*const ()),
}
