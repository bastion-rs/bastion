use crate::context::BastionId;
use dashmap::DashMap;

pub type BastionRegistryMap = DashMap<BastionId, String>;

pub trait RegistryDispatcher {
    fn dispatch(&self, entries: &BastionRegistryMap);
}

#[derive(Debug)]
pub enum RegistryType {
    System,
    Named(String),
}

pub struct DefaultRegistryDispatcher;

pub struct Registry {
    registry_type: RegistryType,
    dispatcher: Box<dyn RegistryDispatcher>,
    storage: BastionRegistryMap,
}

impl Registry {
    pub fn with_registry_type(mut self, registry_type: RegistryType) -> Self {
        trace!("Setting registry the {:?} name.", self.registry_type);
        self.registry_type = registry_type;
        self
    }

    pub fn with_dispatcher(mut self, dispatcher: Box<dyn RegistryDispatcher>) -> Self {
        trace!(
            "Setting dispatcher for the {:?} registry.",
            self.registry_type
        );
        self.dispatcher = dispatcher;
        self
    }

    pub fn register(&self, key: BastionId, module_name: String) {
        self.storage.insert(key, module_name);
        self.dispatcher.dispatch(&self.storage);
    }

    pub fn remove(&self, key: BastionId) {
        self.storage.remove(&key);
    }
}

impl RegistryDispatcher for DefaultRegistryDispatcher {
    fn dispatch(&self, _entries: &BastionRegistryMap) {}
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
