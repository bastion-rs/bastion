#[macro_use]
extern crate log;
extern crate env_logger;

// The Nether
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
