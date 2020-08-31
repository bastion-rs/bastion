use crate::routing::ActorPath;

/// An enum for handling targeting messages to the certain
/// actor, a group of actors, a namespace or a scope.
#[derive(Debug, Clone)]
pub enum Target {
    /// The message must be delivered to the actor(s) by
    /// the matched path.
    Path(ActorPath),
    /// The message must be delivered to actor(s), organized
    /// under the named group.
    Group(String),
}

/// A wrapper for the Target enum type that provides an additional
/// functionality in matching actor paths with the desired pattern.
pub(crate) struct EnvelopeTarget {
    target: Target,
    regex: Option<Regex>,
}

impl EnvelopeTarget {
    pub(crate) fn is_match(&self, actor_path: &ActorPath) -> bool {
        match &self.target {
            // Just do a regular matching by path, or ... via
            // the regular expression if the path contains asterisks.
            Target::Path(path) => true,
            // False because the group name is not a part of the actor's path
            Target::Group(_) => false,
        }
    }
}

// TODO: Add implementation
impl From<Target> for EnvelopeTarget {}

#[cfg(test)]
mod message_target_tests {}
