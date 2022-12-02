//! Provides a type representing a Redis protocol frame as well as utilities for
//! parsing frames from a byte array.
//!
//! Redis serialization protocol (RESP) specification:
//!  https://redis.io/docs/reference/protocol-spec/

use std::convert::TryInto;
use std::fmt;
use std::io::Cursor;

use bytes::{Buf, Bytes};

use crate::error::MiniRedisParseError;

/// A frame in the Redis protocol.
#[derive(Clone, Debug)]
pub enum Frame {
    Simple(String),
    Error(String),
    Integer(u64),
    Bulk(Bytes),
    Null,
    Array(Vec<Frame>),
}

impl PartialEq<&str> for Frame {
    fn eq(&self, other: &&str) -> bool {
        match self {
            Frame::Simple(s) => s.eq(other),
            Frame::Bulk(s) => s.eq(other),
            _ => false,
        }
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use std::str;

        match self {
            Frame::Simple(response) => response.fmt(fmt),
            Frame::Error(msg) => write!(fmt, "error: {}", msg),
            Frame::Integer(num) => num.fmt(fmt),
            Frame::Bulk(msg) => match str::from_utf8(msg) {
                Ok(string) => string.fmt(fmt),
                Err(_) => write!(fmt, "{:?}", msg),
            },
            Frame::Null => "(nil)".fmt(fmt),
            Frame::Array(parts) => {
                for (i, part) in parts.iter().enumerate() {
                    if i > 0 {
                        write!(fmt, " ")?;
                        part.fmt(fmt)?;
                    }
                }

                Ok(())
            }
        }
    }
}

impl Frame {
    /// Returns an empty array
    pub(crate) fn array() -> Frame {
        Frame::Array(vec![])
    }

    /// Push a "bulk" frame into the array. `self` must be an Array frame.
    pub(crate) fn push_bulk(&mut self, bytes: Bytes) -> Result<(), MiniRedisParseError> {
        match self {
            Frame::Array(vec) => {
                vec.push(Frame::Bulk(bytes));
                Ok(())
            }
            _ => Err(MiniRedisParseError::ParseArrayFrame),
        }
    }

    /// Push an "integer" frame into the array. `self` must be an Array frame.
    pub(crate) fn push_int(&mut self, value: u64) -> Result<(), MiniRedisParseError> {
        match self {
            Frame::Array(vec) => {
                vec.push(Frame::Integer(value));
                Ok(())
            }
            _ => Err(MiniRedisParseError::ParseArrayFrame),
        }
    }

    /// Checks if an entire message can be decoded from `src`
    ///
    /// Redis serialization protocol (RESP) specification:
    ///  https://redis.io/docs/reference/protocol-spec/
    pub fn check(src: &mut Cursor<&[u8]>) -> Result<(), MiniRedisParseError> {
        match get_u8(src)? {
            b'+' => {
                get_line(src)?;
                Ok(())
            }
            b'-' => {
                get_line(src)?;
                Ok(())
            }
            b':' => {
                let _ = get_decimal(src)?;
                Ok(())
            }
            b'$' => {
                if b'-' == peek_u8(src)? {
                    // Skip '-1\r\n'
                    skip(src, 4)
                } else {
                    // Read the bulk string
                    let len: usize = get_decimal(src)?.try_into()?;

                    // skip that number of bytes + 2 (\r\n).
                    skip(src, len + 2)
                }
            }
            b'*' => {
                let len = get_decimal(src)?;

                for _ in 0..len {
                    Frame::check(src)?;
                }

                Ok(())
            }
            actual => Err(MiniRedisParseError::Parse(format!(
                "protocol error; invalid frame type byte `{}`",
                actual
            ))),
        }
    }

    /// The message has already been validated with `check`, so parse the bytes to Frame
    ///
    /// Redis serialization protocol (RESP) specification:
    ///  https://redis.io/docs/reference/protocol-spec/
    pub fn parse(src: &mut Cursor<&[u8]>) -> Result<Frame, MiniRedisParseError> {
        match get_u8(src)? {
            b'+' => {
                // Read the line and convert it to `Vec<u8>`
                let line = get_line(src)?.to_vec();

                // Convert the line to a String
                let string = String::from_utf8(line)?;

                Ok(Frame::Simple(string))
            }
            b'-' => {
                // Read the line and convert it to `Vec<u8>`
                let line = get_line(src)?.to_vec();

                // Convert the line to a String
                let string = String::from_utf8(line)?;

                Ok(Frame::Error(string))
            }
            b':' => {
                let len = get_decimal(src)?;
                Ok(Frame::Integer(len))
            }
            b'$' => {
                if b'-' == peek_u8(src)? {
                    let line = get_line(src)?;

                    if line != b"-1" {
                        return Err(MiniRedisParseError::Parse(
                            "protocol error; invalid frame format".into(),
                        ));
                    }

                    Ok(Frame::Null)
                } else {
                    // Read the bulk string
                    let len = get_decimal(src)?.try_into()?;
                    let n = len + 2;

                    if src.remaining() < n {
                        return Err(MiniRedisParseError::Incomplete);
                    }

                    let data = Bytes::copy_from_slice(&src.chunk()[..len]);

                    // skip that number of bytes + 2 (\r\n).
                    skip(src, n)?;

                    Ok(Frame::Bulk(data))
                }
            }
            b'*' => {
                let len = get_decimal(src)?.try_into()?;
                let mut out = Vec::with_capacity(len);

                for _ in 0..len {
                    out.push(Frame::parse(src)?);
                }

                Ok(Frame::Array(out))
            }
            _ => Err(MiniRedisParseError::Unimplemented),
        }
    }
}

fn skip(src: &mut Cursor<&[u8]>, n: usize) -> Result<(), MiniRedisParseError> {
    if src.remaining() < n {
        return Err(MiniRedisParseError::Incomplete);
    }

    src.advance(n);
    Ok(())
}

fn peek_u8(src: &mut Cursor<&[u8]>) -> Result<u8, MiniRedisParseError> {
    if !src.has_remaining() {
        return Err(MiniRedisParseError::Incomplete);
    }

    Ok(src.chunk()[0])
}

fn get_u8(src: &mut Cursor<&[u8]>) -> Result<u8, MiniRedisParseError> {
    if !src.has_remaining() {
        return Err(MiniRedisParseError::Incomplete);
    }

    Ok(src.get_u8())
}

/// Read a new-line terminated decimal
fn get_decimal(src: &mut Cursor<&[u8]>) -> Result<u64, MiniRedisParseError> {
    use atoi::atoi;

    let line = get_line(src)?;

    atoi::<u64>(line).ok_or_else(|| {
        MiniRedisParseError::Parse("protocol error; invalid frame format to get decimal".into())
    })
}

/// Find a line in a frame
fn get_line<'a>(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], MiniRedisParseError> {
    // Scan the bytes directly
    let start = src.position() as usize;
    // Scan to the second to last byte
    let end = src.get_ref().len() - 1;

    for i in start..end {
        if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
            // We found a line, update the position to be *after* the \n
            src.set_position((i + 2) as u64);

            // Return the line
            return Ok(&src.get_ref()[start..i]);
        }
    }

    Err(MiniRedisParseError::Incomplete)
}
