//! Stack abstraction for lightweight processes
//!
//! This abstraction allows us to execute lifecycle callbacks when
//! a process transites from one state to another.
//!
//! If we want to make an analogy, stack abstraction is similar to actor lifecycle abstractions
//! in frameworks like Akka, but tailored version for Rust environment.
use super::proc_state::*;

use std::fmt::{self, Debug, Formatter};

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Stack abstraction for lightweight processes
///
/// # Example
///
/// ```rust
/// use lightproc::proc_stack::ProcStack;
/// use lightproc::proc_state::EmptyProcState;
///
/// ProcStack::default()
///     .with_before_start(|s: &mut EmptyProcState| { println!("Before start"); })
///     .with_after_complete(|s: &mut EmptyProcState| { println!("After complete"); })
///     .with_after_panic(|s: &mut EmptyProcState| { println!("After panic"); });
/// ```
pub struct ProcStack {
    /// Process ID for the Lightweight Process
    ///
    /// Can be used to identify specific processes during any executor, reactor implementations.
    pub pid: AtomicUsize,

    pub(crate) state: ProcState,

    /// Before start callback
    ///
    /// This callback is called before we start to inner future of the process
    pub(crate) before_start: Option<Arc<dyn Fn(ProcState) + Send + Sync>>,

    /// After complete callback
    ///
    /// This callback is called after future resolved to it's output.
    /// Mind that, even panic occurs this callback will get executed.
    ///
    /// Eventually all panics are coming from an Error output.
    pub(crate) after_complete: Option<Arc<dyn Fn(ProcState) + Send + Sync>>,

    /// After panic callback
    ///
    /// This callback is only called when a panic has been occurred.
    /// Mind that [ProcHandle](proc_handle/struct.ProcHandle.html) is not using this
    pub(crate) after_panic: Option<Arc<dyn Fn(ProcState) + Send + Sync>>,
}

impl ProcStack {
    /// Adds pid for the process which is going to take this stack
    ///
    /// # Example
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

    /// Adds state for the process which is going to be embedded into this stack.
    ///
    /// # Example
    ///
    /// ```rust
    /// use lightproc::proc_stack::ProcStack;
    ///
    /// pub struct GlobalState {
    ///    pub amount: usize
    /// }
    ///
    /// ProcStack::default()
    ///     .with_pid(1)
    ///     .with_state(GlobalState { amount: 1 });
    /// ```
    pub fn with_state<S>(mut self, state: S) -> Self
    where
        S: State + 'static,
    {
        self.state = Arc::new(Mutex::new(state));
        self
    }

    /// Adds a callback that will be executed before polling inner future to the stack
    ///
    /// ```rust
    /// use lightproc::proc_stack::{ProcStack};
    /// use lightproc::proc_state::EmptyProcState;
    ///
    /// ProcStack::default()
    ///     .with_before_start(|s: &mut EmptyProcState| { println!("Before start"); });
    /// ```
    pub fn with_before_start<C, S>(mut self, callback: C) -> Self
    where
        S: State,
        C: Fn(&mut S) + Send + Sync + 'static,
    {
        self.before_start = Some(self.wrap_callback(callback));
        self
    }

    /// Adds a callback that will be executed after inner future resolves to an output to the stack
    ///
    /// ```rust
    /// use lightproc::proc_stack::ProcStack;
    /// use lightproc::proc_state::EmptyProcState;
    ///
    /// ProcStack::default()
    ///     .with_after_complete(|s: &mut EmptyProcState| { println!("After complete"); });
    /// ```
    pub fn with_after_complete<C, S>(mut self, callback: C) -> Self
    where
        S: State,
        C: Fn(&mut S) + Send + Sync + 'static,
    {
        self.after_complete = Some(self.wrap_callback(callback));
        self
    }

    /// Adds a callback that will be executed after inner future panics to the stack
    ///
    /// ```rust
    /// use lightproc::proc_stack::ProcStack;
    /// use lightproc::proc_state::EmptyProcState;
    ///
    /// ProcStack::default()
    ///     .with_after_panic(|s: &mut EmptyProcState| { println!("After panic"); });
    /// ```
    pub fn with_after_panic<C, S>(mut self, callback: C) -> Self
    where
        S: State,
        C: Fn(&mut S) + Send + Sync + 'static,
    {
        self.after_panic = Some(self.wrap_callback(callback));
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

    /// Get the state which is embedded into this [ProcStack].
    ///
    /// ```rust
    /// use lightproc::proc_stack::ProcStack;
    ///
    /// #[derive(Copy, Clone)]
    /// pub struct GlobalState {
    ///    pub amount: usize
    /// }
    ///
    /// let mut proc = ProcStack::default().with_pid(123)
    ///             .with_state(GlobalState { amount: 0} );
    ///
    /// let state = proc.get_state::<GlobalState>();
    /// ```
    pub fn get_state<S>(&self) -> S
    where
        S: State + Copy + 'static,
    {
        let state = self.state.clone();
        let s = unsafe { &*(&state as *const ProcState as *const Arc<Mutex<S>>) };
        *s.lock().unwrap()
    }

    /// Wraps the callback to the with given trait boundaries of the state.
    ///
    /// Why there is unsafe?
    /// * Given state is taken as dyn State we don't know the size. But rest assured it is sized.
    /// * Executor of the callback should know what exactly coming up from state. That said it can even pass dynamically sized trait too, we can't constrain that.
    /// * Cast it to the correct type and trust the boundaries which was already ensured at the method signature.
    /// * Synchronization can't revolve around the unsafe. because we lock the dynamic state right after the cast.
    fn wrap_callback<C, S>(&self, callback: C) -> Arc<dyn Fn(ProcState) + Send + Sync>
    where
        S: State + 'static,
        C: Fn(&mut S) + Send + Sync + 'static,
    {
        let wrapped = move |s: ProcState| {
            let x = unsafe { &*(&s as *const ProcState as *const Arc<Mutex<S>>) };
            let mut mg = x.lock().unwrap();
            callback(&mut *mg);
        };
        Arc::new(wrapped)
    }
}

///
/// Default implementation for the ProcStack
impl Default for ProcStack {
    fn default() -> Self {
        ProcStack {
            pid: AtomicUsize::new(0xDEAD_BEEF),
            state: Arc::new(Mutex::new(EmptyState)),
            before_start: None,
            after_complete: None,
            after_panic: None,
        }
    }
}

impl Debug for ProcStack {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("ProcStack")
            .field("pid", &self.pid.load(Ordering::SeqCst))
            .field("state", &self.state)
            .field("before_start", &self.before_start.is_some())
            .field("after_complete", &self.after_complete.is_some())
            .field("after_panic", &self.after_panic.is_some())
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
