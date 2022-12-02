use bytes::Bytes;
use tokio::sync::broadcast;
use tokio::time::Duration;

pub trait KvStore {
    fn get(&self, key: &str) -> Option<Bytes>;

    /// Set the value associated with a key along with an optional expiration
    /// Duration.
    ///
    /// If a value is already associated with the key, it is removed.
    fn set(&self, key: String, value: Bytes, expire: Option<Duration>);

    /// Returns a `Receiver` for the requested channel.
    ///
    /// The returned `Receiver` is used to receive values broadcast by `PUBLISH`
    /// commands.
    fn subscribe(&self, key: String) -> broadcast::Receiver<Bytes>;

    /// Publish a message to the channel. Returns the number of subscribers
    /// listening on the channel.
    fn publish(&self, key: &str, value: Bytes) -> usize;
}
