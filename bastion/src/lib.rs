pub use self::bastion::Bastion;

mod bastion;
mod broadcast;
mod context;
mod proc;
mod system;

pub mod children;
pub mod supervisor;

pub mod prelude {
    pub use crate::Bastion;

    pub use crate::children::{ChildRef, ChildrenRef, Closure, Fut, Message, Shell};
    pub use crate::context::BastionContext;
    pub use crate::supervisor::{SupervisionStrategy, Supervisor, SupervisorRef};
}
