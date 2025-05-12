use alloc::vec::Vec;

/// A generic data backend to support multiple contexts.
pub trait DataBackend: Clone {
    /// Returns the underlying data from the backend.
    fn get(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>>;

    /// Returns `true` if the provided data exists within the set.
    fn has(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<bool>;

    /// Removes the underlying data from the backend.
    ///
    /// Returns the previous data, if existed.
    fn remove(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>>;

    /// Replaces the underlying data from the backend.
    ///
    /// Returns the previous data, if existed.
    fn set(&self, prefix: &[u8], key: &[u8], data: &[u8]) -> anyhow::Result<Option<Vec<u8>>>;

    /// Returns the underlying bulk data from the backend.
    fn get_bulk(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>>;

    /// Replaces the underlying bulk data from the backend.
    fn set_bulk(&self, prefix: &[u8], key: &[u8], data: &[u8]) -> anyhow::Result<()>;
}

#[cfg(feature = "std")]
pub use use_std::*;

#[cfg(feature = "std")]
mod use_std {
    use std::sync::{Arc, Mutex};

    use hashbrown::HashMap;

    use crate::{Blake3Hasher, DataBackend, Hash, Hasher as _};

    /// A memory data backend.
    #[derive(Debug, Clone, Default)]
    pub struct MemoryBackend {
        data: Arc<Mutex<HashMap<Hash, Vec<u8>>>>,
        bulk: Arc<Mutex<HashMap<Hash, Vec<u8>>>>,
    }

    impl DataBackend for MemoryBackend {
        fn get(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
            let key = Blake3Hasher::digest([b"data", prefix, key]);
            let data = self
                .data
                .lock()
                .map_err(|e| anyhow::anyhow!("failed to lock data backend: {e}"))?;

            Ok(data.get(&key).cloned())
        }

        fn has(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<bool> {
            let key = Blake3Hasher::digest([b"data", prefix, key]);
            let data = self
                .data
                .lock()
                .map_err(|e| anyhow::anyhow!("failed to lock data backend: {e}"))?;

            Ok(data.get(&key).is_some())
        }

        fn remove(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
            let key = Blake3Hasher::digest([b"data", prefix, key]);
            let mut data = self
                .data
                .lock()
                .map_err(|e| anyhow::anyhow!("failed to lock data backend: {e}"))?;

            Ok(data.remove(&key))
        }

        fn set(&self, prefix: &[u8], key: &[u8], data: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
            let key = Blake3Hasher::digest([b"data", prefix, key]);
            let mut d = self
                .data
                .lock()
                .map_err(|e| anyhow::anyhow!("failed to lock data backend: {e}"))?;

            Ok(d.insert(key, data.to_vec()))
        }

        fn get_bulk(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
            let key = Blake3Hasher::digest([b"data", prefix, key]);
            let data = self
                .bulk
                .lock()
                .map_err(|e| anyhow::anyhow!("failed to lock data backend: {e}"))?;

            Ok(data.get(&key).cloned())
        }

        fn set_bulk(&self, prefix: &[u8], key: &[u8], data: &[u8]) -> anyhow::Result<()> {
            let key = Blake3Hasher::digest([b"data", prefix, key]);
            let mut d = self
                .bulk
                .lock()
                .map_err(|e| anyhow::anyhow!("failed to lock data backend: {e}"))?;

            d.insert(key, data.to_vec());

            Ok(())
        }
    }
}
