//! # Bastion: Fault-tolerant Runtime for Rust applications
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
//!     * Without web servers, weird shenanigans, forced trait implementations, and static dispatch.
//! * Runtime fault-tolerance makes it a good candidate for distributed systems.
//!     * If you want the smell of Erlang and the powerful aspects of Rust. That's it!
//! * Completely asynchronous runtime with NUMA-aware and cache-affine SMP executor.
//!     * Exploiting hardware locality wherever it is possible. It is designed for servers.
//! * Supervision system makes it easy to manage lifecycles.
//!     * Kill your application in certain condition or restart you subprocesses whenever a certain condition is met.
//! * Automatic member discovery, cluster formation and custom message passing between cluster members.
//!     * Using zeroconf or not, launch your bastion cluster from everywhere, with a single actor block.
//! * Proactive IO system which doesn't depend on anything other than `futures`.
//!     * Bastion's proactive IO has scatter/gather operations, `io_uring` support and much more...
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
//! If one of the questions below is answered with yes, then Bastion is just for you:
//! * Do I want proactive IO?
//! * Do I need fault-tolerance in my project?
//! * Do I need to write resilient middleware/s?
//! * I shouldn't need a webserver to run an actor system, right?
//! * Do I want to make my existing code unbreakable?
//! * Do I need an executor which is using system resources efficiently?
//! * Do I have some trust issues with orchestration systems?
//! * Do I want to implement my own application lifecycle?
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
// Deny using unsafe code
#![deny(unsafe_code)]
// Doc generation experimental features
#![cfg_attr(feature = "docs", feature(doc_cfg))]

pub use self::bastion::Bastion;
pub use self::callbacks::Callbacks;
pub use self::config::Config;

#[macro_use]
mod macros;

mod actor;
mod bastion;
mod broadcast;
mod callbacks;
mod child;
mod config;
mod global_system;

pub mod child_ref;
pub mod children;
pub mod children_ref;
pub mod context;
pub mod dispatcher;
pub mod envelope;
pub mod executor;
#[cfg(not(target_os = "windows"))]
pub mod io;
mod mailbox;
pub mod message;
pub mod path;
#[cfg(feature = "scaling")]
pub mod resizer;
mod routing;
pub mod supervisor;

pub mod errors;

pub mod distributor;

distributed_api! {
    // pub mod dist_messages;
    pub mod distributed;
}

///
/// Prelude of Bastion
pub mod prelude {
    pub use crate::bastion::Bastion;
    pub use crate::callbacks::Callbacks;
    pub use crate::child_ref::ChildRef;
    pub use crate::children::Children;
    pub use crate::children_ref::ChildrenRef;
    pub use crate::config::Config;
    pub use crate::context::{BastionContext, BastionId, NIL_ID};
    pub use crate::dispatcher::{
        BroadcastTarget, DefaultDispatcherHandler, Dispatcher, DispatcherHandler, DispatcherMap,
        DispatcherType, NotificationType,
    };
    pub use crate::distributor::Distributor;
    pub use crate::envelope::{RefAddr, SignedMessage};
    pub use crate::errors::*;
    #[cfg(not(target_os = "windows"))]
    pub use crate::io::*;
    pub use crate::message::{Answer, AnswerSender, Message, MessageHandler, Msg};
    pub use crate::msg;
    pub use crate::path::{BastionPath, BastionPathElement};
    #[cfg(feature = "scaling")]
    pub use crate::resizer::{OptimalSizeExploringResizer, UpperBound, UpscaleStrategy};
    pub use crate::supervisor::{
        ActorRestartStrategy, RestartPolicy, RestartStrategy, SupervisionStrategy, Supervisor,
        SupervisorRef,
    };
    pub use crate::{answer, blocking, children, run, spawn, supervisor};

    distributed_api! {
        // pub use crate::dist_messages::*;
        pub use crate::distributed::*;
        pub use artillery_core::cluster::ap::*;
        pub use artillery_core::epidemic::prelude::*;
    }
}
