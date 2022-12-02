//! Minimal Redis server implementation
//!
//! Provides an async `run` function that listens for inbound connections,
//! spawning one task per connection.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use log::{error, info};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, Semaphore};
use tokio::time;

use crate::consts::MAX_CONNECTIONS;
use crate::server::listener::Listener;
use crate::storage::db::DbDropGuard;

mod handler;
pub(crate) mod listener;
pub(crate) mod shutdown;

/// Run the mini-redis server.
///
/// Accepts connections from the supplied listener. For each inbound connection,
/// a task is spawned to handle that connection. The server runs until the
/// `shutdown` future completes, at which point the server shuts down
/// gracefully.
///
/// `tokio::signal::ctrl_c()` can be used as the `shutdown` argument. This will
/// listen for a SIGINT signal.
pub async fn run(listener: TcpListener, shutdown: impl Future) {
    info!(
        "mini-redis server started listen on: {}",
        listener.local_addr().unwrap()
    );

    // When the provided `shutdown` future completes, we must send a shutdown
    // message to all active connections. We use a broadcast channel for this
    // purpose. The call below ignores the receiver of the broadcast pair, and when
    // a receiver is needed, the subscribe() method on the sender is used to create one.
    let (notify_shutdown, _) = broadcast::channel(1);
    let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel(1);

    // Initialize the listener state
    let mut server = Listener {
        listener,
        db_holder: DbDropGuard::new(),
        limit_connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        notify_shutdown,
        shutdown_complete_tx,
        shutdown_complete_rx,
    };

    // Concurrently run the server and listen for the `shutdown` signal. The
    // server task runs until an error is encountered, so under normal
    // circumstances, this `select!` statement runs until the `shutdown` signal
    // is received.
    //
    // `select!` statements are written in the form of:
    //
    // ```text
    // <result of async op> = <async op> => <step to perform with result>
    // ```
    //
    // All `<async op>` statements are executed concurrently. Once the **first**
    // op completes, its associated `<step to perform with result>` is
    // performed.
    //
    // The `select!` macro is a foundational building block for writing
    // asynchronous Rust. See the API docs for more details:
    //
    // [select](https://docs.rs/tokio/*/tokio/macro.select.html)
    tokio::select! {
        res = server.run() => {
            // If an error is received here, accepting connections from the TCP
            // listener failed multiple times and the server is giving up.
            //
            // Errors encountered when handling individual connections do not
            // bubble up to this point.
            if let Err(err) = res {
                error!("failed to accept: {:?}", err);
            }
        }
        _ = shutdown => {
            // The shutdown signal has been received,
            // sleep for some time and shutdown gracefully
            time::sleep(Duration::from_secs(2)).await;
            info!("server is shutting down");
        }
    }

    // Extract the `shutdown_complete` receiver and transmitter
    // explicitly drop `shutdown_transmitter`. This is important, as the
    // `.await` below would otherwise never complete.
    let Listener {
        mut shutdown_complete_rx,
        shutdown_complete_tx,
        notify_shutdown,
        ..
    } = server;

    // When `notify_shutdown` is dropped, all tasks which have `subscribe`d will
    // receive the shutdown signal and can exit
    drop(notify_shutdown);
    // Drop final `Sender` so the `Receiver` below can complete
    drop(shutdown_complete_tx);

    // Wait for all active connections to finish processing. As the `Sender`
    // handle held by the listener has been dropped above, the only remaining
    // `Sender` instances are held by connection handler tasks. When those drop,
    // the `mpsc` channel will close and `recv()` will return `None`.
    let _ = shutdown_complete_rx.recv().await;
}
