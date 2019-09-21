//! # Bastion: Fault-tolerant Runtime for Rust applications
//! [![Bastion](https://raw.githubusercontent.com/vertexclique/bastion/master/img/bastion.png)](https://github.com/vertexclique/bastion)
//!
//!
//!
//! Bastion is a fault-tolerant runtime which is designed for recovering from
//! faults based on the supervision strategies that you've passed.
//! It is designed to provide persistent runtime for applications which need
//! to be highly-available.
//!
//!

#[macro_use]
extern crate log;
extern crate env_logger;

// The Nether
// Modules which are not exposed to the user.
mod runtime_manager;
mod runtime_system;
mod spawn;
mod tramp;

// The Overworld
pub mod bastion;
pub mod config;

pub mod child;
pub mod context;
pub mod messages;
pub mod receive;
pub mod supervisor;

pub mod macros;

pub mod prelude {
    // Runtime itself
    pub use crate::bastion::Bastion;
    pub use crate::config::*;

    // Primitives
    pub use crate::child::*;
    pub use crate::context::*;
    pub use crate::messages::*;
    pub use crate::receive::*;
    pub use crate::supervisor::*;

    pub use crate::macros::*;

    // Exported macros
    pub use crate::receive;
}
