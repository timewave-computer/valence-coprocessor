#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
#![no_std]

#[cfg(feature = "blake3")]
mod blake3;

#[cfg(feature = "blake3")]
pub use blake3::*;

#[cfg(feature = "sha2")]
mod sha2;

#[cfg(feature = "sha2")]
pub use sha2::*;

/// The hash output byte-length used in cryptographic primitives like the sparse Merkle tree.
pub const HASH_LEN: usize = 32;

/// The hash output array used in cryptographic primitives like the sparse Merkle tree.
pub type Hash = [u8; HASH_LEN];

/// The hasher high-level definition.
pub trait Hasher {
    /// Uses the implementation of the hash function to create a key under a constant context.
    ///
    /// This is useful to emulate namespace within a cryptographic space.
    fn key(context: &str, data: &[u8]) -> Hash;

    /// Hashes the data arguments into an array of bytes.
    fn hash(data: &[u8]) -> Hash;

    /// Merges the two hashes into a single one, extending the cryptographic properties of the
    /// underlying hash function.
    fn merge(a: &Hash, b: &Hash) -> Hash;
}

/// Execution context for guest programs of a zkVM.
///
/// This trait's implementations handle the serialization of witness data into the guest program,
/// while also preserving an execution context. They are capable of performing a dry-run to select
/// and send relevant data to the ZK execution environment.
pub trait ExecutionContext {
    /// The concrete hash implementation for the execution.
    ///
    /// Note: These settings are state-dependent and cannot be altered freely. One instance
    /// involves the sparse Merkle tree, which computes its nodes using a predetermined hash
    /// function. Changing the hash function may cause the Merkle tree construction to fail, as the
    /// relationships between parent and child nodes will no longer be consistent with the previous
    /// selected hash function's computations.
    type Hasher: Hasher;
}
