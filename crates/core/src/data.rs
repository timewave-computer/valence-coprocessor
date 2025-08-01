use std::sync::{Arc, Mutex};

use hashbrown::HashMap;
use valence_coprocessor_merkle::Smt;

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

/// An in-memory SMT implementation.
pub type MemorySmt = Smt<MemoryBackend, Blake3Hasher>;
