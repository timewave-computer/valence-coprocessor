use std::{num::NonZeroUsize, sync::Arc};

use lru::LruCache;
use tokio::sync::Mutex;
use valence_coprocessor::Hash;

/// A LRU cache mapping circuit identifiers to proving keys.
#[derive(Debug, Clone)]
pub struct KeysCache {
    cache: Arc<Mutex<LruCache<Hash, Vec<u8>>>>,
}

impl KeysCache {
    /// Minimum capacity of the LRU cache.
    pub const MIN_CAP: usize = 5;

    /// Creates a new cache instance.
    ///
    /// The capacity is mutated to conform to [`Self::MIN_CAP`].
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(Self::MIN_CAP);
        let capacity = unsafe { NonZeroUsize::new_unchecked(capacity) };
        let cache = LruCache::new(capacity);

        Self {
            cache: Arc::new(Mutex::new(cache)),
        }
    }

    pub async fn get(&self, circuit: &Hash) -> Option<Vec<u8>> {
        self.cache.lock().await.get(circuit).cloned()
    }

    pub async fn set(&self, circuit: Hash, pk: Vec<u8>) {
        self.cache.lock().await.push(circuit, pk);
    }
}
