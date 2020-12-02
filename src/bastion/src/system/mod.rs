mod global_state;
mod node;

use lazy_static::lazy_static;

use crate::system::node::Node;

lazy_static! {
    pub static ref SYSTEM: Node = Node::new();
}
