/// The hash output byte-length used in cryptographic primitives like the sparse Merkle tree.
pub const HASH_LEN: usize = 32;

/// The hash output array used in cryptographic primitives like the sparse Merkle tree.
pub type Hash = [u8; HASH_LEN];

/// The hasher high-level definition.
pub trait Hasher: Clone {
    /// Uses the implementation of the hash function to create a key under a constant context.
    ///
    /// This is useful to emulate namespace within a cryptographic space.
    fn key(context: &str, data: &[u8]) -> Hash;

    /// Hashes the data arguments into an array of bytes.
    fn hash(data: &[u8]) -> Hash;

    /// Merges the two hashes into a single one, extending the cryptographic properties of the
    /// underlying hash function.
    fn merge(a: &Hash, b: &Hash) -> Hash;

    /// Consumes the provided iterator, computing the hash of the data.
    fn digest<'a>(data: impl IntoIterator<Item = &'a [u8]>) -> Hash;
}

impl Hasher for () {
    fn key(_context: &str, _data: &[u8]) -> Hash {
        Hash::default()
    }

    fn hash(_data: &[u8]) -> Hash {
        Hash::default()
    }

    fn merge(_a: &Hash, _b: &Hash) -> Hash {
        Hash::default()
    }

    fn digest<'a>(_data: impl IntoIterator<Item = &'a [u8]>) -> Hash {
        Hash::default()
    }
}

#[cfg(feature = "blake3")]
pub use blake3::*;

#[cfg(feature = "blake3")]
mod blake3 {
    use super::*;

    /// A blake3 hasher implementation for the Valence protocol.
    #[derive(Debug, Default, Clone, Copy)]
    pub struct Blake3Hasher;

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

        fn digest<'a>(data: impl IntoIterator<Item = &'a [u8]>) -> Hash {
            let mut h = ::blake3::Hasher::new();

            h.update(Self::DATA_PREFIX);

            data.into_iter().for_each(|d| {
                h.update(d);
            });

            h.finalize().into()
        }
    }
}
