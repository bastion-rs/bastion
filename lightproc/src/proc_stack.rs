//! Stack abstraction for lightweight processes
//!
//! This abstraction allows us to execute lifecycle callbacks when
//! a process transites from one state to another.
//!
//! If we want to make an analogy, stack abstraction is similar to actor lifecycle abstractions
//! in frameworks like Akka, but tailored version for Rust environment.
use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Stack abstraction for lightweight processes
///
/// # Example
///
/// ```rust
/// use lightproc::proc_stack::ProcStack;
///
/// ProcStack::default()
///     .with_before_start(|| { println!("Before start"); })
///     .with_after_complete(|| { println!("After complete"); })
///     .with_after_panic(|| { println!("After panic"); });
/// ```
#[derive(Default)]
pub struct ProcStack {
    /// Process ID for the Lightweight Process
    ///
    /// Can be used to identify specific processes during any executor, reactor implementations.
    pub pid: AtomicUsize,

    /// Before start callback
    ///
    /// This callback is called before we start to inner future of the process
    pub(crate) before_start: Option<Arc<dyn Fn() + Send + Sync>>,

    /// After complete callback
    ///
    /// This callback is called after future resolved to it's output.
    /// Mind that, even panic occurs this callback will get executed.
    ///
    /// Eventually all panics are coming from an Error output.
    pub(crate) after_complete: Option<Arc<dyn Fn() + Send + Sync>>,

    /// After panic callback
    ///
    /// This callback is only called when a panic has been occurred.
    /// Mind that [ProcHandle](proc_handle/struct.ProcHandle.html) is not using this
    pub(crate) after_panic: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl ProcStack {
    /// Adds pid for the process which is going to take this stack
    ///
    /// ```rust
    /// use lightproc::proc_stack::ProcStack;
    ///
    /// ProcStack::default()
    ///     .with_pid(1);
    /// ```
    pub fn with_pid(mut self, pid: usize) -> Self {
        self.pid = AtomicUsize::new(pid);
        self
    }

    /// Adds a callback that will be executed before polling inner future to the stack
    ///
    /// ```rust
    /// use lightproc::proc_stack::ProcStack;
    ///
    /// ProcStack::default()
    ///     .with_before_start(|| { println!("Before start") });
    /// ```
    pub fn with_before_start<T>(mut self, callback: T) -> Self
    where
        T: Fn() + Send + Sync + 'static,
    {
        self.before_start = Some(Arc::new(callback));
        self
    }

    /// Adds a callback that will be executed after inner future resolves to an output to the stack
    ///
    /// ```rust
    /// use lightproc::proc_stack::ProcStack;
    ///
    /// ProcStack::default()
    ///     .with_after_complete(|| { println!("After complete") });
    /// ```
    pub fn with_after_complete<T>(mut self, callback: T) -> Self
    where
        T: Fn() + Send + Sync + 'static,
    {
        self.after_complete = Some(Arc::new(callback));
        self
    }

    /// Adds a callback that will be executed after inner future panics to the stack
    ///
    /// ```rust
    /// use lightproc::proc_stack::ProcStack;
    ///
    /// ProcStack::default()
    ///     .with_after_panic(|| { println!("After panic") });
    /// ```
    pub fn with_after_panic<T>(mut self, callback: T) -> Self
    where
        T: Fn() + Send + Sync + 'static,
    {
        self.after_panic = Some(Arc::new(callback));
        self
    }

    /// Utility function to get_pid for the implementation of executors.
    ///
    /// ```rust
    /// use lightproc::proc_stack::ProcStack;
    ///
    /// let proc = ProcStack::default().with_pid(123);
    ///
    /// assert_eq!(proc.get_pid(), 123);
    /// ```
    pub fn get_pid(&self) -> usize {
        self.pid.load(Ordering::Acquire)
    }
}

impl Debug for ProcStack {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("ProcStack")
            .field("pid", &self.pid.load(Ordering::SeqCst))
            .finish()
    }
}

impl Clone for ProcStack {
    fn clone(&self) -> Self {
        ProcStack {
            pid: AtomicUsize::new(self.pid.load(Ordering::Acquire)),
            before_start: self.before_start.clone(),
            after_complete: self.after_complete.clone(),
            after_panic: self.after_panic.clone(),
        }
    }
}
