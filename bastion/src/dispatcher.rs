//!
//! Special module that allows users to interact and communicate with a
//! group of actors through the dispatchers that holds information about
//! actors grouped together.
use crate::child_ref::ChildRef;
use crate::envelope::SignedMessage;
use dashmap::DashMap;
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

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
    /// Broadcasts the message to actors in according to the implemented behaviour.
    fn broadcast_message(&self, entries: &DispatcherMap, message: &Arc<SignedMessage>);
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
    pub fn broadcast_message(&self, message: &Arc<SignedMessage>) {
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
    fn broadcast_message(&self, _entries: &DispatcherMap, _message: &Arc<SignedMessage>) {}
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
    pub dispatchers: DashMap<DispatcherType, Arc<Box<Dispatcher>>>,
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
        child_ref: &ChildRef,
        module_name: String,
    ) {
        dispatchers
            .iter()
            .filter(|key| self.dispatchers.contains_key(*key))
            .for_each(|key| {
                if let Some(dispatcher) = self.dispatchers.get(key) {
                    dispatcher.register(child_ref, module_name.clone())
                }
            })
    }

    /// Removes and then returns the record from the registry by the given key.
    /// Returns `None` when the record wasn't found by the given key.
    pub(crate) fn remove(&self, dispatchers: &Vec<DispatcherType>, child_ref: &ChildRef) {
        dispatchers
            .iter()
            .filter(|key| self.dispatchers.contains_key(*key))
            .for_each(|key| {
                if let Some(dispatcher) = self.dispatchers.get(key) {
                    dispatcher.remove(child_ref)
                }
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
    pub(crate) fn broadcast_message(&self, target: BroadcastTarget, message: &Arc<SignedMessage>) {
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
                    dispatcher.broadcast_message(&message.clone());
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

    /// Adds dispatcher to the global registry.
    pub(crate) fn register_dispatcher(&self, dispatcher: &Arc<Box<Dispatcher>>) {
        let dispatcher_type = dispatcher.dispatcher_type();
        let is_registered = self.dispatchers.contains_key(&dispatcher_type.clone());

        if is_registered && dispatcher_type != DispatcherType::Anonymous {
            warn!(
                "The dispatcher with the '{:?}' name already registered in the cluster.",
                dispatcher_type
            );
            return;
        }

        let instance = dispatcher.clone();
        self.dispatchers.insert(dispatcher_type, instance);
    }

    /// Removes dispatcher from the global registry.
    pub(crate) fn remove_dispatcher(&self, dispatcher: &Arc<Box<Dispatcher>>) {
        self.dispatchers.remove(&dispatcher.dispatcher_type());
    }
}

#[cfg(test)]
mod tests {
    use crate::child_ref::ChildRef;
    use crate::context::BastionId;
    use crate::dispatcher::*;
    use crate::envelope::{RefAddr, SignedMessage};
    use crate::message::Msg;
    use crate::path::BastionPath;
    use futures::channel::mpsc;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct CustomHandler {
        called: Arc<Mutex<bool>>,
    }

    impl CustomHandler {
        pub fn new(value: bool) -> Self {
            CustomHandler {
                called: Arc::new(Mutex::new(value)),
            }
        }

        pub fn was_called(&self) -> bool {
            *self.called.clone().lock().unwrap()
        }
    }

    impl DispatcherHandler for CustomHandler {
        fn notify(
            &self,
            _from_child: &ChildRef,
            _entries: &DispatcherMap,
            _notification_type: NotificationType,
        ) {
            let handler_field_ref = self.called.clone();
            let mut data = handler_field_ref.lock().unwrap();
            *data = true;
        }

        fn broadcast_message(&self, _entries: &DispatcherMap, _message: &Arc<SignedMessage>) {
            let handler_field_ref = self.called.clone();
            let mut data = handler_field_ref.lock().unwrap();
            *data = true;
        }
    }

    #[test]
    fn test_get_dispatcher_type_as_anonymous() {
        let instance = Dispatcher::default();

        assert_eq!(instance.dispatcher_type(), DispatcherType::Anonymous);
    }

    #[test]
    fn test_get_dispatcher_type_as_named() {
        let name = "test_group".to_string();
        let dispatcher_type = DispatcherType::Named(name.clone());
        let instance = Dispatcher::default().with_dispatcher_type(dispatcher_type.clone());

        assert_eq!(instance.dispatcher_type(), dispatcher_type);
    }

    #[test]
    fn test_local_dispatcher_append_child_ref() {
        let instance = Dispatcher::default();
        let bastion_id = BastionId::new();
        let (sender, _) = mpsc::unbounded();
        let path = Arc::new(BastionPath::root());
        let child_ref = ChildRef::new(bastion_id, sender, path);

        assert_eq!(instance.actors.contains_key(&child_ref), false);

        instance.register(&child_ref, "my::test::module".to_string());
        assert_eq!(instance.actors.contains_key(&child_ref), true);
    }

    #[test]
    fn test_dispatcher_remove_child_ref() {
        let instance = Dispatcher::default();
        let bastion_id = BastionId::new();
        let (sender, _) = mpsc::unbounded();
        let path = Arc::new(BastionPath::root());
        let child_ref = ChildRef::new(bastion_id, sender, path);

        instance.register(&child_ref, "my::test::module".to_string());
        assert_eq!(instance.actors.contains_key(&child_ref), true);

        instance.remove(&child_ref);
        assert_eq!(instance.actors.contains_key(&child_ref), false);
    }

    #[test]
    fn test_local_dispatcher_notify() {
        let handler = Box::new(CustomHandler::new(false));
        let instance = Dispatcher::default().with_handler(handler.clone());
        let bastion_id = BastionId::new();
        let (sender, _) = mpsc::unbounded();
        let path = Arc::new(BastionPath::root());
        let child_ref = ChildRef::new(bastion_id, sender, path);

        instance.notify(&child_ref, NotificationType::Register);
        let handler_was_called = handler.was_called();
        assert_eq!(handler_was_called, true);
    }

    #[test]
    fn test_local_dispatcher_broadcast_message() {
        let handler = Box::new(CustomHandler::new(false));
        let instance = Dispatcher::default().with_handler(handler.clone());
        let (sender, _) = mpsc::unbounded();
        let path = Arc::new(BastionPath::root());

        const DATA: &'static str = "A message containing data (ask).";
        let message = Arc::new(SignedMessage::new(Msg::broadcast(DATA), RefAddr::new(path, sender)));

        instance.broadcast_message(&message);
        let handler_was_called = handler.was_called();
        assert_eq!(handler_was_called, true);
    }

    #[test]
    fn test_global_dispatcher_add_local_dispatcher() {
        let dispatcher_type = DispatcherType::Named("test".to_string());
        let local_dispatcher = Arc::new(Box::new(
            Dispatcher::default().with_dispatcher_type(dispatcher_type.clone()),
        ));
        let global_dispatcher = GlobalDispatcher::new();

        assert_eq!(
            global_dispatcher.dispatchers.contains_key(&dispatcher_type),
            false
        );

        global_dispatcher.register_dispatcher(&local_dispatcher);
        assert_eq!(
            global_dispatcher.dispatchers.contains_key(&dispatcher_type),
            true
        );
    }

    #[test]
    fn test_global_dispatcher_remove_local_dispatcher() {
        let dispatcher_type = DispatcherType::Named("test".to_string());
        let local_dispatcher = Arc::new(Box::new(
            Dispatcher::default().with_dispatcher_type(dispatcher_type.clone()),
        ));
        let global_dispatcher = GlobalDispatcher::new();

        global_dispatcher.register_dispatcher(&local_dispatcher);
        assert_eq!(
            global_dispatcher.dispatchers.contains_key(&dispatcher_type),
            true
        );

        global_dispatcher.remove_dispatcher(&local_dispatcher);
        assert_eq!(
            global_dispatcher.dispatchers.contains_key(&dispatcher_type),
            false
        );
    }

    #[test]
    fn test_global_dispatcher_register_actor() {
        let bastion_id = BastionId::new();
        let (sender, _) = mpsc::unbounded();
        let path = Arc::new(BastionPath::root());
        let child_ref = ChildRef::new(bastion_id, sender, path);

        let dispatcher_type = DispatcherType::Named("test".to_string());
        let local_dispatcher = Arc::new(Box::new(
            Dispatcher::default().with_dispatcher_type(dispatcher_type.clone()),
        ));
        let actor_groups = vec![dispatcher_type];
        let module_name = "my::test::module".to_string();

        let global_dispatcher = GlobalDispatcher::new();
        global_dispatcher.register_dispatcher(&local_dispatcher);

        assert_eq!(local_dispatcher.actors.contains_key(&child_ref), false);

        global_dispatcher.register(&actor_groups, &child_ref, module_name);
        assert_eq!(local_dispatcher.actors.contains_key(&child_ref), true);
    }

    #[test]
    fn test_global_dispatcher_remove_actor() {
        let bastion_id = BastionId::new();
        let (sender, _) = mpsc::unbounded();
        let path = Arc::new(BastionPath::root());
        let child_ref = ChildRef::new(bastion_id, sender, path);

        let dispatcher_type = DispatcherType::Named("test".to_string());
        let local_dispatcher = Arc::new(Box::new(
            Dispatcher::default().with_dispatcher_type(dispatcher_type.clone()),
        ));
        let actor_groups = vec![dispatcher_type];
        let module_name = "my::test::module".to_string();

        let global_dispatcher = GlobalDispatcher::new();
        global_dispatcher.register_dispatcher(&local_dispatcher);

        global_dispatcher.register(&actor_groups, &child_ref, module_name);
        assert_eq!(local_dispatcher.actors.contains_key(&child_ref), true);

        global_dispatcher.remove(&actor_groups, &child_ref);
        assert_eq!(local_dispatcher.actors.contains_key(&child_ref), false);
    }

    #[test]
    fn test_global_dispatcher_notify() {
        let bastion_id = BastionId::new();
        let (sender, _) = mpsc::unbounded();
        let path = Arc::new(BastionPath::root());
        let child_ref = ChildRef::new(bastion_id, sender, path);

        let dispatcher_type = DispatcherType::Named("test".to_string());
        let handler = Box::new(CustomHandler::new(false));
        let local_dispatcher = Arc::new(Box::new(
            Dispatcher::default()
                .with_dispatcher_type(dispatcher_type.clone())
                .with_handler(handler.clone()),
        ));
        let actor_groups = vec![dispatcher_type];
        let module_name = "my::test::module".to_string();

        let global_dispatcher = GlobalDispatcher::new();
        global_dispatcher.register_dispatcher(&local_dispatcher);
        global_dispatcher.register(&actor_groups, &child_ref, module_name);

        global_dispatcher.notify(&child_ref, &actor_groups, NotificationType::Register);
        let handler_was_called = handler.was_called();
        assert_eq!(handler_was_called, true);
    }

    #[test]
    fn test_global_dispatcher_broadcast_message() {
        let bastion_id = BastionId::new();
        let (sender, _) = mpsc::unbounded();
        let path = Arc::new(BastionPath::root());
        let child_ref = ChildRef::new(bastion_id, sender, path);

        let dispatcher_type = DispatcherType::Named("test".to_string());
        let handler = Box::new(CustomHandler::new(false));
        let local_dispatcher = Arc::new(Box::new(
            Dispatcher::default()
                .with_dispatcher_type(dispatcher_type.clone())
                .with_handler(handler.clone()),
        ));
        let actor_groups = vec![dispatcher_type];
        let module_name = "my::test::module".to_string();

        let global_dispatcher = GlobalDispatcher::new();
        global_dispatcher.register_dispatcher(&local_dispatcher);
        global_dispatcher.register(&actor_groups, &child_ref, module_name);

        let (sender, _) = mpsc::unbounded();
        let path = Arc::new(BastionPath::root());
        const DATA: &'static str = "A message containing data (ask).";
        let message = Arc::new(SignedMessage::new(Msg::broadcast(DATA), RefAddr::new(path, sender)));

        global_dispatcher.broadcast_message(BroadcastTarget::Group("".to_string()), &message);
        let handler_was_called = handler.was_called();
        assert_eq!(handler_was_called, true);
    }
}
