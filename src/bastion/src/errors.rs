use std::result;
use std::time::Duration;
use crate::envelope::Envelope;
use crate::message::TypedMessage;

pub type Result<T> = result::Result<T, BError>;

#[derive(Debug)]
pub enum BError {
    Receive(ReceiveError),
    ChanSend(String),
    ChanRecv(String)
}

#[derive(Debug)]
pub enum ReceiveError {
    Timeout(Duration),
    Other,
}
