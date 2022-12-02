//! Subscribe to a redis channel example.
//!
//! A simple client that connects to a mini-redis server, subscribes to "foo" and "bar" channels
//! and awaits messages published on those channels
//!
//! You can test this out by running:
//!
//!     cargo run --bin mini-redis-server
//!
//! Then in another terminal run:
//!
//!     cargo run --example sub
//!
//! And then in another terminal run:
//!
//!     cargo run --example pub

use mini_redis::client;
use mini_redis::error::MiniRedisClientError;

#[tokio::main]
pub async fn main() -> Result<(), MiniRedisClientError> {
    // Open a connection to the mini-redis address.
    let client = client::connect("127.0.0.1:6379").await?;

    // subscribe to channel foo
    let mut subscriber = client.subscribe(vec!["foo".into()]).await?;

    // await messages on channel foo
    if let Some(msg) = subscriber.next_message().await? {
        println!(
            "got message from the channel: {}; message = {:?}",
            msg.channel, msg.content
        );
    }

    Ok(())
}
