use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MiniRedisServerError {
    #[error(transparent)]
    IoError(#[from] io::Error),

    #[error(transparent)]
    Connect(#[from] MiniRedisConnectionError),

    #[error(transparent)]
    Parse(#[from] MiniRedisParseError),
}

#[derive(Error, Debug)]
pub enum MiniRedisClientError {
    #[error(transparent)]
    Connect(#[from] MiniRedisConnectionError),

    #[error(transparent)]
    Parse(#[from] MiniRedisParseError),
}

/// Error encountered while parsing a frame.
///
/// Only `EndOfStream` errors are handled at runtime. All other errors result in
/// the connection being terminated.
#[derive(Error, Debug)]
pub enum MiniRedisParseError {
    #[error("invalid message encoding, parse failed")]
    Parse(String),

    /// Attempting to extract a value failed due to the frame being fully
    /// consumed.
    #[error("protocol error; unexpected end of stream")]
    EndOfStream,

    #[error("not enough data is available to parse a message")]
    Incomplete,

    #[error("unimplemented command")]
    Unimplemented,

    #[error("not an array frame")]
    ParseArrayFrame,

    #[error(transparent)]
    ParseInt(#[from] std::num::TryFromIntError),
    #[error(transparent)]
    ParseUtf8(#[from] std::string::FromUtf8Error),
}

#[derive(Error, Debug)]
pub enum MiniRedisConnectionError {
    #[error("connection reset by peer")]
    Disconnect,

    #[error(transparent)]
    ParseFrame(#[from] MiniRedisParseError),

    #[error(transparent)]
    IoError(#[from] io::Error),

    #[error("command execute error")]
    CommandExecute(String),
}
