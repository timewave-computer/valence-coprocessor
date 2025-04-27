use crate::{ExecutionContext, Hash, Hasher};

/// A blake3 hasher implementation for the Valence protocol.
pub struct Blake3Hasher;

/// A blake3 execution environment for the Valence protocol.
pub struct Blake3Context;

impl Blake3Hasher {
    /// Prefix for data hash.
    pub const DATA_PREFIX: &[u8] = &[0x00];

    /// Prefix for node hash.
    pub const MERGE_PREFIX: &[u8] = &[0x01];
}

impl Hasher for Blake3Hasher {
    fn key(context: &str, data: &[u8]) -> Hash {
        ::blake3::derive_key(context, data)
    }

    fn hash(data: &[u8]) -> Hash {
        ::blake3::Hasher::new()
            .update(Self::DATA_PREFIX)
            .update(data)
            .finalize()
            .into()
    }

    fn merge(a: &Hash, b: &Hash) -> Hash {
        ::blake3::Hasher::new()
            .update(Self::MERGE_PREFIX)
            .update(a)
            .update(b)
            .finalize()
            .into()
    }
}

impl ExecutionContext for Blake3Context {
    type Hasher = Blake3Hasher;
}
