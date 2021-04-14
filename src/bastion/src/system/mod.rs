mod global_state;
mod node;

use once_cell::sync::Lazy;

use crate::system::node::Node;

pub static SYSTEM: Lazy<Node> = Lazy::new(Node::new);
