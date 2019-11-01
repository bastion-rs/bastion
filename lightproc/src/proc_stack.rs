use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub struct ProcStack {
    pub pid: AtomicUsize,

    // Before action callback
    pub(crate) before_start: Option<Arc<dyn Fn() + Send + Sync>>,

    // After action callback
    pub(crate) after_complete: Option<Arc<dyn Fn() + Send + Sync>>,

    // After panic callback
    pub(crate) after_panic: Option<Arc<dyn Fn() + Send + Sync>>,
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

impl Debug for ProcStack {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("ProcStack")
            .field("pid", &self.pid.load(Ordering::SeqCst))
            .finish()
    }
}
