use rustc_hash::FxHashMap;
use std::alloc::Layout;
use crate::proc_data::ProcData;
use crate::stack::ProcStack;

#[derive(Clone, Debug)]
pub struct ProcLayout {
    pub layout: Layout,
    pub offset_table: FxHashMap<&'static str, usize>,
}

impl Default for ProcLayout {
    fn default() -> Self {
        ProcLayout {
            layout: Layout::new::<ProcData>(),
            offset_table: FxHashMap::default(),
        }
    }
}
