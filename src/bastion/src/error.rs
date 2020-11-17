use std::result;
use std::time::Duration;

pub type Result<T> = result::Result<T, BastionError>;

#[derive(Debug)]
pub enum BastionError {
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
