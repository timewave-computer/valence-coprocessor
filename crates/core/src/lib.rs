#[cfg(feature = "blake3")]
mod blake3;

#[cfg(feature = "blake3")]
pub use blake3::*;

pub const HASH_LEN: usize = 32;
pub type Hash = [u8; HASH_LEN];

pub trait Hasher {
    fn key(context: &str, data: &[u8]) -> Hash;
    fn hash(data: &[u8]) -> Hash;
    fn merge(a: &Hash, b: &Hash) -> Hash;
}

pub trait ExecutionContext {
    type Hasher: Hasher;
}
