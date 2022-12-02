use std::num::ParseIntError;
use std::time::Duration;

use bytes::Bytes;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum Command {
    Ping {
        /// Message to ping
        msg: Option<String>,
    },
    /// Get the value of key.
    Get {
        /// Name of key to get
        key: String,
    },
    /// Set key to hold the string value.
    Set {
        /// Name of key to set
        key: String,

        /// Value to set.
        #[clap(parse(from_str = bytes_from_str))]
        value: Bytes,

        /// Expire the value after specified amount of time
        #[clap(parse(try_from_str = duration_from_ms_str))]
        expires: Option<Duration>,
    },
    ///  Publisher to send a message to a specific channel.
    Publish {
        /// Name of channel
        channel: String,

        #[clap(parse(from_str = bytes_from_str))]
        /// Message to publish
        message: Bytes,
    },
    /// Subscribe a client to a specific channel or channels.
    Subscribe {
        /// Specific channel or channels
        channels: Vec<String>,
    },
}

fn duration_from_ms_str(src: &str) -> Result<Duration, ParseIntError> {
    let ms = src.parse::<u64>()?;
    Ok(Duration::from_millis(ms))
}

fn bytes_from_str(src: &str) -> Bytes {
    Bytes::from(src.to_string())
}
