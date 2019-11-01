use std::alloc::Layout;

#[derive(Clone, Copy)]
pub(crate) struct ProcLayout {
    /// Memory layout of the whole proc.
    pub(crate) layout: Layout,

    /// Offset into the proc at which the stack is stored.
    pub(crate) offset_stack: usize,

    /// Offset into the proc at which the schedule function is stored.
    pub(crate) offset_schedule: usize,

    /// Offset into the proc at which the future is stored.
    pub(crate) offset_future: usize,

    /// Offset into the proc at which the output is stored.
    pub(crate) offset_output: usize,
}
