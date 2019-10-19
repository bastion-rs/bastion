pub use self::bastion::Bastion;

mod bastion;
mod broadcast;
mod registry;
mod system;

pub mod children;
pub mod context;
pub mod supervisor;

pub mod prelude {
    pub use crate::Bastion;

    pub use crate::children::*;
    pub use crate::context::{BastionContext, BastionId};
    pub use crate::supervisor::{SupervisionStrategy, Supervisor};
}
