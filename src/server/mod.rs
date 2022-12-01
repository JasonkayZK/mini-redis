//! Minimal Redis server implementation
//!
//! Provides an async `run` function that listens for inbound connections,
//! spawning one task per connection.

use std::future::Future;

use log::info;
use tokio::net::TcpListener;

mod handler;
mod listener;
mod shutdown;

/// Run the mini-redis server.
///
/// Accepts connections from the supplied listener. For each inbound connection,
/// a task is spawned to handle that connection. The server runs until the
/// `shutdown` future completes, at which point the server shuts down
/// gracefully.
///
/// `tokio::signal::ctrl_c()` can be used as the `shutdown` argument. This will
/// listen for a SIGINT signal.
pub async fn run(listener: TcpListener, _shutdown: impl Future) {
    info!(
        "mini-redis server started listen on: {}",
        listener.local_addr().unwrap()
    );
}
