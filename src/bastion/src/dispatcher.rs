//!
//! Special module that allows users to interact and communicate with a
//! group of actors through the dispatchers that holds information about
//! actors grouped together.
use crate::{
    child_ref::ChildRef,
    message::{Answer, Message},
    prelude::SendError,
};
use crate::{distributor::Distributor, envelope::SignedMessage};
use anyhow::Result as AnyResult;
use futures::Future;
use lever::prelude::*;
use std::sync::RwLock;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::{
    collections::HashMap,
    fmt::{self, Debug},
};
use std::{
    hash::{Hash, Hasher},
    task::Poll,
};
use tracing::{debug, trace};

/// Type alias for the concurrency hashmap. Each key-value pair stores
/// the Bastion identifier as the key and the module name as the value.
pub type DispatcherMap = LOTable<ChildRef, String>;

/// Type alias for the recipients hashset.
/// Each key-value pair stores the Bastion identifier as the key.
pub type RecipientMap = LOTable<ChildRef, ()>;

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

/// A `Recipient` is responsible for maintaining it's list
/// of recipients, and deciding which child gets to receive which message.
pub trait Recipient {
    /// Provide this function to declare which recipient will receive the next message
    fn next(&self) -> Option<ChildRef>;
    /// Return all recipients that will receive a broadcast message
    fn all(&self) -> Vec<ChildRef>;
    /// Add this actor to your list of recipients
    fn register(&self, actor: ChildRef);
    /// Remove this actor from your list of recipients
    fn remove(&self, actor: &ChildRef);
}

/// A `RecipientHandler` is a `Recipient` implementor, that can be stored in the dispatcher
pub trait RecipientHandler: Recipient + Send + Sync + Debug {}

impl Future for RoundRobinHandler {
    type Output = Vec<ChildRef>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let recipients = self.public_recipients();
        if !recipients.is_empty() {
            return Poll::Ready(recipients);
        }

        self.waker.register(cx.waker());

        let recipients = self.public_recipients();
        if !recipients.is_empty() {
            Poll::Ready(recipients)
        } else {
            Poll::Pending
        }
    }
}

impl RecipientHandler for RoundRobinHandler {}

/// The default handler, which does round-robin.
pub type DefaultRecipientHandler = RoundRobinHandler;

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

/// The default handler, which does round-robin.
pub type DefaultDispatcherHandler = RoundRobinHandler;

/// Dispatcher that will do simple round-robin distribution
#[derive(Default, Debug)]
pub struct RoundRobinHandler {
    index: AtomicUsize,
    recipients: RecipientMap,
    waker: futures::task::AtomicWaker,
}

impl RoundRobinHandler {
    fn public_recipients(&self) -> Vec<ChildRef> {
        self.recipients
            .iter()
            .filter_map(|entry| {
                if entry.0.is_public() {
                    Some(entry.0)
                } else {
                    None
                }
            })
            .collect()
    }
}

impl RoundRobinHandler {
    async fn poll_next(&mut self) -> ChildRef {
        let index = self.index.fetch_add(1, Ordering::SeqCst);
        let recipients = self.await;
        // TODO [igni]: unwrap?!
        recipients
            .get(index % recipients.len())
            .map(std::clone::Clone::clone)
            .unwrap()
    }

    async fn poll_all(&mut self) -> Vec<ChildRef> {
        self.await
    }
}

impl Recipient for RoundRobinHandler {
    fn next(&self) -> Option<ChildRef> {
        let entries = self.public_recipients();

        if entries.is_empty() {
            return None;
        }

        let current_index = self.index.load(Ordering::SeqCst) % entries.len();
        self.index.store(current_index + 1, Ordering::SeqCst);
        entries.get(current_index).map(std::clone::Clone::clone)
    }

    fn all(&self) -> Vec<ChildRef> {
        self.public_recipients()
    }

    fn register(&self, actor: ChildRef) {
        let _ = self.recipients.insert(actor, ());
        self.waker.wake();
    }

    fn remove(&self, actor: &ChildRef) {
        let _ = self.recipients.remove(&actor);
    }
}

impl DispatcherHandler for RoundRobinHandler {
    // Will left this implementation as empty.
    fn notify(
        &self,
        _from_child: &ChildRef,
        _entries: &DispatcherMap,
        _notification_type: NotificationType,
    ) {
    }
    // Each child in turn will receive a message.
    fn broadcast_message(&self, entries: &DispatcherMap, message: &Arc<SignedMessage>) {
        let public_childrefs = entries
            .iter()
            .filter_map(|entry| {
                if entry.0.is_public() {
                    Some(entry.0)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if public_childrefs.is_empty() {
            debug!("no public children to broadcast message to");
            return;
        }
        let current_index = self.index.load(Ordering::SeqCst) % public_childrefs.len();

        if let Some(entry) = public_childrefs.get(current_index) {
            debug!(
                "sending message to child {}/{} - {}",
                current_index + 1,
                entries.len(),
                entry.path()
            );
            entry.tell_anonymously(message.clone()).unwrap();
            self.index.store(current_index + 1, Ordering::SeqCst);
        };
    }
}
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
    pub fn handler(&self) -> &(dyn DispatcherHandler + Send + Sync + 'static) {
        &*self.handler
    }

    /// Sets the dispatcher type.
    pub fn with_dispatcher_type(mut self, dispatcher_type: DispatcherType) -> Self {
        trace!("Setting dispatcher the {:?} type.", dispatcher_type);
        self.dispatcher_type = dispatcher_type;
        self
    }

    /// Creates a dispatcher with a specific dispatcher type.
    pub fn with_type(dispatcher_type: DispatcherType) -> Self {
        trace!(
            "Instanciating a dispatcher with type {:?}.",
            dispatcher_type
        );
        Self {
            dispatcher_type,
            handler: Box::new(DefaultDispatcherHandler::default()),
            actors: Default::default(),
        }
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
    pub(crate) fn register(&self, key: &ChildRef, module_name: String) -> AnyResult<()> {
        self.actors.insert(key.to_owned(), module_name)?;
        self.handler
            .notify(key, &self.actors, NotificationType::Register);
        Ok(())
    }

    /// Removes and then returns the record from the registry by the given key.
    /// Returns `None` when the record wasn't found by the given key.
    pub(crate) fn remove(&self, key: &ChildRef) {
        if self.actors.remove(key).is_ok() {
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
            actors: LOTable::new(),
        }
    }
}

impl Default for DispatcherType {
    fn default() -> Self {
        DispatcherType::Anonymous
    }
}

#[allow(clippy::derive_hash_xor_eq)]
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
    pub dispatchers: LOTable<DispatcherType, Arc<Box<Dispatcher>>>,
    // TODO: switch to LOTable once lever implements write optimized granularity
    pub distributors: Arc<RwLock<HashMap<Distributor, Box<(dyn RecipientHandler)>>>>,
}

impl GlobalDispatcher {
    /// Creates a new instance of the global registry.
    pub(crate) fn new() -> Self {
        GlobalDispatcher {
            dispatchers: LOTable::new(),
            distributors: Arc::new(RwLock::new(HashMap::new()))
            // TODO: switch to LOTable once lever implements write optimized granularity
            // distributors: LOTableBuilder::new()
                //.with_concurrency(TransactionConcurrency::Optimistic)
                //.with_isolation(TransactionIsolation::Serializable)
                //.build(),
        }
    }

    /// Appends the information about actor to the dispatcher.
    pub(crate) fn register(
        &self,
        dispatchers: &[DispatcherType],
        child_ref: &ChildRef,
        module_name: String,
    ) -> AnyResult<()> {
        dispatchers
            .iter()
            .filter(|key| self.dispatchers.contains_key(*key))
            .map(|key| {
                if let Some(dispatcher) = self.dispatchers.get(key) {
                    dispatcher.register(child_ref, module_name.clone())
                } else {
                    Ok(())
                }
            })
            .collect::<AnyResult<Vec<_>>>()?;
        Ok(())
    }

    /// Removes and then returns the record from the registry by the given key.
    /// Returns `None` when the record wasn't found by the given key.
    pub(crate) fn remove(&self, dispatchers: &[DispatcherType], child_ref: &ChildRef) {
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
        dispatchers: &[DispatcherType],
        notification_type: NotificationType,
    ) {
        self.dispatchers
            .iter()
            .filter(|pair| dispatchers.contains(&pair.0))
            .for_each(|pair| {
                let dispatcher = pair.1;
                dispatcher.notify(from_actor, notification_type.clone())
            })
    }

    /// Broadcasts the given message in according with the specified target.
    pub(crate) fn broadcast_message(&self, target: BroadcastTarget, message: &Arc<SignedMessage>) {
        let acked_dispatchers = match target {
            BroadcastTarget::All => self
                .dispatchers
                .iter()
                .map(|pair| pair.0.name().into())
                .collect(),
            BroadcastTarget::Group(name) => {
                let target_dispatcher = name.into();
                vec![target_dispatcher]
            }
        };

        for dispatcher_type in acked_dispatchers {
            match self.dispatchers.get(&dispatcher_type) {
                Some(dispatcher) => {
                    dispatcher.broadcast_message(&message.clone());
                }
                // TODO: Put the message into the dead queue
                None => {
                    let name = dispatcher_type.name();
                    debug!(
                        "The message can't be delivered to the group with the '{}' name.",
                        name
                    );
                }
            }
        }
    }

    pub(crate) fn tell<M>(&self, distributor: Distributor, message: M) -> Result<(), SendError>
    where
        M: Message,
    {
        let child = self.next(distributor)?.ok_or(SendError::EmptyRecipient)?;
        child.try_tell_anonymously(message).map(Into::into)
    }

    pub(crate) fn ask<M>(&self, distributor: Distributor, message: M) -> Result<Answer, SendError>
    where
        M: Message,
    {
        let child = self.next(distributor)?.ok_or(SendError::EmptyRecipient)?;
        child.try_ask_anonymously(message).map(Into::into)
    }

    pub(crate) fn ask_everyone<M>(
        &self,
        distributor: Distributor,
        message: M,
    ) -> Result<Vec<Answer>, SendError>
    where
        M: Message + Clone,
    {
        let all_children = self.all(distributor)?;
        if all_children.is_empty() {
            Err(SendError::EmptyRecipient)
        } else {
            all_children
                .iter()
                .map(|child| child.try_ask_anonymously(message.clone()))
                .collect::<Result<Vec<_>, _>>()
        }
    }

    pub(crate) fn tell_everyone<M>(
        &self,
        distributor: Distributor,
        message: M,
    ) -> Result<Vec<()>, SendError>
    where
        M: Message + Clone,
    {
        let all_children = self.all(distributor)?;
        if all_children.is_empty() {
            Err(SendError::EmptyRecipient)
        } else {
            all_children
                .iter()
                .map(|child| child.try_tell_anonymously(message.clone()))
                .collect()
        }
    }

    fn next(&self, distributor: Distributor) -> Result<Option<ChildRef>, SendError> {
        self.distributors
            .read()
            .map_err(|error| {
                SendError::Other(anyhow::anyhow!(
                    "couldn't get read lock on distributors {:?}",
                    error
                ))
            })?
            .get(&distributor)
            .map(|recipient| recipient.next())
            .ok_or_else(|| SendError::from(distributor))
    }

    fn all(&self, distributor: Distributor) -> Result<Vec<ChildRef>, SendError> {
        self.distributors
            .read()
            .map_err(|error| {
                SendError::Other(anyhow::anyhow!(
                    "couldn't get read lock on distributors {:?}",
                    error
                ))
            })?
            .get(&distributor)
            .map(|recipient| recipient.all())
            .ok_or_else(|| SendError::from(distributor))
    }

    /// Adds dispatcher to the global registry.
    pub(crate) fn register_dispatcher(&self, dispatcher: &Arc<Box<Dispatcher>>) -> AnyResult<()> {
        let dispatcher_type = dispatcher.dispatcher_type();
        let is_registered = self.dispatchers.contains_key(&dispatcher_type);

        if is_registered && dispatcher_type != DispatcherType::Anonymous {
            debug!(
                "The dispatcher with the '{:?}' name already registered in the cluster.",
                dispatcher_type
            );
            return Ok(());
        }

        let instance = dispatcher.clone();
        self.dispatchers.insert(dispatcher_type, instance)?;
        Ok(())
    }

    /// Removes dispatcher from the global registry.
    pub(crate) fn remove_dispatcher(&self, dispatcher: &Arc<Box<Dispatcher>>) -> AnyResult<()> {
        self.dispatchers.remove(&dispatcher.dispatcher_type())?;
        Ok(())
    }

    /// Appends the information about actor to the recipients.
    pub(crate) fn register_recipient(
        &self,
        distributor: &Distributor,
        child_ref: ChildRef,
    ) -> AnyResult<()> {
        let mut distributors = self.distributors.write().map_err(|error| {
            anyhow::anyhow!("couldn't get read lock on distributors {:?}", error)
        })?;
        if let Some(recipients) = distributors.get(&distributor) {
            recipients.register(child_ref);
        } else {
            let recipients = DefaultRecipientHandler::default();
            recipients.register(child_ref);
            distributors.insert(
                distributor.clone(),
                Box::new(recipients) as Box<(dyn RecipientHandler)>,
            );
        };
        Ok(())
    }

    pub(crate) fn remove_recipient(
        &self,
        distributor_list: &[Distributor],
        child_ref: &ChildRef,
    ) -> AnyResult<()> {
        let distributors = self.distributors.write().map_err(|error| {
            anyhow::anyhow!("couldn't get read lock on distributors {:?}", error)
        })?;
        distributor_list.iter().for_each(|distributor| {
            distributors
                .get(&distributor)
                .map(|recipients| recipients.remove(child_ref));
        });
        Ok(())
    }

    /// Adds distributor to the global registry.
    pub(crate) fn register_distributor(&self, distributor: &Distributor) -> AnyResult<()> {
        let mut distributors = self.distributors.write().map_err(|error| {
            anyhow::anyhow!("couldn't get read lock on distributors {:?}", error)
        })?;
        if distributors.contains_key(&distributor) {
            debug!(
                "The distributor with the '{:?}' name already registered in the cluster.",
                distributor
            );
        } else {
            distributors.insert(
                distributor.clone(),
                Box::new(DefaultRecipientHandler::default()),
            );
        }
        Ok(())
    }

    /// Removes distributor from the global registry if it has no remaining recipients.
    pub(crate) fn remove_distributor(&self, distributor: &Distributor) -> AnyResult<()> {
        let mut distributors = self.distributors.write().map_err(|error| {
            anyhow::anyhow!("couldn't get write lock on distributors {:?}", error)
        })?;
        if distributors
            .get(&distributor)
            .map(|recipient| recipient.all().is_empty())
            .unwrap_or_default()
        {
            distributors.remove(distributor);
        }
        Ok(())
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

    // Here we actually want both
    // the locking mechanism and
    // the bool value
    #[allow(clippy::mutex_atomic)]
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
        let dispatcher_type = DispatcherType::Named(name);
        let instance = Dispatcher::with_type(dispatcher_type.clone());

        assert_eq!(instance.dispatcher_type(), dispatcher_type);
    }

    #[test]
    fn test_local_dispatcher_append_child_ref() {
        let instance = Dispatcher::default();
        let bastion_id = BastionId::new();
        let (sender, _) = mpsc::unbounded();
        let path = Arc::new(BastionPath::root());
        let name = "test_name".to_string();
        let child_ref = ChildRef::new(bastion_id, sender, name, path);

        assert_eq!(instance.actors.contains_key(&child_ref), false);

        instance
            .register(&child_ref, "my::test::module".to_string())
            .unwrap();
        assert_eq!(instance.actors.contains_key(&child_ref), true);
    }

    #[test]
    fn test_dispatcher_remove_child_ref() {
        let instance = Dispatcher::default();
        let bastion_id = BastionId::new();
        let (sender, _) = mpsc::unbounded();
        let path = Arc::new(BastionPath::root());
        let name = "test_name".to_string();
        let child_ref = ChildRef::new(bastion_id, sender, name, path);

        instance
            .register(&child_ref, "my::test::module".to_string())
            .unwrap();
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
        let name = "test_name".to_string();
        let child_ref = ChildRef::new(bastion_id, sender, name, path);

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

        const DATA: &str = "A message containing data (ask).";
        let message = Arc::new(SignedMessage::new(
            Msg::broadcast(DATA),
            RefAddr::new(path, sender),
        ));

        instance.broadcast_message(&message);
        let handler_was_called = handler.was_called();
        assert_eq!(handler_was_called, true);
    }

    #[test]
    fn test_global_dispatcher_add_local_dispatcher() {
        let dispatcher_type = DispatcherType::Named("test".to_string());
        let local_dispatcher = Arc::new(Box::new(Dispatcher::with_type(dispatcher_type.clone())));
        let global_dispatcher = GlobalDispatcher::new();

        assert_eq!(
            global_dispatcher.dispatchers.contains_key(&dispatcher_type),
            false
        );

        global_dispatcher
            .register_dispatcher(&local_dispatcher)
            .unwrap();
        assert_eq!(
            global_dispatcher.dispatchers.contains_key(&dispatcher_type),
            true
        );
    }

    #[test]
    fn test_global_dispatcher_remove_local_dispatcher() {
        let dispatcher_type = DispatcherType::Named("test".to_string());
        let local_dispatcher = Arc::new(Box::new(Dispatcher::with_type(dispatcher_type.clone())));
        let global_dispatcher = GlobalDispatcher::new();

        global_dispatcher
            .register_dispatcher(&local_dispatcher)
            .unwrap();
        assert_eq!(
            global_dispatcher.dispatchers.contains_key(&dispatcher_type),
            true
        );

        global_dispatcher
            .remove_dispatcher(&local_dispatcher)
            .unwrap();
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
        let name = "test_name".to_string();
        let child_ref = ChildRef::new(bastion_id, sender, name, path);

        let dispatcher_type = DispatcherType::Named("test".to_string());
        let local_dispatcher = Arc::new(Box::new(Dispatcher::with_type(dispatcher_type.clone())));
        let actor_groups = vec![dispatcher_type];
        let module_name = "my::test::module".to_string();

        let global_dispatcher = GlobalDispatcher::new();
        global_dispatcher
            .register_dispatcher(&local_dispatcher)
            .unwrap();

        assert_eq!(local_dispatcher.actors.contains_key(&child_ref), false);

        global_dispatcher
            .register(&actor_groups, &child_ref, module_name)
            .unwrap();
        assert_eq!(local_dispatcher.actors.contains_key(&child_ref), true);
    }

    #[test]
    fn test_global_dispatcher_remove_actor() {
        let bastion_id = BastionId::new();
        let (sender, _) = mpsc::unbounded();
        let path = Arc::new(BastionPath::root());
        let name = "test_name".to_string();
        let child_ref = ChildRef::new(bastion_id, sender, name, path);

        let dispatcher_type = DispatcherType::Named("test".to_string());
        let local_dispatcher = Arc::new(Box::new(Dispatcher::with_type(dispatcher_type.clone())));
        let actor_groups = vec![dispatcher_type];
        let module_name = "my::test::module".to_string();

        let global_dispatcher = GlobalDispatcher::new();
        global_dispatcher
            .register_dispatcher(&local_dispatcher)
            .unwrap();

        global_dispatcher
            .register(&actor_groups, &child_ref, module_name)
            .unwrap();
        assert_eq!(local_dispatcher.actors.contains_key(&child_ref), true);

        global_dispatcher.remove(&actor_groups, &child_ref);
        assert_eq!(local_dispatcher.actors.contains_key(&child_ref), false);
    }

    #[test]
    fn test_global_dispatcher_notify() {
        let bastion_id = BastionId::new();
        let (sender, _) = mpsc::unbounded();
        let path = Arc::new(BastionPath::root());
        let name = "test_name".to_string();
        let child_ref = ChildRef::new(bastion_id, sender, name, path);

        let dispatcher_type = DispatcherType::Named("test".to_string());
        let handler = Box::new(CustomHandler::new(false));
        let local_dispatcher = Arc::new(Box::new(
            Dispatcher::with_type(dispatcher_type.clone()).with_handler(handler.clone()),
        ));
        let actor_groups = vec![dispatcher_type];
        let module_name = "my::test::module".to_string();

        let global_dispatcher = GlobalDispatcher::new();
        global_dispatcher
            .register_dispatcher(&local_dispatcher)
            .unwrap();
        global_dispatcher
            .register(&actor_groups, &child_ref, module_name)
            .unwrap();

        global_dispatcher.notify(&child_ref, &actor_groups, NotificationType::Register);
        let handler_was_called = handler.was_called();
        assert_eq!(handler_was_called, true);
    }

    #[test]
    fn test_global_dispatcher_broadcast_message() {
        let bastion_id = BastionId::new();
        let (sender, _) = mpsc::unbounded();
        let path = Arc::new(BastionPath::root());
        let name = "test_name".to_string();
        let child_ref = ChildRef::new(bastion_id, sender, name, path);

        let dispatcher_type = DispatcherType::Named("test".to_string());
        let handler = Box::new(CustomHandler::new(false));
        let local_dispatcher = Arc::new(Box::new(
            Dispatcher::with_type(dispatcher_type.clone()).with_handler(handler.clone()),
        ));
        let actor_groups = vec![dispatcher_type];
        let module_name = "my::test::module".to_string();

        let global_dispatcher = GlobalDispatcher::new();
        global_dispatcher
            .register_dispatcher(&local_dispatcher)
            .unwrap();
        global_dispatcher
            .register(&actor_groups, &child_ref, module_name)
            .unwrap();

        let (sender, _) = mpsc::unbounded();
        let path = Arc::new(BastionPath::root());
        const DATA: &str = "A message containing data (ask).";
        let message = Arc::new(SignedMessage::new(
            Msg::broadcast(DATA),
            RefAddr::new(path, sender),
        ));

        global_dispatcher.broadcast_message(BroadcastTarget::Group("".to_string()), &message);
        let handler_was_called = handler.was_called();
        assert_eq!(handler_was_called, true);
    }

    #[test]
    fn test_global_dispatcher_removes_distributor_with_no_recipients() {
        let global_dispatcher = GlobalDispatcher::new();
        let distributor = Distributor::named("test-distributor");
        global_dispatcher
            .register_distributor(&distributor)
            .unwrap();
        assert!(!global_dispatcher.distributors.read().unwrap().is_empty());
        global_dispatcher.remove_distributor(&distributor).unwrap();
        assert!(global_dispatcher.distributors.read().unwrap().is_empty());
    }

    #[test]
    fn test_global_dispatcher_keeps_distributor_with_recipients() {
        let bastion_id = BastionId::new();
        let (sender, _) = mpsc::unbounded();
        let path = Arc::new(BastionPath::root());
        let name = "test_name".to_string();
        let child_ref = ChildRef::new(bastion_id, sender, name, path);

        let global_dispatcher = GlobalDispatcher::new();
        let distributor = Distributor::named("test-distributor");
        global_dispatcher
            .register_distributor(&distributor)
            .unwrap();
        global_dispatcher
            .register_recipient(&distributor, child_ref.clone())
            .unwrap();
        global_dispatcher.remove_distributor(&distributor).unwrap();
        // Should maintain the dispatcher because it still has a recipient.
        assert!(!global_dispatcher.distributors.read().unwrap().is_empty());

        global_dispatcher
            .remove_recipient(&[distributor], &child_ref)
            .unwrap();
        global_dispatcher.remove_distributor(&distributor).unwrap();
        // Distributor is now removed because it has no remaining recipients.
        assert!(global_dispatcher.distributors.read().unwrap().is_empty());
    }
}
