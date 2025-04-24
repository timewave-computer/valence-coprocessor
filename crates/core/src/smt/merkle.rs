use alloc::vec::Vec;
use core::{ops::Deref, slice};
use msgpacker::MsgPacker;

use serde::{Deserialize, Serialize};

use crate::{Hash, Hasher, HASH_LEN};

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

    /// Verifies a Merkle opening proof to a known root.
    pub fn verify<H: Hasher>(&self, root: &Hash, key: &Hash, value: &Hash) -> bool {
        let mut node = *value;
        let mut depth = self.path.len();

        for sibling in &self.path {
            depth -= 1;

            let i = depth / 8;
            let j = depth % 8;

            if i == HASH_LEN {
                // The provided key is larger than the bits context.
                return false;
            }

            let bit = (key[i] >> (7 - j)) & 1;

            node = if bit == 0 {
                H::merge(&node, sibling)
            } else {
                H::merge(sibling, &node)
            };
        }

        &node == root
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

/// A postorder traversal Merkle opening proof that opens the data to a Merkle root.
#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker,
)]
pub struct SmtOpening {
    /// Traversal key.
    pub key: Vec<u8>,

    /// Preimage of the leaf hash
    pub data: Vec<u8>,

    /// Final node of the opening.
    pub root: Hash,

    /// Postorder traversal siblings from leaf to root.
    pub opening: Opening,
}

impl Deref for SmtOpening {
    type Target = Opening;

    fn deref(&self) -> &Self::Target {
        &self.opening
    }
}
