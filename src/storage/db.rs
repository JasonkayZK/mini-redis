use std::sync::{Arc, Mutex};

use bytes::Bytes;
use log::info;
use tokio::sync::{broadcast, Notify};
use tokio::time::{self, Duration, Instant};

use crate::storage::store::Store;
use crate::storage::traits::KvStore;

/// A wrapper around a `Db` instance. This exists to allow orderly cleanup
/// of the `Db` by signalling the background purge task to shut down when
/// this struct is dropped.
#[derive(Debug)]
pub(crate) struct DbDropGuard {
    /// The `Db` instance that will be shut down when this `DbHolder` struct
    /// is dropped.
    db: Db,
}

/// Server state shared across all connections.
///
/// `Db` contains a `HashMap` storing the key/value data and all
/// `broadcast::Sender` values for active pub/sub channels.
///
/// A `Db` instance is a handle to shared state. Cloning `Db` is shallow and
/// only incurs an atomic ref count increment.
///
/// When a `Db` value is created, a background task is spawned. This task is
/// used to expire values after the requested duration has elapsed. The task
/// runs until all instances of `Db` are dropped, at which point the task
/// terminates.
#[derive(Debug, Clone)]
struct Db {
    /// Handle to shared state. The background task will also have an
    /// `Arc<Shared>`.
    shared: Arc<SharedDb>,
}

impl Db {
    /// Create a new, empty, `Db` instance. Allocates shared state and spawns a
    /// background task to manage key expiration.
    pub(crate) fn new() -> Db {
        let shared = Arc::new(SharedDb::new());

        // Start the background task.
        tokio::spawn(Db::purge_expired_tasks(shared.clone()));

        Db { shared }
    }

    /// Routine executed by the background task.
    ///
    /// Wait to be notified. On notification, purge any expired keys from the shared
    /// state handle. If `shutdown` is set, terminate the task.
    async fn purge_expired_tasks(shared: Arc<SharedDb>) {
        // If the shutdown flag is set, then the task should exit.
        while !shared.is_shutdown() {
            // Purge all keys that are expired. The function returns the instant at
            // which the **next** key will expire. The worker should wait until the
            // instant has passed then purge again.
            if let Some(when) = shared.purge_expired_keys() {
                // Wait until the next key expires **or** until the background task
                // is notified. If the task is notified, then it must reload its
                // state as new keys have been set to expire early. This is done by
                // looping.
                tokio::select! {
                _ = time::sleep_until(when) => {}
                _ = shared.background_task.notified() => {}
            }
            } else {
                // There are no keys expiring in the future. Wait until the task is
                // notified.
                shared.background_task.notified().await;
            }
        }

        info!("Purge background task shut down")
    }

    /// Signals the purge background task to shut down. This is called by the
    /// `DbShutdown`s `Drop` implementation.
    fn shutdown_purge_task(&self) {
        todo!()
    }
}

impl KvStore for Db {
    /// Get the value associated with a key.
    ///
    /// Returns `None` if there is no value associated with the key. This may be
    /// due to never having assigned a value to the key or a previously assigned
    /// value expired.
    fn get(&self, key: &str) -> Option<Bytes> {
        todo!()
    }

    /// Set the value associated with a key along with an optional expiration
    /// Duration.
    ///
    /// If a value is already associated with the key, it is removed.
    fn set(&self, key: String, value: Bytes, expire: Option<Duration>) {
        todo!()
    }

    /// Returns a `Receiver` for the requested channel.
    ///
    /// The returned `Receiver` is used to receive values broadcast by `PUBLISH`
    /// commands.
    fn subscribe(&self, key: String) -> broadcast::Receiver<Bytes> {
        todo!()
    }

    /// Publish a message to the channel. Returns the number of subscribers
    /// listening on the channel.
    fn publish(&self, key: &str, value: Bytes) -> usize {
        todo!()
    }
}

#[derive(Debug)]
struct SharedDb {
    /// The shared state is guarded by a mutex. This is a `std::sync::Mutex` and
    /// not a Tokio mutex. This is because there are no asynchronous operations
    /// being performed while holding the mutex. Additionally, the critical
    /// sections are very small.
    ///
    /// A Tokio mutex is mostly intended to be used when locks need to be held
    /// across `.await` yield points. All other cases are **usually** best
    /// served by a std mutex. If the critical section does not include any
    /// async operations but is long (CPU intensive or performing blocking
    /// operations), then the entire operation, including waiting for the mutex,
    /// is considered a "blocking" operation and `tokio::task::spawn_blocking`
    /// should be used.
    store: Mutex<Store>,

    /// Notifies the background task handling entry expiration. The background
    /// task waits on this to be notified, then checks for expired values or the
    /// shutdown signal.
    background_task: Notify,
}

impl SharedDb {
    fn new() -> Self {
        SharedDb {
            store: Mutex::new(Store::new()),
            background_task: Notify::new(),
        }
    }

    /// Purge all expired keys and return the `Instant` at which the **next**
    /// key will expire. The background task will sleep until this instant.
    fn purge_expired_keys(&self) -> Option<Instant> {
        let mut store = self.store.lock().unwrap();

        if store.shutdown {
            // The database is shutting down. All handles to the shared state
            // have dropped. The background task should exit.
            return None;
        }

        // This is needed to make the borrow checker happy. In short, `lock()`
        // returns a `MutexGuard` and not a `&mut Store`. The borrow checker is
        // not able to see "through" the mutex guard and determine that it is
        // safe to access both `state.expirations` and `state.entries` mutably,
        // so we get a "real" mutable reference to `Store` outside of the loop.
        let store = &mut *store;

        // Find all keys scheduled to expire **before** now.
        let now = Instant::now();
        while let Some((&(when, id), key)) = store.expirations.iter().next() {
            if when > now {
                // Done purging, `when` is the instant at which the next key
                // expires. The worker task will wait until this instant.
                return Some(when);
            }

            // The key expired, remove it
            store.entries.remove(key);
            store.expirations.remove(&(when, id));
        }

        None
    }

    /// Returns `true` if the database is shutting down
    ///
    /// The `shutdown` flag is set when all `Db` values have dropped, indicating
    /// that the shared state can no longer be accessed.
    fn is_shutdown(&self) -> bool {
        self.store.lock().unwrap().shutdown
    }
}
