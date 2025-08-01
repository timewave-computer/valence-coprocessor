use core::marker::PhantomData;

use alloc::vec::Vec;
use msgpacker::MsgPacker;
use serde::{Deserialize, Serialize};
use valence_coprocessor_types::{DataBackend, Hash, Hasher};
use zerocopy::{Immutable, IntoBytes, TryFromBytes};

/// A sparse Merkle tree implementation for the Valence protocol.
pub struct Smt<D, H>
where
    D: DataBackend,
    H: Hasher,
{
    pub(crate) namespace_node: Hash,
    pub(crate) namespace_data: Hash,
    pub(crate) namespace_key: Hash,
    pub(crate) d: D,
    pub(crate) h: PhantomData<H>,
}

/// A children tuple of a parent node in the sparse Merkle tree.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    TryFromBytes,
    IntoBytes,
    Immutable,
    Serialize,
    Deserialize,
    MsgPacker,
)]
pub struct SmtChildren {
    /// The left child associated with `0` in the key traversal.
    pub left: Hash,
    /// The right child associated with `1` in the key traversal.
    pub right: Hash,
}

/// A Merkle opening to a root.
#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker,
)]
pub struct Opening {
    /// The Merkle path to the root.
    pub path: Vec<Hash>,
}

impl SmtChildren {
    /// Computes the parent node in a sparse Merkle tree, given the children tuple.
    pub fn parent<H: Hasher>(&self) -> Hash {
        H::merge(&self.left, &self.right)
    }
}

/// A Merkle opening with its node value and key.
#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker,
)]
pub struct KeyedOpening {
    /// Node key
    pub key: Hash,

    /// Leaf value
    pub node: Hash,

    /// Merkle path
    pub opening: Opening,
}

impl KeyedOpening {
    /// Verifies a Merkle opening proof to a known root.
    pub fn verify<H: Hasher>(&self, root: &Hash) -> bool {
        self.opening.verify::<H>(root, &self.key, &self.node)
    }
}
