use std::fmt::Debug;

/// A trait that message needs to implement for typed actors (it is
/// already automatically implemented but forces message to
/// implement the following traits: [`Any`], [`Send`],
/// [`Sync`] and [`Debug`]).
///
/// [`Any`]: https://doc.rust-lang.org/std/any/trait.Any.html
/// [`Send`]: https://doc.rust-lang.org/std/marker/trait.Send.html
/// [`Sync`]: https://doc.rust-lang.org/std/marker/trait.Sync.html
/// [`Debug`]: https://doc.rust-lang.org/std/fmt/trait.Debug.html
pub trait TypedMessage: Clone + Send + Debug + 'static {}
impl<T> TypedMessage for T where T: Clone + Send + Debug + 'static {}
