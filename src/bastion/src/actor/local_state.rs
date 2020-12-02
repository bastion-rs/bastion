/// This module contains implementation of the local state for
/// Bastion actors. Each actor hold its own data and doesn't expose
/// it to others, so that it will be possible to do updates in runtime
/// without being affected by other actors or potential data races.
use std::any::{Any, TypeId};
use std::sync::Arc;

use lever::table::lotable::LOTable;
use std::borrow::BorrowMut;
use std::borrow::Borrow;
use std::convert::identity;
use std::collections::HashMap;

#[derive(Debug)]
/// A unified storage for actor's data and intended to use
/// only in the context of the single actor.
pub(crate) struct LocalState {
    table: HashMap<TypeId, LocalDataContainer>,
}

#[derive(Debug)]
#[repr(transparent)]
/// Transparent type for the `Box<dyn Any + Send + Sync>` type that provides
/// simpler and easier to use API to developers.
pub struct LocalDataContainer(Box<dyn Any + Send + Sync>);

impl LocalState {
    /// Returns a new instance of local state for actor.
    pub(crate) fn new() -> Self {
        LocalState {
            table: HashMap::with_capacity(1 << 10),
        }
    }

    /// Inserts the given value in the table. If the value
    /// exists, it will be overridden.
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) {
        let container = LocalDataContainer::new(value);
        self.table.insert(TypeId::of::<T>(), container);
    }

    /// Checks the given values is storing in the table.
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.table.contains_key(&TypeId::of::<T>())
    }

    /// Runs given closure on the state
    pub fn with_state<F, R, T: Send + Sync + 'static>(&self, f: F) -> Option<R>
    where
        F: FnOnce(Option<&T>) -> Option<R>
    {
        self.table.get(&TypeId::of::<T>())
            .and_then(|e| f(e.get()))
    }

    /// Deletes the entry from the table.
    pub fn remove<T: Send + Sync + 'static>(&mut self) -> bool {
        self.table.remove(&TypeId::of::<T>()).is_some()
    }

    /// Returns immutable data to the caller.
    pub fn get<'a, T: Send + Sync + 'static>(&'a self) -> Option<&'a T> {
        self.get_container::<T>().and_then(|e| e.0.downcast_ref())
    }

    /// Returns mutable data to the caller.
    pub fn get_mut<'a, T: Send + Sync + 'static>(&'a mut self) -> Option<&'a mut T> {
        self.get_container_mut::<T>().and_then(|e| e.0.downcast_mut())
    }

    /// Returns local data container to the caller if it exists.
    fn get_container<T: Send + Sync + 'static>(&self) -> Option<&LocalDataContainer> {
        self.table
            .get(&TypeId::of::<T>())
    }

    /// Returns local data container to the caller if it exists.
    fn get_container_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut LocalDataContainer> {
        self.table
            .get_mut(&TypeId::of::<T>())
    }
}

impl LocalDataContainer {
    pub(crate) fn new<T: Send + Sync + 'static>(value: T) -> Self {
        LocalDataContainer(Box::new(value))
    }

    /// Returns immutable data to the caller.
    fn get<'a, T: Send + Sync + 'static>(&'a self) -> Option<&'a T> {
        self.0.downcast_ref()
    }

    /// Returns mutable data to the caller.
    fn get_mut<'a, T: Send + Sync + 'static>(&'a mut self) -> Option<&'a mut T> {
        self.0.downcast_mut()
    }
}

#[cfg(test)]
mod tests {
    use crate::actor::local_state::LocalState;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct Data {
        counter: u64,
    }

    #[test]
    fn test_insert() {
        let mut instance = LocalState::new();

        instance.insert(Data { counter: 0 });
        assert_eq!(instance.contains::<Data>(), true);
    }

    #[test]
    fn test_insert_with_duplicated_data() {
        let mut instance = LocalState::new();

        instance.insert(Data { counter: 0 });
        assert_eq!(instance.contains::<Data>(), true);

        instance.insert(Data { counter: 1 });
        assert_eq!(instance.contains::<Data>(), true);
    }

    #[test]
    fn test_contains_returns_false() {
        let instance = LocalState::new();

        assert_eq!(instance.contains::<usize>(), false);
    }

    #[test]
    fn test_get_container() {
        let mut instance = LocalState::new();

        let expected = Data { counter: 1 };

        instance.insert(expected.clone());
        assert_eq!(instance.contains::<Data>(), true);

        let result_get = instance.get_container::<Data>();
        assert_eq!(result_get.is_some(), true);

        let container = result_get.unwrap();
        let data = container.get::<Data>();
        assert_eq!(data.is_some(), true);
        assert_eq!(data.unwrap(), &expected);
    }

    #[test]
    fn test_get_container_with_mutable_data() {
        let mut instance = LocalState::new();

        let mut expected = Data { counter: 1 };

        instance.insert(expected.clone());
        assert_eq!(instance.contains::<Data>(), true);

        // Get the current snapshot of data
        let mut data = instance.get_mut::<Data>();
        assert_eq!(data.is_some(), true);
        assert_eq!(data, Some(&mut expected));

        data.map(|d| {
            d.counter += 1;
            d
        });

        // Replace the data onto new one
        let expected_update = Data { counter: 2 };

        // Check the data was updated
        let result_updated_data = instance.get::<Data>();
        assert_eq!(result_updated_data.is_some(), true);
        assert_eq!(result_updated_data.unwrap(), &expected_update);
    }

    #[test]
    fn test_get_container_returns_none() {
        let mut instance = LocalState::new();

        let container = instance.get_container::<usize>();
        assert_eq!(container.is_none(), true);
    }

    #[test]
    fn test_remove_returns_true() {
        let mut instance = LocalState::new();

        instance.insert(Data { counter: 0 });
        assert_eq!(instance.contains::<Data>(), true);

        let is_removed = instance.remove::<Data>();
        assert_eq!(is_removed, true);
    }

    #[test]
    fn test_remove_returns_false() {
        let mut instance = LocalState::new();

        let is_removed = instance.remove::<usize>();
        assert_eq!(is_removed, false);
    }
}
