use once_cell::sync::Lazy;

use crate::system::node::Node;

pub mod definition;
mod global_state;
mod node;

static SYSTEM: Lazy<Node> = Lazy::new(|| Node::new());
