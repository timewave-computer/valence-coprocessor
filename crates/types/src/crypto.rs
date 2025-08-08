use core::{ops::Deref, slice};

use alloc::vec::Vec;
use msgpacker::MsgPacker;
use serde::{Deserialize, Serialize};

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

    /// Hashes the data arguments using no prefix.
    fn hash_raw(data: &[u8]) -> Hash;

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

    fn hash_raw(_data: &[u8]) -> Hash {
        Hash::default()
    }

    fn merge(_a: &Hash, _b: &Hash) -> Hash {
        Hash::default()
    }

    fn digest<'a>(_data: impl IntoIterator<Item = &'a [u8]>) -> Hash {
        Hash::default()
    }
}

/// The preimage of a node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, MsgPacker)]
pub enum Preimage {
    /// An unknown pre-image to zero.
    Zero,

    /// A known pre-image to a non-zero node.
    Data(Vec<u8>),

    /// A pre-computed node value.
    Node(Hash),
}

impl Preimage {
    /// Computes the node value from the pre-image.
    pub fn to_node<H: Hasher>(&self) -> Hash {
        match self {
            Preimage::Zero => Hash::default(),
            Preimage::Data(d) => H::hash(d),
            Preimage::Node(n) => *n,
        }
    }

    /// Returns `true` if the pre-image is of zero.
    pub const fn is_zero(&self) -> bool {
        matches!(self, Self::Zero)
    }
}

/// A Merkle opening to a root.
#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker,
)]
pub struct Opening {
    /// The Merkle path to the root.
    pub path: Vec<Hash>,
}

impl Opening {
    /// Creates a new Merkle opening proof from a path.
    pub fn new(path: Vec<Hash>) -> Self {
        Self { path }
    }

    /// Computes the root for the opening.
    pub fn root<H: Hasher>(&self, key: &Hash, value: &Hash) -> Hash {
        let mut node = *value;
        let mut depth = self.path.len();

        for sibling in &self.path {
            depth -= 1;

            let i = depth / 8;
            let j = depth % 8;

            if i == HASH_LEN {
                // The provided key is larger than the bits context.
                break;
            }

            let bit = (key[i] >> (7 - j)) & 1;

            node = if bit == 0 {
                H::merge(&node, sibling)
            } else {
                H::merge(sibling, &node)
            };
        }

        node
    }

    /// Verifies a Merkle opening proof to a known root.
    pub fn verify<H: Hasher>(&self, root: &Hash, key: &Hash, value: &Hash) -> bool {
        *root == self.root::<H>(key, value)
    }

    /// Verifies the non-membership of the value in the key.
    pub fn verify_non_membership<H: Hasher>(
        &self,
        root: &Hash,
        key: &Hash,
        value: Option<&Hash>,
        preimage: &Preimage,
    ) -> bool {
        let node = preimage.to_node::<H>();

        if let Some(v) = value {
            if v == &node {
                return false;
            }
        }

        self.verify::<H>(root, key, &node)
    }
}

impl Deref for Opening {
    type Target = [Hash];

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl<'a> IntoIterator for &'a Opening {
    type Item = &'a Hash;
    type IntoIter = slice::Iter<'a, Hash>;

    fn into_iter(self) -> Self::IntoIter {
        self.path.iter()
    }
}

impl FromIterator<Hash> for Opening {
    fn from_iter<T: IntoIterator<Item = Hash>>(iter: T) -> Self {
        Self::new(iter.into_iter().collect())
    }
}

/// A non-membership proof.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker)]
pub struct OpeningNonMembership {
    /// The pre-image to the leaf node.
    pub preimage: Preimage,

    /// The opening to the root.
    pub opening: Opening,
}

impl OpeningNonMembership {
    /// Verifies the non-membership of the value in the key.
    pub fn verify<H: Hasher>(&self, root: &Hash, key: &Hash, value: &Hash) -> bool {
        self.opening
            .verify_non_membership::<H>(root, key, Some(value), &self.preimage)
    }
}

/// A non-membership proof of a domain block in the historical tree.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker)]
pub struct HistoricalNonMembership {
    /// Optional domain proof.
    ///
    /// Will be absent if historical opens to zero.
    pub domain: Option<OpeningNonMembership>,

    /// Historical opening to the domain root.
    pub historical: OpeningNonMembership,
}

impl HistoricalNonMembership {
    /// Verifies the non-membership proof of the block.
    pub fn verify<H: Hasher>(
        &self,
        root: &Hash,
        domain_id: &Hash,
        number: u64,
        state_root: &Hash,
    ) -> bool {
        if !self.historical.opening.verify_non_membership::<H>(
            root,
            domain_id,
            None,
            &self.historical.preimage,
        ) {
            return false;
        }

        if self.historical.preimage.is_zero() {
            return true;
        }

        let domain = self.historical.preimage.to_node::<H>();
        let proof = match &self.domain {
            Some(d) => d,
            None => return false,
        };

        let key = HistoricalUpdate::block_number_to_key(number);

        proof
            .opening
            .verify_non_membership::<H>(&domain, &key, Some(state_root), &proof.preimage)
    }
}

/// A Merkle opening with its node value and key.
#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker,
)]
pub struct KeyedOpening {
    /// Node key, if present for the node.
    pub key: Option<Hash>,

    /// Leaf value
    pub node: Hash,

    /// Merkle path
    pub opening: Opening,
}

impl KeyedOpening {
    /// Verifies a Merkle opening proof to a known root.
    pub fn verify<H: Hasher>(&self, root: &Hash) -> bool {
        let key = match self.key {
            Some(k) => k,
            None => return false,
        };

        self.opening.verify::<H>(root, &key, &self.node)
    }
}

/// A compound Merkle opening keyed opening.
#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker,
)]
pub struct CompoundEntry {
    /// Opening key for the compound entry.
    pub key: Hash,

    /// Merkle path to a root.
    pub opening: Opening,
}

/// A compound Merkle opening to a root.
#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker,
)]
pub struct CompoundOpening {
    /// A set of trees with their paths that opens to the root
    pub trees: Vec<CompoundEntry>,
}

impl CompoundOpening {
    /// Computes the root for the compound opening.
    pub fn root<H: Hasher>(&self, value: &Hash) -> Hash {
        let mut node = *value;

        for CompoundEntry { key, opening } in &self.trees {
            node = opening.root::<H>(key, &node);
        }

        node
    }

    /// Verifies a Merkle opening proof to a known root.
    pub fn verify<H: Hasher>(&self, root: &Hash, value: &Hash) -> bool {
        *root == self.root::<H>(value)
    }
}

/// A historical tree transition proof.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker)]
pub struct HistoricalTransitionProof {
    /// Non-membership proof of the previous block.
    pub previous: HistoricalNonMembership,

    /// Update that triggered the transition.
    pub update: HistoricalUpdate,

    /// A post-inclusion Merkle proof.
    pub proof: CompoundOpening,
}

impl HistoricalTransitionProof {
    /// Verifies the correctness of the transition.
    pub fn verify<H: Hasher>(self) -> anyhow::Result<HistoricalUpdate> {
        let Self {
            previous,
            update,
            mut proof,
        } = self;

        anyhow::ensure!(previous.verify::<H>(
            &update.previous,
            &update.block.domain,
            update.block.number,
            &update.block.root,
        ));

        anyhow::ensure!(
            proof.trees.len() == 2,
            "the historical tree contains two compound levels"
        );

        proof.trees[0].key = HistoricalUpdate::block_number_to_key(update.block.number);
        proof.trees[1].key = update.block.domain;

        anyhow::ensure!(
            proof.verify::<H>(&update.root, &update.block.root),
            "the updated state is not consistent"
        );

        let mut current = proof.trees[1].opening.path.clone();
        let mut previous = previous.historical.opening.path;

        current.reverse();
        previous.reverse();

        for (previous, current) in previous.into_iter().zip(current) {
            anyhow::ensure!(previous == current, "the previous and current tree must have a shared path from root to leaf so the two updates are bound");
        }

        Ok(update)
    }
}

#[cfg(feature = "blake3")]
pub use blake3::*;

use crate::HistoricalUpdate;

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

        fn hash_raw(data: &[u8]) -> Hash {
            ::blake3::hash(data).into()
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
