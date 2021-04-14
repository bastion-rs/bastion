mod global_state;
mod node;

use crate::system::node::Node;
use once_cell::sync::Lazy;

pub static SYSTEM: Lazy<Node> = Lazy::new(Node::new);
