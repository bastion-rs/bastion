//! Stack abstraction for lightweight processes
//!
//! This abstraction allows us to execute lifecycle callbacks when
//! a process transites from one state to another.
//!
//! If we want to make an analogy, stack abstraction is similar to actor lifecycle abstractions
//! in frameworks like Akka, but tailored version for Rust environment.
use std::any::Any;
use std::cell::RefCell;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Stack abstraction for lightweight processes
///
/// # Example
///
/// ```rust
/// use lightproc::proc_stack::{ProcStack, EmptyProcState};
///
/// ProcStack::default()
///     .with_before_start(|s: EmptyProcState| { println!("Before start"); s })
///     .with_after_complete(|s: EmptyProcState| { println!("After complete"); s })
///     .with_after_panic(|s: EmptyProcState| { println!("After panic"); s });
/// ```
pub struct ProcStack {
    /// Process ID for the Lightweight Process
    ///
    /// Can be used to identify specific processes during any executor, reactor implementations.
    pub pid: AtomicUsize,

    pub(crate) state: Rc<RefCell<dyn State>>,

    /// Before start callback
    ///
    /// This callback is called before we start to inner future of the process
    pub(crate) before_start: Option<Arc<dyn Fn(Rc<RefCell<dyn State>>) + Send + Sync>>,

    /// After complete callback
    ///
    /// This callback is called after future resolved to it's output.
    /// Mind that, even panic occurs this callback will get executed.
    ///
    /// Eventually all panics are coming from an Error output.
    pub(crate) after_complete: Option<Arc<dyn Fn(Rc<RefCell<dyn State>>) + Send + Sync>>,

    /// After panic callback
    ///
    /// This callback is only called when a panic has been occurred.
    /// Mind that [ProcHandle](proc_handle/struct.ProcHandle.html) is not using this
    pub(crate) after_panic: Option<Arc<dyn Fn(Rc<RefCell<dyn State>>) + Send + Sync>>,
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

    pub fn with_state<S>(mut self, state: S) -> Self
    where
        S: State + 'static,
    {
        self.state = Rc::new(RefCell::new(state));
        self
    }

    /// Adds a callback that will be executed before polling inner future to the stack
    ///
    /// ```rust
    /// use lightproc::proc_stack::{ProcStack, EmptyProcState};
    ///
    /// ProcStack::default()
    ///     .with_before_start(|s: EmptyProcState| { println!("Before start"); s });
    /// ```
    pub fn with_before_start<C, S>(mut self, callback: C) -> Self
    where
        S: State,
        C: Fn(&mut S) + Send + Sync + 'static,
    {
        self.before_start = Some(Self::wrap_callback(callback));
        self
    }

    /// Adds a callback that will be executed after inner future resolves to an output to the stack
    ///
    /// ```rust
    /// use lightproc::proc_stack::{ProcStack, EmptyProcState};
    ///
    /// ProcStack::default()
    ///     .with_after_complete(|s: EmptyProcState| { println!("After complete"); s });
    /// ```
    pub fn with_after_complete<C, S>(mut self, callback: C) -> Self
    where
        S: State,
        C: Fn(&mut S) + Send + Sync + 'static,
    {
        self.after_complete = Some(Self::wrap_callback(callback));
        self
    }

    /// Adds a callback that will be executed after inner future panics to the stack
    ///
    /// ```rust
    /// use lightproc::proc_stack::{ProcStack, EmptyProcState};
    ///
    /// ProcStack::default()
    ///     .with_after_panic(|s: EmptyProcState| { println!("After panic"); s });
    /// ```
    pub fn with_after_panic<C, S>(mut self, callback: C) -> Self
    where
        S: State,
        C: Fn(&mut S) + Send + Sync + 'static,
    {
        self.after_panic = Some(Self::wrap_callback(callback));
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

    fn wrap_callback<C, S>(callback: C) -> Arc<dyn Fn(Rc<RefCell<dyn State>>) + Send + Sync>
    where
        S: State,
        C: Fn(&mut S) + Send + Sync + 'static,
    {
        Arc::new(move |state: Rc<RefCell<dyn State>>| {
            let mut state = state.borrow_mut();
            let state = state
                .as_any()
                .downcast_mut::<S>()
                .expect("Wrong State type.");
            callback(state);
        })
    }
}

trait AsAny {
    fn as_any(&mut self) -> &mut dyn Any;
}
impl<T: Any> AsAny for T {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

pub trait State: Any + AsAny {}

pub type EmptyProcState = Box<Empty>;
pub struct Empty;
impl State for Empty {}

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
            state: self.state.clone(),
            before_start: self.before_start.clone(),
            after_complete: self.after_complete.clone(),
            after_panic: self.after_panic.clone(),
        }
    }
}
