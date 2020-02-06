//!
//! Special module that allows users to interact and communicate with a
//! group of actors through the dispatchers that holds information about
//! actors grouped together.
use crate::child_ref::ChildRef;
use crate::envelope::SignedMessage;
use dashmap::DashMap;
use std::fmt::{self, Debug};

/// Type alias for the concurrency hashmap. Each key-value pair stores
/// the Bastion identifier as the key and the module name as the value.
pub type BastionDispatcherMap = DashMap<ChildRef, String>;

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

#[derive(Debug, Clone)]
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
    fn notify(&self, entries: &BastionDispatcherMap, notification_type: NotificationType);
    fn broadcast_message(&self, entries: &BastionDispatcherMap, message: SignedMessage);
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
    /// The handler used for each `notification` or a message.
    handler: Box<dyn DispatcherHandler>,
    /// Special field that stores information about all
    /// registered actors in the group.
    actors: BastionDispatcherMap,
}

impl Dispatcher {
    /// Returns the type of the dispatcher.
    pub fn dispatcher_type(&self) -> DispatcherType {
        self.dispatcher_type.clone()
    }

    /// Returns the used handler by the dispatcher.
    pub fn handler(&self) -> &Box<dyn DispatcherHandler> {
        &self.handler
    }

    /// Sets the dispatcher type.
    pub fn with_dispatcher_type(mut self, dispatcher_type: DispatcherType) -> Self {
        trace!("Setting dispatcher the {:?} type.", dispatcher_type);
        self.dispatcher_type = dispatcher_type;
        self
    }

    /// Sets the handler for the dispatcher.
    pub fn with_handler(mut self, handler: Box<dyn DispatcherHandler>) -> Self {
        trace!(
            "Setting handler for the {:?} dispatcher.",
            self.dispatcher_type
        );
        self.handler = handler;
        self
    }

    /// Appends the information about actor to the dispatcher.
    pub fn register(&self, key: ChildRef, module_name: String) {
        self.actors.insert(key, module_name);
        self.handler
            .notify(&self.actors, NotificationType::Register);
    }

    /// Removes and then returns the record from the registry by the given key.
    /// Returns `None` when the record wasn't found by the given key.
    pub fn remove(&self, key: ChildRef) -> Option<(ChildRef, String)> {
        let result = self.actors.remove(&key);
        if result.is_some() {
            self.handler.notify(&self.actors, NotificationType::Remove);
        }
        result
    }

    /// Sends the message to the group of actors.
    /// The logic of who and how should receive the message relies onto
    /// the handler implementation.
    pub fn broadcast_message(&self, message: SignedMessage) {
        self.handler.broadcast_message(&self.actors, message);
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
    fn notify(&self, entries: &BastionDispatcherMap, notification_type: NotificationType) {}
    fn broadcast_message(&self, entries: &BastionDispatcherMap, message: SignedMessage) {}
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
