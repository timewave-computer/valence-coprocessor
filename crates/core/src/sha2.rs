use crate::{ExecutionContext, Hash, Hasher};
use sha2::{Digest, Sha256};
/// A blake3 hasher implementation for the Valence protocol.
pub struct Sha2Hasher;

/// A blake3 execution environment for the Valence protocol.
pub struct Sha2Context;

impl Sha2Hasher {
    /// Prefix for data hash.
    pub const DATA_PREFIX: &[u8] = &[0x00];

    /// Prefix for node hash.
    pub const MERGE_PREFIX: &[u8] = &[0x01];
}

impl Hasher for Sha2Hasher {
    fn key(context: &str, data: &[u8]) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(context);

        hasher.update(data);
        hasher.finalize().into()
    }

    fn hash(data: &[u8]) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(Self::DATA_PREFIX);

        hasher.update(data);
        hasher.finalize().into()
    }

    fn merge(a: &Hash, b: &Hash) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(a);

        hasher.update(b);
        hasher.finalize().into()
    }
}

impl ExecutionContext for Sha2Context {
    type Hasher = Sha2Hasher;
}
