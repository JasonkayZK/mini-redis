use async_stream::try_stream;
use bytes::Bytes;
use log::{debug, error};
use tokio_stream::Stream;

use crate::client::cli::Client;
use crate::cmd::unsubscribe::Unsubscribe;
use crate::connection::frame::Frame;
use crate::error::MiniRedisConnectionError;

/// A client that has entered pub/sub mode.
///
/// Once clients subscribe to a channel, they may only perform pub/sub related
/// commands. The `Client` type is transitioned to a `Subscriber` type in order
/// to prevent non-pub/sub methods from being called.
pub struct Subscriber {
    /// The subscribed client.
    pub(crate) client: Client,

    /// The set of channels to which the `Subscriber` is currently subscribed.
    pub(crate) subscribed_channels: Vec<String>,
}

/// A message received on a subscribed channel.
#[derive(Debug, Clone)]
pub struct Message {
    pub channel: String,
    pub content: Bytes,
}

impl Subscriber {
    /// Subscribe to a list of new channels
    pub async fn subscribe(&mut self, channels: &[String]) -> Result<(), MiniRedisConnectionError> {
        // Issue the subscribe command
        self.client.subscribe_cmd(channels).await?;

        // Update the set of subscribed channels.
        self.subscribed_channels
            .extend(channels.iter().map(Clone::clone));

        Ok(())
    }

    /// Returns the set of channels currently subscribed to.
    pub fn get_subscribed(&self) -> &[String] {
        &self.subscribed_channels
    }

    /// Receive the next message published on a subscribed channel, waiting if
    /// necessary.
    ///
    /// `None` indicates the subscription has been terminated.
    pub async fn next_message(&mut self) -> Result<Option<Message>, MiniRedisConnectionError> {
        match self.client.connection.read_frame().await? {
            Some(frame) => {
                debug!("subscribe received next message: {:?}", frame);

                match frame {
                    Frame::Array(ref frame) => match frame.as_slice() {
                        [message, channel, content] if *message == "message" => Ok(Some(Message {
                            channel: channel.to_string(),
                            content: Bytes::from(content.to_string()),
                        })),
                        _ => {
                            error!("invalid message, frame: {:?}", frame);
                            Err(MiniRedisConnectionError::InvalidFrameType)
                        }
                    },
                    frame => Err(MiniRedisConnectionError::CommandExecute(frame.to_string())),
                }
            }
            None => Ok(None),
        }
    }

    /// Convert the subscriber into a `Stream` yielding new messages published
    /// on subscribed channels.
    ///
    /// `Subscriber` does not implement stream itself as doing so with safe code
    /// is non trivial. The usage of async/await would require a manual Stream
    /// implementation to use `unsafe` code. Instead, a conversion function is
    /// provided and the returned stream is implemented with the help of the
    /// `async-stream` crate.
    pub fn into_stream(mut self) -> impl Stream<Item = Result<Message, MiniRedisConnectionError>> {
        // Uses the `try_stream` macro from the `async-stream` crate. Generators
        // are not stable in Rust. The crate uses a macro to simulate generators
        // on top of async/await. There are limitations, so read the
        // documentation there.
        try_stream! {
            while let Some(message) = self.next_message().await? {
                yield message;
            }
        }
    }

    /// Unsubscribe to a list of new channels
    pub async fn unsubscribe(
        &mut self,
        channels: &[String],
    ) -> Result<(), MiniRedisConnectionError> {
        let frame = Unsubscribe::new(&channels).into_frame();

        debug!("unsubscribe command: {:?}", frame);

        // Write the frame to the socket
        self.client.connection.write_frame(&frame).await?;

        // if the input channel list is empty, server acknowledges as unsubscribing
        // from all subscribed channels, so we assert that the unsubscribe list received
        // matches the client subscribed one
        let num = if channels.is_empty() {
            self.subscribed_channels.len()
        } else {
            channels.len()
        };

        // Read the response
        for _ in 0..num {
            let response = self.client.read_response().await?;

            match response {
                Frame::Array(ref frame) => match frame.as_slice() {
                    [unsubscribe, channel, ..] if *unsubscribe == "unsubscribe" => {
                        let len = self.subscribed_channels.len();

                        if len == 0 {
                            // There must be at least one channel
                            return Err(MiniRedisConnectionError::InvalidArgument(
                                response.to_string(),
                            ));
                        }

                        // unsubscribed channel should exist in the subscribed list at this point
                        self.subscribed_channels.retain(|c| *channel != &c[..]);

                        // Only a single channel should be removed from the
                        // list of subscribed channels.
                        if self.subscribed_channels.len() != len - 1 {
                            return Err(MiniRedisConnectionError::CommandExecute(
                                response.to_string(),
                            ));
                        }
                    }
                    _ => {
                        return Err(MiniRedisConnectionError::InvalidFrameType);
                    }
                },
                frame => return Err(MiniRedisConnectionError::CommandExecute(frame.to_string())),
            };
        }

        Ok(())
    }
}
