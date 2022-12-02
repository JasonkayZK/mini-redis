//! Minimal Redis client implementation
//!
//! Provides an async connect and methods for issuing the supported commands.

use crate::client::subscriber::Subscriber;
use crate::cmd::get::Get;
use bytes::Bytes;
use log::{debug, error};
use std::time::Duration;

use crate::cmd::ping::Ping;
use crate::cmd::publish::Publish;
use crate::cmd::set::Set;
use crate::cmd::subscribe::Subscribe;
use crate::connection::connect::Connection;
use crate::connection::frame::Frame;
use crate::error::MiniRedisConnectionError;

/// Established connection with a Redis server.
///
/// Backed by a single `TcpStream`, `Client` provides basic network client
/// functionality (no pooling, retrying, ...). Connections are established using
/// the [`connect`](fn@connect) function.
///
/// Requests are issued using the various methods of `Client`.
pub struct Client {
    /// The TCP connection decorated with the redis protocol encoder / decoder
    /// implemented using a buffered `TcpStream`.
    ///
    /// When `Listener` receives an inbound connection, the `TcpStream` is
    /// passed to `Connection::new`, which initializes the associated buffers.
    /// `Connection` allows the handler to operate at the "frame" level and keep
    /// the byte level protocol parsing details encapsulated in `Connection`.
    pub(crate) connection: Connection,
}

impl Client {
    /// Ping to the server.
    ///
    /// Returns PONG if no argument is provided, otherwise
    /// return a copy of the argument as a bulk.
    ///
    /// This command is often used to test if a connection
    /// is still alive, or to measure latency.
    ///
    /// # Examples
    ///
    /// Demonstrates basic usage.
    /// ```no_run
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = mini_redis::client::connect("localhost:6379").await.unwrap();
    ///
    ///     let pong = client.ping(None).await.unwrap();
    ///     assert_eq!(b"PONG", &pong[..]);
    /// }
    /// ```
    pub async fn ping(&mut self, msg: Option<String>) -> Result<Bytes, MiniRedisConnectionError> {
        let frame = Ping::new(msg).into_frame();
        debug!("request: {:?}", frame);

        self.connection.write_frame(&frame).await?;

        match self.read_response().await? {
            Frame::Simple(value) => Ok(value.into()),
            Frame::Bulk(value) => Ok(value),
            frame => Err(MiniRedisConnectionError::CommandExecute(frame.to_string())),
        }
    }

    /// Get the value of key.
    ///
    /// If the key does not exist the special value `None` is returned.
    ///
    /// # Examples
    ///
    /// Demonstrates basic usage.
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = mini_redis::client::connect("localhost:6379").await.unwrap();
    ///
    ///     let val = client.get("foo").await.unwrap();
    ///     println!("Got = {:?}", val);
    /// }
    /// ```
    pub async fn get(&mut self, key: &str) -> Result<Option<Bytes>, MiniRedisConnectionError> {
        // Create a `Get` command for the `key` and convert it to a frame.
        let frame = Get::new(key).into_frame();

        debug!("get command request: {:?}", frame);

        // Write the frame to the socket. This writes the full frame to the
        // socket, waiting if necessary.
        self.connection.write_frame(&frame).await?;

        // Wait for the response from the server
        //
        // Both `Simple` and `Bulk` frames are accepted. `Null` represents the
        // key not being present and `None` is returned.
        match self.read_response().await? {
            Frame::Simple(value) => Ok(Some(value.into())),
            Frame::Bulk(value) => Ok(Some(value)),
            Frame::Null => Ok(None),
            frame => Err(MiniRedisConnectionError::CommandExecute(frame.to_string())),
        }
    }

    /// Set `key` to hold the given `value`.
    ///
    /// The `value` is associated with `key` until it is overwritten by the next
    /// call to `set` or it is removed.
    ///
    /// If key already holds a value, it is overwritten. Any previous time to
    /// live associated with the key is discarded on successful SET operation.
    ///
    /// # Examples
    ///
    /// Demonstrates basic usage.
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = mini_redis::client::connect("localhost:6379").await.unwrap();
    ///
    ///     client.set("foo", "bar".into()).await.unwrap();
    ///
    ///     // Getting the value immediately works
    ///     let val = client.get("foo").await.unwrap().unwrap();
    ///     assert_eq!(val, "bar");
    /// }
    /// ```
    pub async fn set(&mut self, key: &str, value: Bytes) -> Result<(), MiniRedisConnectionError> {
        // Create a `Set` command and pass it to `set_cmd`. A separate method is
        // used to set a value with an expiration. The common parts of both
        // functions are implemented by `set_cmd`.
        self.set_cmd(Set::new(key, value, None)).await
    }

    /// Set `key` to hold the given `value`. The value expires after `expiration`
    ///
    /// The `value` is associated with `key` until one of the following:
    /// - it expires.
    /// - it is overwritten by the next call to `set`.
    /// - it is removed.
    ///
    /// If key already holds a value, it is overwritten. Any previous time to
    /// live associated with the key is discarded on a successful SET operation.
    ///
    /// # Examples
    ///
    /// Demonstrates basic usage. This example is not **guaranteed** to always
    /// work as it relies on time based logic and assumes the client and server
    /// stay relatively synchronized in time. The real world tends to not be so
    /// favorable.
    ///
    /// ```no_run
    /// use tokio::time;
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let ttl = Duration::from_millis(500);
    ///     let mut client = mini_redis::client::connect("localhost:6379").await.unwrap();
    ///
    ///     client.set_expires("foo", "bar".into(), ttl).await.unwrap();
    ///
    ///     // Getting the value immediately works
    ///     let val = client.get("foo").await.unwrap().unwrap();
    ///     assert_eq!(val, "bar");
    ///
    ///     // Wait for the TTL to expire
    ///     time::sleep(ttl).await;
    ///
    ///     let val = client.get("foo").await.unwrap();
    ///     assert!(val.is_some());
    /// }
    /// ```
    pub async fn set_expires(
        &mut self,
        key: &str,
        value: Bytes,
        expiration: Duration,
    ) -> Result<(), MiniRedisConnectionError> {
        // Create a `Set` command and pass it to `set_cmd`. A separate method is
        // used to set a value with an expiration. The common parts of both
        // functions are implemented by `set_cmd`.
        self.set_cmd(Set::new(key, value, Some(expiration))).await
    }

    /// The core `SET` logic, used by both `set` and `set_expires.
    async fn set_cmd(&mut self, cmd: Set) -> Result<(), MiniRedisConnectionError> {
        // Convert the `Set` command into a frame
        let frame = cmd.into_frame();

        debug!("set command request: {:?}", frame);

        // Write the frame to the socket. This writes the full frame to the
        // socket, waiting if necessary.
        self.connection.write_frame(&frame).await?;

        // Wait for the response from the server. On success, the server
        // responds simply with `OK`. Any other response indicates an error.
        match self.read_response().await? {
            Frame::Simple(response) if response == "OK" => Ok(()),
            frame => Err(MiniRedisConnectionError::CommandExecute(frame.to_string())),
        }
    }

    /// Posts `message` to the given `channel`.
    ///
    /// Returns the number of subscribers currently listening on the channel.
    /// There is no guarantee that these subscribers receive the message as they
    /// may disconnect at any time.
    ///
    /// # Examples
    ///
    /// Demonstrates basic usage.
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = mini_redis::client::connect("localhost:6379").await.unwrap();
    ///
    ///     let val = client.publish("foo", "bar".into()).await.unwrap();
    ///     println!("Got = {:?}", val);
    /// }
    /// ```
    pub async fn publish(
        &mut self,
        channel: &str,
        message: Bytes,
    ) -> Result<u64, MiniRedisConnectionError> {
        // Convert the `Publish` command into a frame
        let frame = Publish::new(channel, message).into_frame();

        debug!("publish command request: {:?}", frame);

        // Write the frame to the socket
        self.connection.write_frame(&frame).await?;

        // Read the response
        match self.read_response().await? {
            Frame::Integer(response) => Ok(response),
            frame => Err(MiniRedisConnectionError::CommandExecute(frame.to_string())),
        }
    }

    /// Subscribes the client to the specified channels.
    ///
    /// Once a client issues a subscribe command, it may no longer issue any
    /// non-pub/sub commands. The function consumes `self` and returns a `Subscriber`.
    ///
    /// The `Subscriber` value is used to receive messages as well as manage the
    /// list of channels the client is subscribed to.
    pub async fn subscribe(
        mut self,
        channels: Vec<String>,
    ) -> Result<Subscriber, MiniRedisConnectionError> {
        // Issue the subscribe command to the server and wait for confirmation.
        // The client will then have been transitioned into the "subscriber"
        // state and may only issue pub/sub commands from that point on.
        self.subscribe_cmd(&channels).await?;

        // Return the `Subscriber` type
        Ok(Subscriber {
            client: self,
            subscribed_channels: channels,
        })
    }

    /// The core `SUBSCRIBE` logic, used by misc subscribe fns
    pub(crate) async fn subscribe_cmd(
        &mut self,
        channels: &[String],
    ) -> Result<(), MiniRedisConnectionError> {
        // Convert the `Subscribe` command into a frame
        let frame = Subscribe::new(&channels).into_frame();

        debug!("subscribe command request: {:?}", frame);

        // Write the frame to the socket
        self.connection.write_frame(&frame).await?;

        // For each channel being subscribed to, the server responds with a
        // message confirming subscription to that channel.
        for channel in channels {
            // Read the response
            let response = self.read_response().await?;

            // Verify it is confirmation of subscription.
            match response {
                Frame::Array(ref frame) => match frame.as_slice() {
                    // The server responds with an array frame in the form of:
                    //
                    // ```
                    // [ "subscribe", channel, num-subscribed ]
                    // ```
                    //
                    // where channel is the name of the channel and
                    // num-subscribed is the number of channels that the client
                    // is currently subscribed to.
                    [subscribe, schannel, ..]
                        if *subscribe == "subscribe" && *schannel == channel =>
                    {
                        debug!("subscribe channel: {} success", channel);
                    }
                    _ => {
                        error!("subscribe frame failed, response: {}", response);
                        return Err(MiniRedisConnectionError::CommandExecute(
                            response.to_string(),
                        ));
                    }
                },
                frame => {
                    error!(
                        "subscribe frame failed, response frame type not match: {}",
                        frame
                    );
                    return Err(MiniRedisConnectionError::InvalidFrameType);
                }
            };
        }

        Ok(())
    }

    /// Reads a response frame from the socket.
    ///
    /// If an `Error` frame is received, it is converted to `Err`.
    pub(crate) async fn read_response(&mut self) -> Result<Frame, MiniRedisConnectionError> {
        let response = self.connection.read_frame().await?;

        debug!("read response: {:?}", response);

        match response {
            // Error frames are converted to `Err`
            Some(Frame::Error(msg)) => Err(MiniRedisConnectionError::CommandExecute(msg)),
            Some(frame) => Ok(frame),
            None => {
                // Receiving `None` here indicates the server has closed the
                // connection without sending a frame. This is unexpected and is
                // represented as a "connection reset by peer" error.
                Err(MiniRedisConnectionError::Disconnect)
            }
        }
    }
}
