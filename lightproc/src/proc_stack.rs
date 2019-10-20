use std::fmt;
use std::fmt::{Error, Formatter};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub struct ProcStack {
    pub pid: AtomicUsize,

    // Before action callbacks
    pub before_start: Option<Arc<dyn Fn() + Send + Sync>>,

    // After action callbacks
    pub after_complete: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl fmt::Debug for ProcStack {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.debug_struct("ProcStack")
            .field("pid", &self.pid.load(Ordering::SeqCst))
            .finish()
    }
}
