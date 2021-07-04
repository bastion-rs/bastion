use std::fmt::Debug;

/// A trait that message needs to implement for typed actors (it
/// forces message to implement the following traits: [`Any`],
/// [`Send`] and [`Debug`]).
///
/// [`Any`]: https://doc.rust-lang.org/std/any/trait.Any.html
/// [`Send`]: https://doc.rust-lang.org/std/marker/trait.Send.html
/// [`Debug`]: https://doc.rust-lang.org/std/fmt/trait.Debug.html
pub trait Message: Send + Debug + 'static {}
impl<T> Message for T where T: ?Sized + Send + Debug + 'static {}

/// Enum that provides information what type of the message
/// being sent through the channel.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MessageType {
    /// A message type that requires sending a confirmation to the
    /// sender after begin the processing stage.
    Ask,
    /// A message that can be broadcasted (e.g. via system dispatchers). This
    /// message type doesn't require to be acked from the receiver's side.
    Broadcast,
    /// A message was sent directly and doesn't require confirmation for the
    /// delivery and being processed.
    Tell,
}
