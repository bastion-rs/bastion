use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use uuid::Uuid;

use crate::actor::traits::Actor;
use crate::routing::path::{ActorPath, Scope};

type CustomActorNameFn = dyn Fn() -> String + Send + 'static;

/// A structure that holds configuration of the Bastion actor.
pub struct Definition {
    /// A unique definition name;
    name: String,
    /// A struct that implements actor's behaviour
    actor: Option<Arc<dyn Actor>>,
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
        let name = Uuid::new_v4().to_string();
        let scope = Scope::User;
        let actor_name_fn = None;
        let redundancy = 1;
        let actor = None;

        Definition {
            name,
            actor,
            scope,
            actor_name_fn,
            redundancy,
        }
    }

    /// Overrides the definition's name. The passed value must be unique.
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Sets the actor to schedule.
    pub fn actor<T: 'static>(mut self, actor: T) -> Self
    where
        T: Actor,
    {
        self.actor = Some(Arc::new(actor));
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

impl Eq for Definition {}
impl PartialEq for Definition {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Hash for Definition {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use crate::actor::context::Context;
    use crate::actor::definition::Definition;
    use crate::actor::traits::Actor;
    use crate::error::Result;
    use crate::routing::path::Scope;

    use async_trait::async_trait;

    #[derive(Debug)]
    struct FakeActor {
        counter: u32,
    }

    #[async_trait]
    impl Actor for FakeActor {
        async fn handler(&mut self, ctx: &mut Context) -> Result<()> {
            Ok(())
        }
    }

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
    fn test_definition_with_custom_actor() {
        let instance = Definition::new().actor(FakeActor { counter: 0 });

        assert_eq!(instance.actor.is_some(), true);
        assert_eq!(instance.scope, Scope::User);
        assert_eq!(instance.actor_name_fn.is_none(), true);
        assert_eq!(instance.redundancy, 1);
    }

    #[test]
    fn test_definition_with_custom_name() {
        let instance = Definition::new().name("test_name");

        assert_eq!(instance.name, "test_name");
        assert_eq!(instance.scope, Scope::User);
        assert_eq!(instance.actor_name_fn.is_some(), false);
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
