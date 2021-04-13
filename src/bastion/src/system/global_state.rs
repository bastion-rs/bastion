/// This module contains implementation of the global state that
/// available to all actors in runtime. To provide safety and avoid
/// data races, the implementation is heavily relies on software
/// transaction memory (or shortly STM) mechanisms to eliminate any
/// potential data races and provide consistency across actors.
use std::any::{Any, TypeId};
use std::ops::Deref;
use std::sync::Arc;

use lever::sync::atomics::AtomicBox;
use lever::table::lotable::LOTable;

use crate::error::{BastionError, Result};

#[derive(Debug)]
pub struct GlobalState {
    table: LOTable<TypeId, GlobalDataContainer>,
}

#[derive(Debug, Clone)]
/// A container for user-defined types.
struct GlobalDataContainer(Arc<dyn Any + Send + Sync>);

impl GlobalState {
    /// Returns a new instance of global state.
    pub(crate) fn new() -> Self {
        GlobalState {
            table: LOTable::new(),
        }
    }

    /// Inserts the given value in the global state. If the value
    /// exists, it will be overridden.
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) -> bool {
        let container = GlobalDataContainer::new(value);
        self.table
            .insert(TypeId::of::<T>(), container)
            .ok()
            .is_some()
    }

    /// Returns the requested data type to the caller.
    pub fn read<T: Send + Sync + 'static>(&mut self) -> Option<Arc<T>> {
        self.table
            .get(&TypeId::of::<Arc<T>>())
            .and_then(|gdc| gdc.read())
    }

    /// Checks the given values is storing in the global state.
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.table.contains_key(&TypeId::of::<T>())
    }

    /// Deletes the entry from the global state.
    pub fn remove<T: Send + Sync + 'static>(&mut self) -> bool {
        match self.table.remove(&TypeId::of::<T>()) {
            Ok(entry) => entry.is_some(),
            Err(_) => false,
        }
    }
}

impl GlobalDataContainer {
    pub fn new<T: Send + Sync + 'static>(value: T) -> Self {
        GlobalDataContainer(Arc::new(AtomicBox::new(Box::new(value))))
    }

    pub fn read<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.0.downcast_ref::<Arc<T>>().map(Arc::clone)
    }
}

#[cfg(test)]
mod tests {
    use crate::system::global_state::GlobalState;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestData {
        counter: u64,
    }

    #[test]
    fn test_insert() {
        let mut instance = GlobalState::new();
        let test_data = TestData { counter: 0 };

        instance.insert(test_data.clone());
        assert_eq!(instance.contains::<TestData>(), true);
    }

    #[test]
    fn test_insert_with_overriding_data() {
        let mut instance = GlobalState::new();

        let first_insert_data = TestData { counter: 0 };
        instance.insert(first_insert_data.clone());
        assert_eq!(instance.contains::<TestData>(), true);

        let second_insert_data = TestData { counter: 1 };
        instance.insert(second_insert_data.clone());
        assert_eq!(instance.contains::<TestData>(), true);
    }

    #[test]
    fn test_contains_returns_true() {
        let mut instance = GlobalState::new();
        assert_eq!(instance.contains::<TestData>(), false);

        instance.insert(TestData { counter: 0 });
        assert_eq!(instance.contains::<TestData>(), true);
    }

    #[test]
    fn test_contains_returns_false() {
        let instance = GlobalState::new();

        assert_eq!(instance.contains::<usize>(), false);
    }

    #[test]
    fn test_remove_returns_true() {
        let mut instance = GlobalState::new();

        instance.insert(TestData { counter: 0 });
        assert_eq!(instance.contains::<TestData>(), true);

        let is_removed = instance.remove::<TestData>();
        assert_eq!(is_removed, true);
    }

    #[test]
    fn test_remove_returns_false() {
        let mut instance = GlobalState::new();

        let is_removed = instance.remove::<usize>();
        assert_eq!(is_removed, false);
    }
}
