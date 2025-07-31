use alloc::vec::Vec;
use msgpacker::MsgPacker;
use serde::{Deserialize, Serialize};
use valence_coprocessor_types::{DataBackend, Hash, Hasher};

use crate::{Opening, Smt};

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

/// A builder for a Merkle compound opening computation.
///
/// The proof starts with the deepest tree. Assume the following structure:
///
/// ```text
///      n0
///     /  \
///    o    0
///   / \
///  k1 n1
///    /  \
///   k2   k3
/// ```
///
/// ```text
///
/// // We open from the innermost tree; that is n1
/// let compound = CompoundOpeningBuilder::new(n1)
///
///     // we provide the namespace that created the root n1 with its respective leaf key.
///     .with_tree(namespace1, k3)
///
///     // we proceed with the outmost tree n0 with the key k0 that opens to the root of n1.
///     .with_tree(namespace0, k0)
///
///     // finally, we open from the deepest tree.
///     .opening(tree.with_namespace(n1))?;
/// ```
#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker,
)]
pub struct CompoundOpeningBuilder {
    root: Hash,
    openings: Vec<(Vec<u8>, Hash)>,
}

impl CompoundOpeningBuilder {
    /// Starts a new compound Merkle opening.
    pub fn new(root: Hash) -> Self {
        Self {
            root,
            openings: Vec::with_capacity(4),
        }
    }

    /// Appends the provided with with the namespace to the opening.
    pub fn with_tree<N>(mut self, namespace: N, key: Hash) -> Self
    where
        N: AsRef<[u8]>,
    {
        self.openings.push((namespace.as_ref().to_vec(), key));
        self
    }

    /// Computes a compound Merkle opening proof.
    pub fn opening<D, H>(self, mut smt: Smt<D, H>) -> anyhow::Result<CompoundOpening>
    where
        D: DataBackend,
        H: Hasher,
    {
        let mut trees = Vec::with_capacity(self.openings.len());
        let mut root = self.root;

        for (namespace, key) in self.openings {
            smt = smt.with_namespace(namespace);

            let (opening, node) = smt.get_opening_with_node(root, &key)?;
            let opening = match opening {
                Some(o) => o,
                None => break,
            };

            root = node;
            trees.push(CompoundEntry { key, opening });
        }

        Ok(CompoundOpening { trees })
    }
}

impl<D, H> Smt<D, H>
where
    D: DataBackend,
    H: Hasher,
{
    /// Inserts `compound` as a leaf into `tree`.
    ///
    /// `compound` is expected to be the root of another tree with a different namespace.
    ///
    /// The `key` will define the traversal path on `tree`.
    ///
    /// Example:
    ///
    /// - n0: "foo"
    /// - n1: "bar"
    ///
    /// - k0: 0b0100..
    /// - k1: 0b0010..
    /// - k2: 0b0100..
    /// - k3: 0b1100..
    ///
    /// If we insert [k0, k1] into n0, and [k2, k3] into n1
    ///
    /// ```text
    ///      n0      n1
    ///     /  \    /  \
    ///    o    0  k2   k3
    ///   / \
    ///  k1  k0
    /// ```
    ///
    /// If we `insert_compound(n0, k0, n1)`:
    ///
    /// ```text
    ///      n0
    ///     /  \
    ///    o    0
    ///   / \
    ///  k1 n1
    ///    /  \
    ///   k2   k3
    /// ```
    pub fn insert_compound(&self, tree: Hash, key: &Hash, compound: Hash) -> anyhow::Result<Hash> {
        self.insert_with_leaf(tree, key, compound, &[])
    }

    /// Verifies a compound Merkle opening generated via [`Smt::get_opening`].
    pub fn verify_compound(opening: &CompoundOpening, root: &Hash, data: &[u8]) -> bool {
        let value = H::hash(data);

        opening.verify::<H>(root, &value)
    }
}
