use crate::{ExecutionContext, Hash, Hasher};

/// A blake3 hasher implementation for the Valence protocol.
pub struct Blake3Hasher;

/// A blake3 execution environment for the Valence protocol.
pub struct Blake3Context;

impl Hasher for Blake3Hasher {
    fn key(context: &str, data: &[u8]) -> Hash {
        ::blake3::derive_key(context, data)
    }

    fn hash(data: &[u8]) -> Hash {
        ::blake3::hash(data).into()
    }

    fn merge(a: &Hash, b: &Hash) -> Hash {
        ::blake3::Hasher::new()
            .update(a)
            .update(b)
            .finalize()
            .into()
    }
}

impl ExecutionContext for Blake3Context {
    type Hasher = Blake3Hasher;
}
