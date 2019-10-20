use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::fmt::{Formatter, Error};
use std::fmt;

#[derive(Clone, Default)]
pub struct ProcStack {
    pub after_start: Option<Arc<dyn Fn() + Send + Sync>>,

    pub after_complete: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl fmt::Debug for ProcStack {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.debug_struct("ProcStack")
            .finish()
    }
}
