//! # Bastion: Fault-tolerant Runtime for Rust applications
//! [![Bastion](https://raw.githubusercontent.com/bastion-rs/bastion/master/img/bastion.png)](https://github.com/bastion-rs/bastion)
//!
//!
//!
//! Bastion is a fault-tolerant runtime which is designed for recovering from
//! faults based on the supervision strategies that you've passed.
//! It is designed to provide persistent runtime for applications which need
//! to be highly-available.
//!
//! ## Why Bastion?
//! If one of the questions below answered with yes, then Bastion is just for you:
//! * Do I need fault-tolerancy in my project?
//! * Do I hate to implement weird Actor traits?
//! * I shouldn't need a webserver to run an actor system, right?
//! * Do I want to make my existing code unbreakable?
//! * Do I have some trust issues against orchestration systems? Because I want to implement my own application lifecycle.
//!
//! ## Features
//! * Message-based communication makes this project a lean mesh of actor system.
//!     * without web servers, weird shenanigans, forced trait implementations, and static dispatch.
//! * Runtime fault-tolerance makes it a good candidate for small scale distributed system code.
//!     * If you want to smell of Erlang and it's powerful aspects in Rust. That's it!
//! * Supervision makes it easy to manage lifecycles.
//!     * Kill your application in certain condition or restart you subprocesses whenever a certain condition met.
//! All up to you. And it should be up to you.
//!

#![doc(html_logo_url = "https://raw.githubusercontent.com/bastion-rs/bastion/master/img/bastion-logo.png")]

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
    //!
    //! Prelude for bare minimum runtime dependencies

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
