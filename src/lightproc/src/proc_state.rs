//!
//! State layer for lightproc implementation
//!
//! Contains trait bounds and state wrapping code
//!
//! # Example
//! ```rust
//! use lightproc::proc_state::State;
//! use crate::lightproc::proc_state::AsAny;
//!
//! #[derive(Clone)]
//! pub struct SharedState {
//!     name: String,
//!     surname: String,
//!     id: u64,
//! }
//!
//!
//! let mut s = SharedState {
//!     name: "Riemann".to_string(),
//!     surname: "Sum".to_string(),
//!     id: 123
//! };
//!
//! s.as_any();
//! ```

use std::any::Any;

use std::fmt::{Error, Formatter};

use std::fmt;
use std::sync::{Arc, Mutex};

///
/// State trait to implement a state for Bastion
pub trait State: Send + Sync + AsAny + 'static {}

///
/// Blanket implementation for the state when rules apply
impl<T> State for T where T: Send + Sync + 'static {}

///
/// Default debug implementation for dynamically dispatched state
impl fmt::Debug for dyn State {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_fmt(format_args!("State :: {:?}", self.type_id()))
    }
}

///
/// Generic protection type where state is stored as is.
/// This allows us to share the state between the threads (both by ref and by value).
/// All state implementors bound to use this.
pub type ProcState = Arc<Mutex<dyn State>>;

///
/// Generic dynamic programming construct which allows us to downcast to the typeless level.
/// And do costly conversion between types if possible.
pub trait AsAny {
    ///
    /// Downcast implemented type to Any.
    fn as_any(&mut self) -> &mut dyn Any;
}

///
/// Blanket implementation for Any conversion if type is implementing Any.
impl<T: Any> AsAny for T {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

// ------------------------------------------------------
// State related types are below
// ------------------------------------------------------

///
/// Empty proc state which is an heap allocated empty struct.
pub type EmptyProcState = Box<EmptyState>;

///
/// Base construct for empty state struct.
#[derive(Debug)]
pub struct EmptyState;
