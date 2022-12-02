///! Core storage implementation for mini-redis
use std::collections::{BTreeMap, HashMap};

use bytes::Bytes;
use tokio::sync::broadcast;
use tokio::time::Instant;

#[derive(Debug)]
pub(crate) struct Store {
    /// The key-value data. We are not trying to do anything fancy so a
    /// `std::collections::HashMap` works fine.
    /// For production implementation, more complex structure can be used!
    pub(crate) entries: HashMap<String, Entry>,

    /// The pub/sub key-space. Redis uses a **separate** key space for key-value
    /// and pub/sub. `mini-redis` handles this by using a separate `HashMap`.
    pub(crate) pub_sub: HashMap<String, broadcast::Sender<Bytes>>,

    /// Tracks key TTLs.
    ///
    /// A `BTreeMap` is used to maintain expirations sorted by when they expire.
    /// This allows the background task to iterate this map to find the value
    /// expiring next.
    ///
    /// While highly unlikely, it is possible for more than one expiration to be
    /// created for the same instant. Because of this, the `Instant` is
    /// insufficient for the key. A unique expiration identifier (`u64`) is used
    /// to break these ties.
    pub(crate) expirations: BTreeMap<(Instant, u64), String>,

    /// Identifier to use for the next expiration. Each expiration is associated
    /// with a unique identifier. See above for why.
    pub(crate) next_id: u64,

    /// True when the Db instance is shutting down. This happens when all `Db`
    /// values drop. Setting this to `true` signals to the background task to
    /// exit.
    pub(crate) shutdown: bool,
}

/// Entry in the key-value store
#[derive(Debug)]
pub(crate) struct Entry {
    /// Uniquely identifies this entry.
    pub(crate) id: u64,

    /// Stored data
    pub(crate) data: Bytes,

    /// Instant at which the entry expires and should be removed from the
    /// database.
    pub(crate) expires_at: Option<Instant>,
}

impl Store {
    pub(crate) fn new() -> Store {
        Store {
            entries: HashMap::new(),
            pub_sub: HashMap::new(),
            expirations: BTreeMap::new(),
            next_id: 0,
            shutdown: false,
        }
    }

    /// Get the next expiration instant for notify
    pub(crate) fn next_expiration(&self) -> Option<Instant> {
        self.expirations.keys().next().map(|expire| expire.0)
    }
}
