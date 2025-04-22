use hashbrown::HashMap;

use crate::{Blake3Hasher, Hash, Hasher as _};

/// A generic data backend to support multiple contexts.
pub trait DataBackend {
    /// Returns the underlying data from the backend.
    fn get(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>>;

    /// Returns `true` if the provided data exists within the set.
    fn has(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<bool>;

    /// Removes the underlying data from the backend.
    ///
    /// Returns the previous data, if existed.
    fn remove(&mut self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>>;

    /// Replaces the underlying data from the backend.
    ///
    /// Returns the previous data, if existed.
    fn set(&mut self, prefix: &[u8], key: &[u8], data: &[u8]) -> anyhow::Result<Option<Vec<u8>>>;
}

/// A memory data backend.
#[derive(Debug, Default)]
pub struct MemoryBackend {
    data: HashMap<Hash, Vec<u8>>,
}

impl DataBackend for MemoryBackend {
    fn get(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        let key = Blake3Hasher::digest([b"data", prefix, key]);

        Ok(self.data.get(&key).cloned())
    }

    fn has(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<bool> {
        let key = Blake3Hasher::digest([b"data", prefix, key]);

        Ok(self.data.get(&key).is_some())
    }

    fn remove(&mut self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        let key = Blake3Hasher::digest([b"data", prefix, key]);

        Ok(self.data.remove(&key))
    }

    fn set(&mut self, prefix: &[u8], key: &[u8], data: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        let key = Blake3Hasher::digest([b"data", prefix, key]);

        Ok(self.data.insert(key, data.to_vec()))
    }
}

#[cfg(feature = "std")]
pub use use_std::*;

#[cfg(feature = "std")]
mod use_std {
    use std::sync::{Arc, Mutex};

    use super::{DataBackend, MemoryBackend};

    /// A memory data backend.
    #[derive(Debug, Clone, Default)]
    pub struct SyncMemoryBackend {
        data: Arc<Mutex<MemoryBackend>>,
    }

    impl DataBackend for SyncMemoryBackend {
        fn get(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
            self.data.lock().unwrap().get(prefix, key)
        }

        fn has(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<bool> {
            self.data.lock().unwrap().has(prefix, key)
        }

        fn remove(&mut self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
            self.data.lock().unwrap().remove(prefix, key)
        }

        fn set(
            &mut self,
            prefix: &[u8],
            key: &[u8],
            data: &[u8],
        ) -> anyhow::Result<Option<Vec<u8>>> {
            self.data.lock().unwrap().set(prefix, key, data)
        }
    }
}
