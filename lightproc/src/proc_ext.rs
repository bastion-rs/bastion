use std::future::Future;
use std::panic::UnwindSafe;
use crate::catch_unwind::CatchUnwind;

pub trait ProcFutureExt: Future {
    fn catch_unwind(self) -> CatchUnwind<Self>
        where Self: Sized + UnwindSafe
    {
        CatchUnwind::new(self)
    }
}

impl<T: ?Sized> ProcFutureExt for T where T: Future {}
