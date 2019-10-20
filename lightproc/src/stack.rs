use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::fmt::{Formatter, Error};
use std::fmt;

#[derive(Default)]
pub struct ProcStack {
    pub pid: AtomicUsize,

    pub after_start: Option<Arc<dyn Fn() + Send + Sync>>,

    pub after_complete: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl fmt::Debug for ProcStack {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.debug_struct("ProcStack")
            .field("pid", &self.pid.load(Ordering::SeqCst))
            .finish()
    }
}
