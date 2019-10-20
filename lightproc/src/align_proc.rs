use crate::proc_layout::ProcLayout;
use std::ptr::NonNull;
use crate::proc_data::ProcData;
use std::sync::atomic::AtomicUsize;
use crate::state::*;
use std::cell::Cell;
use crate::proc_vtable::ProcVTable;
use std::task::RawWakerVTable;

pub struct AlignProc {
    pub(crate) pdata: *const ProcData,

    pub(crate) schedule: *const u8,

    pub(crate) stack: *mut u8,

    pub(crate) future: *mut u8,

    pub(crate) output: *mut u8,
}

impl AlignProc {
    #[inline]
    pub(crate) fn from_layout(ptr: *const (), proc_layout: &ProcLayout) -> Self {
        let p = ptr as *const u8;

        unsafe {
            Self {
                pdata: p as *const ProcData,
                schedule: p.add(
                    Self::get_offset(proc_layout, "schedule")
                ) as *const u8,
                stack: p.add(
                    Self::get_offset(proc_layout, "stack")
                ) as *mut u8,
                future: p.add(
                    Self::get_offset(proc_layout, "future")
                ) as *mut u8,
                output: p.add(
                    Self::get_offset(proc_layout, "output")
                ) as *mut u8,
            }
        }
    }

    #[inline]
    pub(crate) fn get_offset(proc_layout: &ProcLayout, offset_of: &str) -> usize {
        if let Some(offset) = proc_layout.offset_table.get(offset_of).cloned() {
            dbg!(offset_of);
            dbg!(offset);
            offset
        } else {
            dbg!("align_proc::not_found", offset_of);
            0x00_usize
        }
    }
}
