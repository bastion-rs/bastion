// Because if we can't trust, we can't make.
#![forbid(unsafe_code)]
#![feature(const_fn)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(clamp)]
#![feature(panic_info_message)]

#[macro_use]
extern crate log;
extern crate env_logger;

// The Nether
mod runtime_manager;
mod spawn;
mod tramp;
mod runtime_system;

// The Overworld
pub mod bastion;
pub mod config;

pub mod child;
pub mod context;
pub mod receive;
pub mod supervisor;
pub mod messages;

pub mod macros;

pub mod prelude {
    // Runtime itself
    pub use crate::bastion::Bastion;
    pub use crate::config::*;

    // Primitives
    pub use crate::child::*;
    pub use crate::context::*;
    pub use crate::receive::*;
    pub use crate::supervisor::*;
    pub use crate::messages::*;

    pub use crate::macros::*;

    // Exported macros
    pub use crate::receive;
}
