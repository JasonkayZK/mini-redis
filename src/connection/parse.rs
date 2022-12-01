use std::vec;

use bytes::Bytes;

use crate::connection::frame::Frame;
use crate::error::MiniRedisParseError;

/// Utility for parsing a command
///
/// Commands are represented as array frames. Each entry in the frame is a
/// "token". A `Parse` is initialized with the array frame and provides a
/// cursor-like API. Each command struct includes a `parse_frame` method that
/// uses a `Parse` to extract its fields.
#[derive(Debug)]
pub(crate) struct Parse {
    /// Array frame iterator.
    parts: vec::IntoIter<Frame>,
}

impl Parse {
    /// Create a new `Parse` to parse the contents of `frame`.
    ///
    /// Returns `Err` if `frame` is not an array frame.
    pub(crate) fn new(frame: Frame) -> Result<Parse, MiniRedisParseError> {
        let array = match frame {
            Frame::Array(array) => array,
            frame => {
                return Err(MiniRedisParseError::Parse(format!(
                    "protocol error; expected array, got {:?}",
                    frame
                )))
            }
        };

        Ok(Parse {
            parts: array.into_iter(),
        })
    }

    /// Return the next entry. Array frames are arrays of frames, so the next
    /// entry is a frame.
    fn next(&mut self) -> Result<Frame, MiniRedisParseError> {
        self.parts.next().ok_or(MiniRedisParseError::EndOfStream)
    }

    /// Return the next entry as a string.
    ///
    /// If the next entry cannot be represented as a String, then an error is returned.
    pub(crate) fn next_string(&mut self) -> Result<String, MiniRedisParseError> {
        match self.next()? {
            // Both `Simple` and `Bulk` representation may be strings. Strings
            // are parsed to UTF-8.
            //
            // While errors are stored as strings, they are considered separate
            // types.
            Frame::Simple(s) => Ok(s),
            Frame::Bulk(data) => std::str::from_utf8(&data[..])
                .map(|s| s.to_string())
                .map_err(|_| MiniRedisParseError::Parse("protocol error; invalid string".into())),
            frame => Err(MiniRedisParseError::Parse(format!(
                "protocol error; expected simple frame or bulk frame, got {:?}",
                frame
            ))),
        }
    }

    /// Return the next entry as raw bytes.
    ///
    /// If the next entry cannot be represented as raw bytes, an error is
    /// returned.
    pub(crate) fn next_bytes(&mut self) -> Result<Bytes, MiniRedisParseError> {
        match self.next()? {
            // Both `Simple` and `Bulk` representation may be raw bytes.
            //
            // Although errors are stored as strings and could be represented as
            // raw bytes, they are considered separate types.
            Frame::Simple(s) => Ok(Bytes::from(s.into_bytes())),
            Frame::Bulk(data) => Ok(data),
            frame => Err(MiniRedisParseError::Parse(format!(
                "protocol error; expected simple frame or bulk frame, got {:?}",
                frame
            ))),
        }
    }

    /// Return the next entry as an integer.
    ///
    /// This includes `Simple`, `Bulk`, and `Integer` frame types. `Simple` and
    /// `Bulk` frame types are parsed.
    ///
    /// If the next entry cannot be represented as an integer, then an error is
    /// returned.
    pub(crate) fn next_int(&mut self) -> Result<u64, MiniRedisParseError> {
        use atoi::atoi;

        match self.next()? {
            // An integer frame type is already stored as an integer.
            Frame::Integer(v) => Ok(v),
            // Simple and bulk frames must be parsed as integers. If the parsing
            // fails, an error is returned.
            Frame::Simple(data) => atoi::<u64>(data.as_bytes())
                .ok_or_else(|| MiniRedisParseError::Parse("protocol error; invalid number".into())),
            Frame::Bulk(data) => atoi::<u64>(&data)
                .ok_or_else(|| MiniRedisParseError::Parse("protocol error; invalid number".into())),
            frame => Err(MiniRedisParseError::Parse(format!(
                "protocol error; expected int frame but got {:?}",
                frame
            ))),
        }
    }

    /// Ensure there are no more entries in the array
    pub(crate) fn finish(&mut self) -> Result<(), MiniRedisParseError> {
        if self.parts.next().is_none() {
            Ok(())
        } else {
            Err(MiniRedisParseError::Parse(
                "protocol error; expected end of frame, but there was more".into(),
            ))
        }
    }
}
