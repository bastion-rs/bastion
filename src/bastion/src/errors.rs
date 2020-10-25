use std::result;
use std::time::Duration;

pub type Result<T> = result::Result<T, BError>;

#[derive(Debug)]
pub enum BError {
    Receive(ReceiveError),
    ChanSend(String),
    ChanRecv(String),
    UnackedMessage,
}

#[derive(Debug)]
pub enum ReceiveError {
    Timeout(Duration),
    Other,
}
