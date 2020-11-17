/// Enum that provides information what type of the message
/// being sent through the channel.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MessageType {
    /// A message type that requires sending a confirmation to the
    /// sender after begin the processing stage.
    Ack,
    /// A message that can be broadcasted (e.g. via system dispatchers). This
    /// message type doesn't require to be acked from the receiver's side.
    Broadcast,
    /// A message was sent directly and doesn't require confirmation for the
    /// delivery and being processed.
    Tell,
}
