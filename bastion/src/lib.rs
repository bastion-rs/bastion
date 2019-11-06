// Force missing implementations
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

pub use self::bastion::Bastion;

mod bastion;
mod broadcast;
mod context;
mod system;

pub mod children;
pub mod message;
pub mod supervisor;

pub mod prelude {
    pub use crate::bastion::Bastion;
    pub use crate::children::{ChildRef, Children, ChildrenRef};
    pub use crate::context::BastionContext;
    pub use crate::message::{Answer, Message, Msg, Sender};
    pub use crate::msg;
    pub use crate::supervisor::{SupervisionStrategy, Supervisor, SupervisorRef};
}
