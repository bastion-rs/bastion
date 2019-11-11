//! # Bastion: Fault-tolerant Runtime for Rust applications
//! [![Bastion](https://raw.githubusercontent.com/bastion-rs/bastion/master/img/bastion.png)](https://github.com/bastion-rs/bastion)
//!
//!
//!
//! Bastion is a highly-available, fault-tolerant runtime system
//! with dynamic dispatch oriented lightweight process model.
//! It supplies actor model like concurrency with primitives
//! called [lightproc] and utilize all the system resources
//! efficiently with at-most-once message delivery guarantee.
//!
//! To have a quick start please head to: [Bastion System Documentation](struct.Bastion.html).
//!
//! ## Features
//! * Message-based communication makes this project a lean mesh of actor system.
//!     * without web servers, weird shenanigans, forced trait implementations, and static dispatch.
//! * Runtime fault-tolerance makes it a good candidate for distributed systems.
//!     * If you want to smell of Erlang and it's powerful aspects in Rust. That's it!
//! * Completely asynchronous runtime with NUMA-aware and cache-affine SMP executor.
//!     * Exploiting hardware locality wherever it is possible. It is designed for servers.
//! * Supervision system makes it easy to manage lifecycles.
//!     * Kill your application in certain condition or restart you subprocesses whenever a certain condition met.
//!
//! ## Guarantees
//! * At most once delivery for all the messages.
//! * Completely asynchronous system design.
//! * Asynchronous program boundaries with [fort].
//! * Dynamic supervision of supervisors (adding a subtree later during the execution)
//! * Lifecycle management both at `futures` and `lightproc` layers.
//! * Faster middleware development.
//! * Above all "fault-tolerance".
//!
//! ## Why Bastion?
//! If one of the questions below answered with yes, then Bastion is just for you:
//! * Do I need fault-tolerance in my project?
//! * Do I need to write resilient middleware/s?
//! * I shouldn't need a webserver to run an actor system, right?
//! * Do I want to make my existing code unbreakable?
//! * Do I need an executor which is using system resources efficiently?
//! * Do I have some trust issues against orchestration systems? Because I want to implement my own application lifecycle.
//!
//!
//! [lightproc]: https://docs.rs/lightproc/
//! [fort]: https://docs.rs/fort/
//!

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/bastion-rs/bastion/master/img/bastion-logo.png"
)]
// Force missing implementations
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

pub use self::bastion::Bastion;
pub use self::callbacks::Callbacks;
pub use self::context::BastionContext;

mod bastion;
mod broadcast;
mod callbacks;
mod context;
mod system;

pub mod children;
pub mod message;
pub mod supervisor;

///
/// Prelude of Bastion
pub mod prelude {
    pub use crate::bastion::Bastion;
    pub use crate::callbacks::Callbacks;
    pub use crate::children::{ChildRef, Children, ChildrenRef};
    pub use crate::context::BastionContext;
    pub use crate::message::{Answer, Message, Msg, Sender};
    pub use crate::msg;
    pub use crate::supervisor::{SupervisionStrategy, Supervisor, SupervisorRef};
}
