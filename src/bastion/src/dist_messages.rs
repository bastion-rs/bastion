use serde::{Serialize, Deserialize};

// TODO: These are not ready yet. Probably these are going to change since some work is done by Artillery.

/// Client request
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "c", content="d")]
enum Request {
    Handshake(String),
    Ping,
    Pong,
    /// Message(msg_id, type_id, ver, payload)
    Message(u64, String, String, String),
}

/// Server response
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag="c", content="d")]
enum Response {
    Handshake,
    Ping,
    Pong,
    /// Announce supported message types
    Supported(Vec<String>),
    /// Response(msg_id, payload)
    Result(u64, String),
    /// Error(msg_id, error-code)
    Error(u64, u16),
}
