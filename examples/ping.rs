//! Ping client.
//!
//! A simple client that connects to a mini-redis server, and say `ping`
//!
//! You can test this out by running:
//!
//!     cargo run --bin mini-redis-server
//!
//! And then in another terminal run:
//!
//!     cargo run --example ping

use mini_redis::client;
use mini_redis::error::MiniRedisConnectionError;

#[tokio::main]
pub async fn main() -> Result<(), MiniRedisConnectionError> {
    // Open a connection to the mini-redis address.
    let mut client = client::connect("127.0.0.1:6379").await?;

    let result = client.ping(None).await?;
    println!("empty ping response: {:?}", result);

    let result = client.ping(Some("hello".into())).await?;
    println!("bytes ping response: {:?}", result);

    Ok(())
}
