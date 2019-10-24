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

    pub after_panic: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl ProcStack {
    pub fn with_pid(mut self, pid: usize) -> Self {
        self.pid = AtomicUsize::new(pid);
        self
    }

    pub fn with_before_start<T>(mut self, callback: T) -> Self
    where
        T: Fn() + Send + Sync + 'static,
    {
        self.before_start = Some(Arc::new(callback));
        self
    }

    pub fn with_after_complete<T>(mut self, callback: T) -> Self
    where
        T: Fn() + Send + Sync + 'static,
    {
        self.after_complete = Some(Arc::new(callback));
        self
    }

    pub fn with_after_panic<T>(mut self, callback: T) -> Self
    where
        T: Fn() + Send + Sync + 'static,
    {
        self.after_panic = Some(Arc::new(callback));
        self
    }
}

impl fmt::Debug for ProcStack {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.debug_struct("ProcStack")
            .field("pid", &self.pid.load(Ordering::SeqCst))
            .finish()
    }
}
