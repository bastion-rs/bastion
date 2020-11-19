/// This module contains implementation of the local state for
/// Bastion actors. Each actor hold its own data and doesn't expose
/// it to others, so that it will be possible to do updates in runtime
/// without being affected by other actors or potential data races.
use std::any::{Any, TypeId};
use std::sync::Arc;

use lever::table::lotable::LOTable;

#[derive(Debug, Clone)]
/// A unified storage for actor's data and intended to use
/// only in the context of the single actor.
pub(crate) struct LocalState {
    table: LOTable<TypeId, LocalDataContainer>,
}

#[derive(Debug, Clone)]
/// Wrapper for the `Arc<AtomicBox<Box<T>>` type that provides
/// simpler and easier to use API to developers.
pub struct LocalDataContainer(Arc<Box<dyn Any + Send + Sync + 'static>>);

impl LocalState {
    /// Returns a new instance of local state for actor.
    pub(crate) fn new() -> Self {
        LocalState {
            table: LOTable::new(),
        }
    }

    /// Inserts the given value in the table. If the value
    /// exists, it will be overridden.
    pub fn insert<T: Send + Sync + 'static>(&self, value: T) {
        let container = LocalDataContainer::new(value);
        self.table.insert(TypeId::of::<T>(), container).ok();
    }

    /// Checks the given values is storing in the table.
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        match self.table.get(&TypeId::of::<T>()) {
            Some(_) => true,
            _ => false,
        }
    }

    /// Returns local data container to the caller if it exists.
    pub fn get_container<T: Send + Sync + 'static>(&self) -> Option<LocalDataContainer> {
        self.table
            .get(&TypeId::of::<T>())
            .and_then(|data| Some(data))
    }

    /// Deletes the entry from the table.
    pub fn remove<T: Send + Sync + 'static>(&self) -> bool {
        match self.table.remove(&TypeId::of::<T>()) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

impl LocalDataContainer {
    pub(crate) fn new<T: Send + Sync + 'static>(value: T) -> Self {
        LocalDataContainer(Arc::new(Box::new(value)))
    }

    /// Returns immutable data to the caller.
    pub fn get<'a, T: Send + Sync + 'static>(&'a self) -> Option<&'a T> {
        self.0.downcast_ref()
    }

    /// Returns mutable data to the caller.
    pub fn get_mut<'a, T: Send + Sync + 'static>(&'a mut self) -> Option<&'a mut T> {
        Arc::get_mut(&mut self.0).and_then(|data| data.downcast_mut())
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
        let instance = LocalState::new();

        instance.insert(Data { counter: 0 });
        assert_eq!(instance.contains::<Data>(), true);
    }

    #[test]
    fn test_insert_with_duplicated_data() {
        let instance = LocalState::new();

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
        let instance = LocalState::new();

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
        let instance = LocalState::new();

        let expected = Data { counter: 1 };

        instance.insert(expected.clone());
        assert_eq!(instance.contains::<Data>(), true);

        let result_get = instance.get_container::<Data>();
        assert_eq!(result_get.is_some(), true);

        let mut container = result_get.unwrap();
        let result_mut_data = container.get_mut::<Data>();
        assert_eq!(result_mut_data.is_some(), true);

        let mut_data = result_mut_data.unwrap();
        mut_data.counter += 1;

        let expected_update = Data { counter: 2 };
        let result_updated_data = container.get::<Data>();
        assert_eq!(result_updated_data.is_some(), true);
        assert_eq!(result_updated_data.unwrap(), &expected_update);
    }

    #[test]
    fn test_get_container_returns_none() {
        let instance = LocalState::new();

        let container = instance.get_container::<usize>();
        assert_eq!(container.is_none(), true);
    }

    #[test]
    fn test_remove_returns_true() {
        let instance = LocalState::new();

        instance.insert(Data { counter: 0 });
        assert_eq!(instance.contains::<Data>(), true);

        let is_removed = instance.remove::<Data>();
        assert_eq!(is_removed, true);
    }

    #[test]
    fn test_remove_returns_false() {
        let instance = LocalState::new();

        let is_removed = instance.remove::<usize>();
        assert_eq!(is_removed, true);
    }
}
