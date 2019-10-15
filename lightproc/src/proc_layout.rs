use std::alloc::Layout;
use crate::stack::ProcStack;

#[derive(Clone, Copy)]
pub struct ProcLayout {
    pub layout: Layout,
}

impl Default for ProcLayout {
    fn default() -> Self {
        ProcLayout {
            layout: Layout::new::<ProcStack>()
        }
    }
}
