//!
//! Special module that allows users to interact and communicate with a
//! group of actors through the dispatchers that holds information about
//! actors grouped together.
use crate::child_ref::ChildRef;
use crate::envelope::SignedMessage;
use dashmap::DashMap;
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};

/// Type alias for the concurrency hashmap. Each key-value pair stores
/// the Bastion identifier as the key and the module name as the value.
pub type DispatcherMap = DashMap<ChildRef, String>;

#[derive(Debug, Clone)]
/// Defines types of the notifications handled by the dispatcher
/// when the group of actors is changing.
pub enum NotificationType {
    /// Represents a notification when a new actor wants to
    /// join to the existing group of actors.
    Register,
    /// Represents a notification when the existing actor
    /// was stopped, killed, suspended or finished an execution.
    Remove,
}

#[derive(Debug, Clone)]
/// Defines types of the notifications handled by the dispatcher
/// when the group of actors is changing.
///
/// If the message can't be delivered to the declared group, then
/// the message will be marked as the "dead letter".
pub enum BroadcastTarget {
    /// Send the broadcasted message to everyone in the system.
    All,
    /// Send the broadcasted message to each actor in group.
    Group(String),
}

#[derive(Debug, Clone, Eq, PartialEq)]
/// Defines the type of the dispatcher.
///
/// The default type is `Anonymous`.
pub enum DispatcherType {
    /// The default kind of the dispatcher which is using for
    /// handling all actors in the cluster. Can be more than
    /// one instance of this type.
    Anonymous,
    /// The dispatcher with a unique name which will be using
    /// for updating and notifying actors in the same group
    /// base on the desired strategy. The logic handling broadcasted
    /// messages and their distribution across the group depends on
    /// the dispatcher's handler.
    Named(String),
}

#[derive(Debug)]
/// The special handler, which is creating by default. Initially
/// doesn't do any useful processing.
pub struct DefaultDispatcherHandler;

/// Generic trait which any custom dispatcher handler must implement for
/// the further usage by the `Dispatcher` instances.
pub trait DispatcherHandler {
    /// Sends the notification of the certain type to each actor in group.
    fn notify(
        &self,
        from_child: &ChildRef,
        entries: &DispatcherMap,
        notification_type: NotificationType,
    );
    /// Broadcast the message to actors in according to the implemented behaviour.
    fn broadcast_message(&self, entries: &DispatcherMap, message: &SignedMessage);
}

/// A generic implementation of the Bastion dispatcher
///
/// The main idea of the dispatcher is to provide an alternative way to
/// communicate between a group of actors. For example, dispatcher can
/// be used when a developer wants to send a specific message or share a
/// local state between the specific group of registered actors with
/// the usage of a custom dispatcher.
pub struct Dispatcher {
    /// Defines the type of the dispatcher.
    dispatcher_type: DispatcherType,
    /// The handler used for a notification or a message.
    handler: Box<dyn DispatcherHandler + Send + Sync + 'static>,
    /// Special field that stores information about all
    /// registered actors in the group.
    actors: DispatcherMap,
}

impl Dispatcher {
    /// Returns the type of the dispatcher.
    pub fn dispatcher_type(&self) -> DispatcherType {
        self.dispatcher_type.clone()
    }

    /// Returns the used handler by the dispatcher.
    pub fn handler(&self) -> &Box<dyn DispatcherHandler + Send + Sync + 'static> {
        &self.handler
    }

    /// Sets the dispatcher type.
    pub fn with_dispatcher_type(mut self, dispatcher_type: DispatcherType) -> Self {
        trace!("Setting dispatcher the {:?} type.", dispatcher_type);
        self.dispatcher_type = dispatcher_type;
        self
    }

    /// Sets the handler for the dispatcher.
    pub fn with_handler(
        mut self,
        handler: Box<dyn DispatcherHandler + Send + Sync + 'static>,
    ) -> Self {
        trace!(
            "Setting handler for the {:?} dispatcher.",
            self.dispatcher_type
        );
        self.handler = handler;
        self
    }

    /// Appends the information about actor to the dispatcher.
    pub(crate) fn register(&self, key: &ChildRef, module_name: String) {
        self.actors.insert(key.to_owned(), module_name);
        self.handler
            .notify(key, &self.actors, NotificationType::Register);
    }

    /// Removes and then returns the record from the registry by the given key.
    /// Returns `None` when the record wasn't found by the given key.
    pub(crate) fn remove(&self, key: &ChildRef) {
        if let Some(_) = self.actors.remove(key) {
            self.handler
                .notify(key, &self.actors, NotificationType::Remove);
        }
    }

    /// Forwards the message to the handler for processing.
    pub fn notify(&self, from_child: &ChildRef, notification_type: NotificationType) {
        self.handler
            .notify(from_child, &self.actors, notification_type)
    }

    /// Sends the message to the group of actors.
    /// The logic of who and how should receive the message relies onto
    /// the handler implementation.
    pub fn broadcast_message(&self, message: &SignedMessage) {
        self.handler.broadcast_message(&self.actors, &message);
    }
}

impl Debug for Dispatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Dispatcher(type: {:?}, actors: {:?})",
            self.dispatcher_type,
            self.actors.len()
        )
    }
}

impl DispatcherHandler for DefaultDispatcherHandler {
    fn notify(
        &self,
        _from_child: &ChildRef,
        _entries: &DispatcherMap,
        _notification_type: NotificationType,
    ) {
    }
    fn broadcast_message(&self, _entries: &DispatcherMap, _message: &SignedMessage) {}
}

impl DispatcherType {
    pub(crate) fn name(&self) -> String {
        match self {
            DispatcherType::Anonymous => String::from("__Anonymous__"),
            DispatcherType::Named(value) => value.to_owned(),
        }
    }
}

impl Default for Dispatcher {
    fn default() -> Self {
        Dispatcher {
            dispatcher_type: DispatcherType::default(),
            handler: Box::new(DefaultDispatcherHandler::default()),
            actors: DashMap::new(),
        }
    }
}

impl Default for DefaultDispatcherHandler {
    fn default() -> Self {
        DefaultDispatcherHandler {}
    }
}

impl Default for DispatcherType {
    fn default() -> Self {
        DispatcherType::Anonymous
    }
}

impl Hash for DispatcherType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name().hash(state);
    }
}

impl Into<DispatcherType> for String {
    fn into(self) -> DispatcherType {
        match self == DispatcherType::Anonymous.name() {
            true => DispatcherType::Anonymous,
            false => DispatcherType::Named(self),
        }
    }
}

#[derive(Debug)]
/// The global dispatcher of bastion the cluster.
///
/// The main purpose of this dispatcher is be a point through
/// developers can communicate with actors through group names.
pub(crate) struct GlobalDispatcher {
    /// Storage for all registered group of actors.
    dispatchers: DashMap<DispatcherType, Dispatcher>,
}

impl GlobalDispatcher {
    /// Creates a new instance of the global registry.
    pub(crate) fn new() -> Self {
        GlobalDispatcher {
            dispatchers: DashMap::new(),
        }
    }

    /// Appends the information about actor to the dispatcher.
    pub(crate) fn register(
        &self,
        dispatchers: &Vec<DispatcherType>,
        key: &ChildRef,
        module_name: String,
    ) {
        self.dispatchers
            .iter()
            .filter(|pair| dispatchers.contains(&pair.key()))
            .for_each(|pair| {
                let dispatcher = pair.value();
                dispatcher.register(key, module_name.clone())
            })
    }

    /// Removes and then returns the record from the registry by the given key.
    /// Returns `None` when the record wasn't found by the given key.
    pub(crate) fn remove(&self, dispatchers: &Vec<DispatcherType>, key: &ChildRef) {
        self.dispatchers
            .iter()
            .filter(|pair| dispatchers.contains(&pair.key()))
            .for_each(|pair| {
                let dispatcher = pair.value();
                dispatcher.remove(key)
            })
    }

    /// Passes the notification from the actor to everyone that registered in the same
    /// groups as the caller.
    pub(crate) fn notify(
        &self,
        from_actor: &ChildRef,
        dispatchers: &Vec<DispatcherType>,
        notification_type: NotificationType,
    ) {
        self.dispatchers
            .iter()
            .filter(|pair| dispatchers.contains(&pair.key()))
            .for_each(|pair| {
                let dispatcher = pair.value();
                dispatcher.notify(from_actor, notification_type.clone())
            })
    }

    /// Broadcasts the given message in according with the specified target.
    pub(crate) fn broadcast_message(&self, target: BroadcastTarget, message: &SignedMessage) {
        let mut acked_dispatchers: Vec<DispatcherType> = Vec::new();

        match target {
            BroadcastTarget::All => self
                .dispatchers
                .iter()
                .map(|pair| pair.key().name().into())
                .for_each(|group_name| acked_dispatchers.push(group_name)),
            BroadcastTarget::Group(name) => {
                let target_dispatcher = name.into();
                acked_dispatchers.push(target_dispatcher);
            }
        }

        for dispatcher_type in acked_dispatchers {
            match self.dispatchers.get(&dispatcher_type) {
                Some(pair) => {
                    let dispatcher = pair.value();
                    dispatcher.broadcast_message(message);
                }
                // TODO: Put the message into the dead queue
                None => {
                    let name = dispatcher_type.name();
                    warn!(
                        "The message can't be delivered to the group with the '{}' name.",
                        name
                    );
                }
            }
        }
    }
}
