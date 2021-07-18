use std::result;

use thiserror::Error;

pub type Result<T> = result::Result<T, BastionError>;

#[derive(Error, Debug)]
pub enum BastionError {
    #[error("The message cannot be sent via the channel. Reason: {0}")]
    ChanSend(String),
    #[error("The message cannot be received from the channel. Reason: {0}")]
    ChanRecv(String),
    #[error("The actors channel is empty.")]
    EmptyChannel,
}
