use crate::stack::ProcStack;
use rustc_hash::FxHashMap;
use std::alloc::Layout;

#[derive(Clone)]
pub struct ProcLayout {
    pub layout: Layout,
    pub offset_table: FxHashMap<&'static str, usize>,
}

impl Default for ProcLayout {
    fn default() -> Self {
        ProcLayout {
            layout: Layout::new::<ProcStack>(),
            offset_table: FxHashMap::default(),
        }
    }
}
