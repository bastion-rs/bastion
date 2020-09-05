use crate::actor::traits::Actor;
use crate::routing::path::ActorPath;

/// A structure that holds configuration of the Bastion actor.
#[derive(Debug, Clone)]
pub struct Definition {
    /// The certain implementation of the Bastion actor that
    /// needs to be spawned.
    implementation: Box<dyn Actor>,
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
    pub fn new(implementation: impl Actor) -> Self {
        let children = Vec::new();
        let path = ActorPath::default();

        Definition {
            implementation: Box::new(implementation),
            children,
            path,
        }
    }

    /// Append a single actor definition to the children list.
    pub fn with_actor(mut self, definition: Definition) -> self {
        self.children.push(definition);
        self
    }

    /// Adds a list of actors to the children list.
    pub fn with_actors(mut self, actors: Vec<Definition>) -> self {
        self.children.extend(actors);
        self
    }

    /// Overrides the path on the user defined.
    pub fn with_path(mut self, path: ActorPath) -> Self {
        self.path = path;
        self
    }
}
