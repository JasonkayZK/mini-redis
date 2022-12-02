use log::debug;

use crate::connection::connect::Connection;
use crate::connection::frame::Frame;
use crate::error::MiniRedisConnectionError;

/// Represents an "unknown" command. This is not a real `Redis` command.
#[derive(Debug)]
pub struct Unknown {
    command_name: String,
}

impl Unknown {
    /// Create a new `Unknown` command which responds to unknown commands
    /// issued by clients
    pub(crate) fn new(key: impl ToString) -> Unknown {
        Unknown {
            command_name: key.to_string(),
        }
    }

    /// Returns the command name
    pub(crate) fn get_name(&self) -> &str {
        &self.command_name
    }

    /// Responds to the client, indicating the command is not recognized.
    ///
    /// This usually means the command is not yet implemented by `mini-redis`.
    pub(crate) async fn apply(self, dst: &mut Connection) -> Result<(), MiniRedisConnectionError> {
        let response = Frame::Error(format!("err unknown command '{}'", self.command_name));

        debug!("apply unknown command resp: {:?}", response);

        dst.write_frame(&response).await?;
        Ok(())
    }
}
