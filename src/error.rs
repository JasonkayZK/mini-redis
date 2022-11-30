use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MiniRedisServerError {
    #[error("client disconnected")]
    Disconnect(#[from] io::Error),
}

#[derive(Error, Debug)]
pub enum MiniRedisClientError {
    #[error("server cannot connected")]
    Connect(#[from] io::Error),
}
