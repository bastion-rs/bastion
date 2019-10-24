use std::alloc::Layout;

#[derive(Clone, Copy)]
pub(crate) struct TaskLayout {
    /// Memory layout of the whole proc.
    pub(crate) layout: Layout,

    /// Offset into the proc at which the stack is stored.
    pub(crate) offset_t: usize,

    /// Offset into the proc at which the schedule function is stored.
    pub(crate) offset_s: usize,

    /// Offset into the proc at which the future is stored.
    pub(crate) offset_f: usize,

    /// Offset into the proc at which the output is stored.
    pub(crate) offset_r: usize,
}
