use std::sync::Arc;
/// This module contains implementation of the global state that
/// available to all actors in runtime. To provide safety and avoid
/// data races, the implementation is heavily relies on software
/// transaction memory (or shortly STM) mechanisms to eliminate any
/// potential data races and provide consistency across actors.
use std::{
    any::{Any, TypeId},
    sync::RwLock,
};
use std::{collections::hash_map::Entry, ops::Deref};

use lever::sync::atomics::AtomicBox;
use lever::table::lotable::LOTable;
use lightproc::proc_state::AsAny;
use std::collections::HashMap;
#[derive(Debug, Clone)]
pub struct GlobalState {
    table: Arc<RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>>, // todo: remove the arc<rwlock< once we figure it out
}

impl GlobalState {
    /// Returns a new instance of global state.
    pub(crate) fn new() -> Self {
        GlobalState {
            table: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Inserts the given value in the global state. If the value
    /// exists, it will be overridden.
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) -> bool {
        self.table
            .write()
            .unwrap()
            .insert(
                TypeId::of::<T>(),
                Arc::new(value) as Arc<dyn Any + Send + Sync>,
            )
            .is_some()
    }

    /// Invokes a function with the requested data type.
    pub fn read<T: Send + Sync + 'static, O>(&self, f: impl FnOnce(Option<&T>) -> O) -> O {
        let table = self.table.read().unwrap();

        let data = table.get(&TypeId::of::<T>()).map(|value| {
            // This call to unwrap is fine because we always insert data of
            // type T in the slot of TypeId::of<T>.
            value.downcast_ref().unwrap()
        });

        f(data)
    }

    /// Invokes a function with the requested data type.
    pub fn write<T: std::fmt::Debug + Send + Sync + 'static, F>(&mut self, f: F)
    where
        F: Fn(Option<&T>) -> Option<T>,
    {
        let mut hm = self.table.write().unwrap();
        let stuff_to_insert = match hm.entry(TypeId::of::<T>()) {
            Entry::Occupied(data) => f(data.get().downcast_ref()),
            Entry::Vacant(_) => f(None),
        };

        if let Some(stuff) = stuff_to_insert {
            hm.insert(
                TypeId::of::<T>(),
                Arc::new(stuff) as Arc<dyn Any + Send + Sync>,
            );
        } else {
            hm.remove(&TypeId::of::<T>());
        };
    }

    /// Checks the given values is storing in the global state.
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.table.read().unwrap().contains_key(&TypeId::of::<T>())
    }

    /// Deletes the entry from the global state.
    pub fn remove<T: Send + Sync + 'static>(&mut self) -> bool {
        self.table
            .write()
            .unwrap()
            .remove(&TypeId::of::<T>())
            .is_some()
    }
}

#[cfg(test)]
mod tests {
    use crate::global_state::GlobalState;

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

    #[test]
    fn test_write_read() {
        let mut instance = GlobalState::new();

        #[derive(Debug, PartialEq, Clone)]
        struct Hello {
            foo: bool,
            bar: usize,
        }

        let expected = Hello { foo: true, bar: 42 };

        instance.insert(expected.clone());

        instance.read(|actual: Option<&Hello>| {
            assert_eq!(&expected, actual.unwrap());
        });

        let expected_updated = Hello {
            foo: false,
            bar: 43,
        };

        instance.write::<Hello, _>(|maybe_to_update| {
            let to_update = maybe_to_update.unwrap();

            let updated = Hello {
                foo: !to_update.foo,
                bar: to_update.bar + 1,
            };

            Some(updated)
        });

        instance.read(|updated: Option<&Hello>| {
            let updated = updated.unwrap();
            assert_eq!(updated, &expected_updated);
        });
    }
}
