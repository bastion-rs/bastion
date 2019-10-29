pub use self::bastion::Bastion;

mod bastion;
mod broadcast;
mod context;
mod macros;
mod proc;
mod system;

pub mod children;
pub mod supervisor;

pub mod prelude {
    pub use crate::children::{ChildRef, ChildrenRef, Closure, Fut, Message, Msg, Shell};
    pub use crate::context::BastionContext;
    pub use crate::message;
    pub use crate::supervisor::{SupervisionStrategy, Supervisor, SupervisorRef};
    pub use crate::Bastion;
}
