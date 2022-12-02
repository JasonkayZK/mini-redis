use bytes::Bytes;

use crate::connection::frame::Frame;
use crate::connection::parse::Parse;
use crate::error::MiniRedisParseError;

/// Unsubscribes the client from one or more channels.
///
/// When no channels are specified, the client is unsubscribed from all the
/// previously subscribed channels.
#[derive(Clone, Debug)]
pub struct Unsubscribe {
    pub(crate) channels: Vec<String>,
}

impl Unsubscribe {
    /// Create a new `Unsubscribe` command with the given `channels`.
    pub(crate) fn new(channels: &[String]) -> Unsubscribe {
        Unsubscribe {
            channels: channels.to_vec(),
        }
    }

    /// Parse a `Unsubscribe` instance from a received frame.
    ///
    /// The `Parse` argument provides a cursor-like API to read fields from the
    /// `Frame`. At this point, the entire frame has already been received from
    /// the socket.
    ///
    /// The `UNSUBSCRIBE` string has already been consumed.
    ///
    /// # Returns
    ///
    /// On success, the `Unsubscribe` value is returned. If the frame is
    /// malformed, `Err` is returned.
    ///
    /// # Format
    ///
    /// Expects an array frame containing at least one entry.
    ///
    /// ```text
    /// UNSUBSCRIBE [channel [channel ...]]
    /// ```
    pub(crate) fn parse_frames(parse: &mut Parse) -> Result<Unsubscribe, MiniRedisParseError> {
        // There may be no channels listed, so start with an empty vec.
        let mut channels = vec![];

        // Each entry in the frame must be a string or the frame is malformed.
        // Once all values in the frame have been consumed, the command is fully
        // parsed.
        loop {
            match parse.next_string() {
                // A string has been consumed from the `parse`, push it into the
                // list of channels to unsubscribe from.
                Ok(s) => channels.push(s),
                // The `EndOfStream` error indicates there is no further data to
                // parse.
                Err(MiniRedisParseError::EndOfStream) => break,
                // All other errors are bubbled up, resulting in the connection
                // being terminated.
                Err(err) => return Err(err),
            }
        }

        Ok(Unsubscribe { channels })
    }

    /// Converts the command into an equivalent `Frame`.
    ///
    /// This is called by the client when encoding an `Unsubscribe` command to
    /// send to the server.
    pub(crate) fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("unsubscribe".as_bytes()));

        for channel in self.channels {
            frame.push_bulk(Bytes::from(channel.into_bytes()));
        }

        frame
    }
}

/// Creates the response to an unsubscribe request.
pub(crate) fn make_unsubscribe_frame(channel_name: String, num_subs: usize) -> Frame {
    let mut response = Frame::array();
    response.push_bulk(Bytes::from_static(b"unsubscribe"));
    response.push_bulk(Bytes::from(channel_name));
    response.push_int(num_subs as u64);
    response
}
