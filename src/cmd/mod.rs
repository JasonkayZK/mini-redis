use crate::cmd::ping::Ping;
use crate::cmd::unknown::Unknown;
use crate::connection::connect::Connection;
use crate::connection::frame::Frame;
use crate::error::{MiniRedisConnectionError, MiniRedisParseError};
use crate::server::shutdown::Shutdown;
use crate::storage::db::Db;

mod ping;
mod unknown;

/// Enumeration of supported Redis commands.
///
/// Methods called on `Command` are delegated to the command implementation.
#[derive(Debug)]
pub enum Command {
    // Get(Get),
    // Publish(Publish),
    // Set(Set),
    // Subscribe(Subscribe),
    // Unsubscribe(Unsubscribe),
    Ping(Ping),
    Unknown(Unknown),
}

impl Command {
    /// Parse a command from a received frame.
    ///
    /// The `Frame` must represent a Redis command supported by `mini-redis` and
    /// be the array variant.
    ///
    /// # Returns
    ///
    /// On success, the command value is returned, otherwise, `Err` is returned.
    pub fn from_frame(frame: Frame) -> Result<Command, MiniRedisParseError> {
        todo!()
    }

    /// Apply the command to the specified `Db` instance.
    ///
    /// The response is written to `dst`. This is called by the server in order
    /// to execute a received command.
    /// Apply the command to the specified `Db` instance.
    ///
    /// The response is written to `dst`. This is called by the server in order
    /// to execute a received command.
    pub(crate) async fn apply(
        self,
        db: &Db,
        dst: &mut Connection,
        shutdown: &mut Shutdown,
    ) -> Result<(), MiniRedisConnectionError> {
        use Command::*;

        match self {
            // Get(cmd) => cmd.apply(db, dst).await,
            // Publish(cmd) => cmd.apply(db, dst).await,
            // Set(cmd) => cmd.apply(db, dst).await,
            // Subscribe(cmd) => cmd.apply(db, dst, shutdown).await,
            Ping(cmd) => cmd.apply(dst).await,
            Unknown(cmd) => cmd.apply(dst).await,
            // `Unsubscribe` cannot be applied. It may only be received from the
            // context of a `Subscribe` command.
            // Unsubscribe(_) => Err("`Unsubscribe` is unsupported in this context".into()),
        }
    }

    /// Returns the command name
    pub(crate) fn get_name(&self) -> &str {
        match self {
            // Command::Get(_) => "get",
            // Command::Publish(_) => "pub",
            // Command::Set(_) => "set",
            // Command::Subscribe(_) => "subscribe",
            // Command::Unsubscribe(_) => "unsubscribe",
            Command::Ping(_) => "ping",
            Command::Unknown(cmd) => cmd.get_name(),
        }
    }
}
