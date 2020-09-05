use crate::actor::traits::Actor;
use crate::routing::path::ActorPath;
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

/// A structure that holds configuration of the Bastion actor.
#[derive(Clone)]
pub struct Definition {
    /// The certain implementation of the Bastion actor that
    /// needs to be spawned.
    implementation: Arc<Box<dyn Actor>>,
    /// Defines actors that must be spawned in the hierarchy
    /// in the beginning of the actor's lifetime. The further
    /// amount of children may vary in runtime a won't be
    /// adjusted to the initial definition.
    children: Vec<Definition>,
    /// The path to the actor in the node.
    path: ActorPath,
}

impl Definition {
    /// Returns a new Definition instance.
    pub fn new(implementation: impl Actor + 'static) -> Self {
        let children = Vec::new();
        let path = ActorPath::default();

        Definition {
            implementation: Arc::new(Box::new(implementation)),
            children,
            path,
        }
    }

    /// Adds a single definition to the children list.
    pub fn with_parent_for(mut self, definition: Definition) -> Self {
        self.children.push(definition);
        self
    }

    /// Overrides the path on the user defined.
    pub fn with_path(mut self, path: ActorPath) -> Self {
        self.path = path;
        self
    }
}

impl Debug for Definition {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("Definition")
            .field("children", &self.children)
            .field("path", &self.path)
            .finish()
    }
}

// TODO: Add tests
