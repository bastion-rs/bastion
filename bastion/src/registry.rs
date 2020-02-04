//!
//! Special module that allows users to interact and communicate with a
//! group of actors through the Registry name.  
use crate::context::BastionId;
use crate::envelope::SignedMessage;
use dashmap::DashMap;
use std::fmt::{self, Debug};

/// Type alias for the concurrency hashmap. Each key-value pair stores
/// the Bastion identifier as the key and the module name as the value.
pub type BastionRegistryMap = DashMap<BastionId, String>;

/// Generic trait which any custom dispatcher must implement for
/// further usage by the `Registry`.
pub trait RegistryDispatcher {
    /// Method that handles logic with registering a new actor.
    fn dispatch(&self, entries: &BastionRegistryMap);
    /// Method that handles logic with processing incoming messages
    /// from the certain actor.
    fn broadcast_message(&self, entries: &BastionRegistryMap, message: SignedMessage);
}

#[derive(Debug, Clone)]
/// Defines the type of the registry.
///
/// The default type is `System`.
pub enum RegistryType {
    /// The default kind of the registry used for handling all
    /// actors in the cluster. The only one instance of this
    /// registry must exist of this type.
    System,
    /// The registry with a unique name which will be using
    /// for organizing actors in groups on the certain criterias.
    /// The logic for organizing and registering actors in the
    /// group depends on the dispatcher implementation.
    Named(String),
}

#[derive(Debug)]
/// The special kind of dispatchers, created by default for the
/// new registries. Initially doesn't do any useful processing.
pub struct DefaultRegistryDispatcher;

/// A generic implementation of the Bastion registry
///
/// The main idea of the registry is to provide an alternative way to
/// communicate between a group of actors. For example, registries can
/// be used when a developer wants to send a specific message or share a
/// local state between the specific group of registered actors with
/// the usage of a custom dispatcher.
pub struct Registry {
    /// Defines the type of the used registry.
    registry_type: RegistryType,
    /// The dispatcher used for each `register` call.
    dispatcher: Box<dyn RegistryDispatcher>,
    /// Stores information about all registered actors in
    /// the group.
    storage: BastionRegistryMap,
}

impl Registry {
    /// Returns the type of the registry.
    pub fn registry_type(&self) -> RegistryType {
        self.registry_type.clone()
    }

    /// Returns the used dispatcher by the registry.
    pub fn dispatcher(&self) -> &Box<dyn RegistryDispatcher> {
        &self.dispatcher
    }

    /// Sets the registry type.
    pub fn with_registry_type(mut self, registry_type: RegistryType) -> Self {
        trace!("Setting registry the {:?} name.", self.registry_type);
        self.registry_type = registry_type;
        self
    }

    /// Sets the used dispatcher for the registry.
    pub fn with_dispatcher(mut self, dispatcher: Box<dyn RegistryDispatcher>) -> Self {
        trace!(
            "Setting dispatcher for the {:?} registry.",
            self.registry_type
        );
        self.dispatcher = dispatcher;
        self
    }

    /// Appends the information about actor to the registry.
    pub fn register(&self, key: BastionId, module_name: String) {
        self.storage.insert(key, module_name);
        self.dispatcher.dispatch(&self.storage);
    }

    /// Removes and then returns the record from the registry by the given key.
    /// Returns `None` when the record wasn't found by the given key.
    pub fn remove(&self, key: BastionId) -> Option<(BastionId, String)> {
        self.storage.remove(&key)
    }

    /// Sends the message to the registered group of actors.
    /// The logic of who and how should receive the message relies onto
    /// the dispatcher's implementation.
    pub fn broadcast_message(&self, message: SignedMessage) {
        self.dispatcher.broadcast_message(&self.storage, message);
    }
}

impl Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<Registry(type: {:?}, actors: {:?})>",
            self.registry_type,
            self.storage.len()
        )
    }
}

impl RegistryDispatcher for DefaultRegistryDispatcher {
    fn dispatch(&self, _entries: &BastionRegistryMap) {}
    fn broadcast_message(&self, _entries: &BastionRegistryMap, _message: SignedMessage) {}
}

impl Default for Registry {
    fn default() -> Self {
        Registry {
            registry_type: RegistryType::default(),
            dispatcher: Box::new(DefaultRegistryDispatcher::default()),
            storage: DashMap::new(),
        }
    }
}

impl Default for DefaultRegistryDispatcher {
    fn default() -> Self {
        DefaultRegistryDispatcher {}
    }
}

impl Default for RegistryType {
    fn default() -> Self {
        RegistryType::System
    }
}
