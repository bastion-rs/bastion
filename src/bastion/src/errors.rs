use std::time::Duration;

#[derive(Debug)]
pub enum ReceiveError {
    Timeout(Duration),
    Other,
}
