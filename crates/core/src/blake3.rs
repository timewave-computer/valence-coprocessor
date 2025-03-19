use crate::{ExecutionContext, Hash, Hasher};

pub struct Blake3Hasher;

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
