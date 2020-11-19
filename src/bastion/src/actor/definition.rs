use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use crate::actor::traits::Actor;
use crate::routing::path::{ActorPath, Scope};

type CustomActorNameFn = dyn Fn() -> String + Send + 'static;

/// A structure that holds configuration of the Bastion actor.
pub struct Definition {
    /// A struct that implements actor's behaviour
    actor: Option<Arc<Box<dyn Actor>>>,
    /// Defines a used scope for instantiating actors.
    scope: Scope,
    /// Defines a function used for generating unique actor names.
    actor_name_fn: Option<Arc<CustomActorNameFn>>,
    /// Defines how much actors must be instantiated in the beginning.
    redundancy: usize,
}

impl Definition {
    /// Returns a new instance of the actor's definition.
    pub fn new() -> Self {
        let scope = Scope::User;
        let actor_name_fn = None;
        let redundancy = 1;
        let actor = None;

        Definition {
            actor,
            scope,
            actor_name_fn,
            redundancy,
        }
    }

    /// Sets the actor to schedule.
    pub fn actor(mut self, actor: impl Actor) -> Self {
        self.actor = Some(Arc::new(Box::new(actor)));
        self
    }

    /// Overrides the default scope in which actors must be spawned
    pub fn scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    /// Overrides the default behaviour for generating actor names
    pub fn custom_actor_name<F>(mut self, func: F) -> Self
    where
        F: Fn() -> String + Send + 'static,
    {
        self.actor_name_fn = Some(Arc::new(func));
        self
    }

    /// Overrides the default values for redundancy
    pub fn redundancy(mut self, redundancy: usize) -> Self {
        self.redundancy = match redundancy == std::usize::MIN {
            true => redundancy.saturating_add(1),
            false => redundancy,
        };

        self
    }

    pub(crate) fn generate_actor_path(&self) -> ActorPath {
        match &self.actor_name_fn {
            Some(func) => {
                let custom_name = func();
                ActorPath::default()
                    .scope(self.scope.clone())
                    .name(&custom_name)
            }
            None => ActorPath::default().scope(self.scope.clone()),
        }
    }
}

impl Debug for Definition {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("Definition")
            .field("scope", &self.scope)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::actor::definition::Definition;
    use crate::routing::path::Scope;

    fn fake_actor_name() -> String {
        let index = 1;
        format!("Actor_{}", index)
    }

    #[test]
    fn test_default_definition() {
        let instance = Definition::new();

        assert_eq!(instance.scope, Scope::User);
        assert_eq!(instance.actor_name_fn.is_none(), true);
        assert_eq!(instance.redundancy, 1);
    }

    #[test]
    fn test_definition_with_custom_actor_name() {
        let instance = Definition::new().custom_actor_name(fake_actor_name);

        assert_eq!(instance.scope, Scope::User);
        assert_eq!(instance.actor_name_fn.is_some(), true);
        assert_eq!(instance.redundancy, 1);

        let actor_path = instance.generate_actor_path();
        assert_eq!(actor_path.to_string(), "bastion://node/user/Actor_1");
        assert_eq!(actor_path.is_local(), true);
        assert_eq!(actor_path.is_user_scope(), true);
    }

    #[test]
    fn test_definition_with_custom_actor_name_closure() {
        let instance = Definition::new().custom_actor_name(move || -> String {
            let index = 1;
            format!("Actor_{}", index)
        });

        assert_eq!(instance.scope, Scope::User);
        assert_eq!(instance.actor_name_fn.is_some(), true);
        assert_eq!(instance.redundancy, 1);

        let actor_path = instance.generate_actor_path();
        assert_eq!(actor_path.to_string(), "bastion://node/user/Actor_1");
        assert_eq!(actor_path.is_local(), true);
        assert_eq!(actor_path.is_user_scope(), true);
    }

    #[test]
    fn test_definition_with_custom_scope_and_actor_name_closure() {
        let instance =
            Definition::new()
                .scope(Scope::Temporary)
                .custom_actor_name(move || -> String {
                    let index = 1;
                    format!("Actor_{}", index)
                });

        assert_eq!(instance.scope, Scope::Temporary);
        assert_eq!(instance.actor_name_fn.is_some(), true);
        assert_eq!(instance.redundancy, 1);

        let actor_path = instance.generate_actor_path();
        assert_eq!(actor_path.to_string(), "bastion://node/temporary/Actor_1");
        assert_eq!(actor_path.is_temporary_scope(), true);
    }
}
