use std::fmt;

use valence_coprocessor::{DataBackend, MemoryBackend};
use valence_coprocessor_redis::RedisBackend;

#[derive(Debug, Clone)]
pub enum ServiceBackend {
    Memory(MemoryBackend),
    Redis(RedisBackend),
}

impl Default for ServiceBackend
{
    fn default() -> Self {
        Self::Memory(Default::default())
    }
}

impl From<MemoryBackend> for ServiceBackend {
    fn from(b: MemoryBackend) -> Self {
        Self::Memory(b)
    }
}

impl From<RedisBackend> for ServiceBackend {
    fn from(b: RedisBackend) -> Self {
        Self::Redis(b)
    }
}

impl DataBackend for ServiceBackend {
    fn get(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        match self {
            ServiceBackend::Memory(b) => b.get(prefix, key),
            ServiceBackend::Redis(b) => b.get(prefix, key),
        }
    }

    fn has(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<bool> {
        match self {
            ServiceBackend::Memory(b) => b.has(prefix, key),
            ServiceBackend::Redis(b) => b.has(prefix, key),
        }
    }

    fn remove(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        match self {
            ServiceBackend::Memory(b) => b.remove(prefix, key),
            ServiceBackend::Redis(b) => b.remove(prefix, key),
        }
    }

    fn set(&self, prefix: &[u8], key: &[u8], data: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        match self {
            ServiceBackend::Memory(b) => b.set(prefix, key, data),
            ServiceBackend::Redis(b) => b.set(prefix, key, data),
        }
    }
}

impl fmt::Display for ServiceBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceBackend::Memory(_) => write!(f, "memory"),
            ServiceBackend::Redis(_) => write!(f, "data"),
        }
    }
}
