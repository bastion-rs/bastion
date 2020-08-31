#[derive(PartialEq, PartialOrd)]
pub enum MailboxState {
    /// Mailbox has been scheduled
    Scheduled,
    /// Message has been sent to destination
    Sent,
    /// Ack has currently been awaited
    Awaiting
}
