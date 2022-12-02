//! Hello world client.
//!
//! A simple client that connects to a mini-redis server, sets key "hello" with value "world",
//! and gets it from the server after
//!
//! You can test this out by running:
//!
//!     cargo run --bin mini-redis-server
//!
//! And then in another terminal run:
//!
//!     cargo run --example hello_world

use mini_redis::client;
use mini_redis::error::MiniRedisClientError;

#[tokio::main]
pub async fn main() -> Result<(), MiniRedisClientError> {
    // Open a connection to the mini-redis address.
    let mut client = client::connect("127.0.0.1:6379").await?;

    // Set the key "hello" with value "world"
    let result = client.set("hello", "world".into()).await?;
    println!("set value to the server success， result: {:?}", result);

    // Get key "hello"
    let result = client.get("hello").await?;
    println!("got value from the server success， result: {:?}", result);

    Ok(())
}
