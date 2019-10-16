use crate::lightproc::LightProc;
use crate::proc_data::ProcData;
use crate::proc_layout::ProcLayout;
use std::future::Future;


/// Raw pointers to the fields of a task.
pub struct RawProc<F, R, S, T> {
    pub(crate) pdata: *const ProcData,

    pub(crate) schedule: *const S,

    pub(crate) stack: *mut T,

    pub(crate) future: *mut F,

    pub(crate) output: *mut R,
}

impl<F, R, S, T> Copy for RawProc<F, R, S, T> {}

impl<F, R, S, T> Clone for RawProc<F, R, S, T> {
    fn clone(&self) -> Self {
        Self {
            pdata: self.pdata,
            schedule: self.schedule,
            stack: self.stack,
            future: self.future,
            output: self.output,
        }
    }
}

impl<F, R, S, T> RawProc<F, R, S, T>
{
    #[inline]
    pub(crate) fn from_ptr(ptr: *const (), proc_layout: &ProcLayout) -> Self {
        let p = ptr as *const u8;

        unsafe {
            Self {
                pdata: p as *const ProcData,
                schedule: p.add(
                    Self::get_offset(proc_layout, "schedule")
                ) as *const S,
                stack: p.add(
                    Self::get_offset(proc_layout, "stack")
                ) as *mut T,
                future: p.add(
                    Self::get_offset(proc_layout, "future")
                ) as *mut F,
                output: p.add(
                    Self::get_offset(proc_layout, "output")
                ) as *mut R,
            }
        }
    }

    #[inline]
    pub(crate) fn get_offset(proc_layout: &ProcLayout, offset_of: &str) -> usize {
        if let Some(offset) = proc_layout.offset_table.get(offset_of).cloned() {
            dbg!(offset);
            offset
        } else {
            0x00_usize
        }
    }
}
