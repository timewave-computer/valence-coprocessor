use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::{DataBackend, Hash, Hasher, HASH_LEN};

mod merkle;
mod types;

pub use merkle::*;
pub use types::*;

#[cfg(all(test, feature = "std"))]
mod tests;

/// An in-memory SMT implementation.
#[cfg(feature = "std")]
pub type MemorySmt = Smt<crate::MemoryBackend, crate::Blake3Hasher>;

#[doc = include_str!("README.md")]
pub struct Smt<D, H>
where
    D: DataBackend,
    H: Hasher,
{
    d: D,
    h: PhantomData<H>,
}

impl<D, H> Smt<D, H>
where
    D: DataBackend,
    H: Hasher,
{
    /// Prefix used for tree nodes.
    pub const PREFIX_NODE: &[u8] = b"smt-node";

    /// Prefix used for data nodes.
    pub const PREFIX_DATA: &[u8] = b"smt-data";

    /// Prefix used for key nodes.
    pub const PREFIX_KEY: &[u8] = b"smt-key";

    /// Prefix used for leaf roots.
    pub const PREFIX_ROOT: &[u8] = b"smt-root";

    /// Returns a stateless empty root to be used for newly allocated sparse Merkle trees.
    ///
    /// This is a cryptographic stateless computation and won't touch the data backend.
    pub fn empty_tree_root() -> Hash {
        Hash::default()
    }

    /// Removes an entire subtree along with its linked leaf keys and data.
    pub fn prune(&self, root: &Hash) -> anyhow::Result<()> {
        // TODO don't recurse here to not overflow the stack on very deep trees
        if let Some(SmtChildren { left, right }) = self.get_children(root)? {
            self.prune(&left)?;
            self.prune(&right)?;
        }

        if let Some(key) = self.remove_node_key(root)? {
            self.remove_key_data(&key)?;
        }

        self.remove_children(root)?;

        Ok(())
    }

    /// Computes a Merkle opening proof for the provided leaf to the root.
    ///
    /// The leaf is defined by the combination of the context and its data.
    pub fn get_opening(
        &self,
        context: &str,
        root: Hash,
        key: &[u8],
    ) -> anyhow::Result<Option<SmtOpening>> {
        let key_raw = key.to_vec();
        let key = H::key(context, key);
        let data = match self.get_key_data(&key)? {
            Some(d) => d,
            None => return Ok(None),
        };

        let (mut i, mut j) = (0, 0);
        let mut leaf_node = root;
        let mut opening = Vec::with_capacity(HASH_LEN * 8);

        while let Some(SmtChildren { left, right }) = self.get_children(&leaf_node)? {
            // is current node a leaf?
            if self.has_node_key(&leaf_node)? {
                break;
            } else if i == HASH_LEN {
                anyhow::bail!("The provided key was depleted without a leaf opening.");
            }

            let bit = (key[i] >> (7 - j)) & 1;

            if bit == 0 {
                leaf_node = left;
                opening.push(right);
            } else {
                leaf_node = right;
                opening.push(left);
            };

            j += 1;

            if j == 8 && i == HASH_LEN {
                break;
            } else if j == 8 {
                j = 0;
                i += 1;
            }
        }

        opening.reverse();

        Ok(Some(SmtOpening {
            key: key_raw,
            data,
            root,
            opening: Opening::new(opening),
        }))
    }

    /// Checks if a leaf with the given data exists in the tree at the specified root.
    ///
    /// Returns `true` if the leaf exists, `false` otherwise.
    pub fn leaf_exists(&self, context: &str, root: Hash, data: &[u8]) -> anyhow::Result<bool> {
        self.get_opening(context, root, data).map(|o| o.is_some())
    }

    /// Verifies a proof obtained via [Smt::get_opening].
    pub fn verify(context: &str, root: &Hash, proof: &SmtOpening) -> bool {
        let key = H::key(context, &proof.key);
        let node = H::hash(&proof.data);

        proof.verify::<H>(root, &key, &node)
    }

    /// Returns `true` if the provided node is associated with a leaf key.
    pub fn is_leaf(&self, node: &Hash) -> anyhow::Result<bool> {
        Ok(node == &Hash::default() || self.has_node_key(node)?)
    }

    /// Returns the most recent tree root associated with the provided key.
    pub fn get_key_root(&self, key: &Hash) -> anyhow::Result<Option<Hash>> {
        self.d
            .get(Self::PREFIX_ROOT, key)?
            .map(|o| o.try_into())
            .transpose()
            .map_err(|_| anyhow::anyhow!("error converting bytes to hash"))
    }

    /// Inserts a leaf into the tree.
    ///
    /// The leaf key will be computed given the context and data, and will have a collision
    /// resistance up to [HASH_LEN] bytes.
    pub fn insert(
        &self,
        root: Hash,
        context: &str,
        key: &[u8],
        data: Vec<u8>,
    ) -> anyhow::Result<Hash> {
        let mut depth = 0;

        let key = H::key(context, key);
        let leaf = H::hash(&data);

        self.insert_key_root(&key, &root)?;
        self.insert_key_data(&key, &data)?;
        self.insert_node_key(&leaf, &key)?;

        // childless node
        if root == Hash::default() {
            return Ok(leaf);
        }

        // single node tree
        if self.is_leaf(&root)? {
            let sibling_key = match self.get_node_key(&root)? {
                Some(k) => k,
                None => anyhow::bail!("inconsistent tree state; root {root:x?} is a leaf but doesn't have associated leaf key"),
            };

            let i = depth / 8;
            let j = depth % 8;

            if key == sibling_key {
                // key depleted; replace the value
                return Ok(leaf);
            }

            let mut node_bit = (key[i] >> (7 - j)) & 1;
            let mut sibling_bit = (sibling_key[i] >> (7 - j)) & 1;

            while node_bit == sibling_bit {
                depth += 1;

                let i = depth / 8;
                let j = depth % 8;

                node_bit = (key[i] >> (7 - j)) & 1;
                sibling_bit = (sibling_key[i] >> (7 - j)) & 1;
            }

            let children = SmtChildren {
                left: if node_bit == 0 { leaf } else { root },
                right: if node_bit == 0 { root } else { leaf },
            };
            let mut root = children.parent::<H>();

            self.insert_children(&root, &children)?;

            while depth > 0 {
                depth -= 1;

                let i = depth / 8;
                let j = depth % 8;
                let bit = (key[i] >> (7 - j)) & 1;

                let sibling = Hash::default();
                let children = SmtChildren {
                    left: if bit == 0 { root } else { sibling },
                    right: if bit == 0 { sibling } else { root },
                };

                root = children.parent::<H>();

                self.insert_children(&root, &children)?;
            }

            return Ok(root);
        }

        let mut node = root;
        let mut opening = Vec::with_capacity(HASH_LEN * 8);
        let mut is_leaf = false;

        // traverse until leaf
        while let Some(SmtChildren { left, right }) = self.get_children(&node)? {
            let i = depth / 8;
            let j = depth % 8;

            if i == HASH_LEN {
                anyhow::bail!("tree collision over maximum depth");
            }

            let bit = (key[i] >> (7 - j)) & 1;
            let sibling = if bit == 0 { right } else { left };

            node = if bit == 0 { left } else { right };

            opening.push(sibling);

            depth += 1;

            // empty leaf override
            if node == Hash::default() {
                let i = depth / 8;
                let j = depth % 8;
                let bit = (key[i] >> (7 - j)) & 1;

                let children = SmtChildren {
                    left: if bit == 0 { leaf } else { Hash::default() },
                    right: if bit == 0 { Hash::default() } else { leaf },
                };

                node = children.parent::<H>();

                self.insert_children(&node, &children)?;

                is_leaf = true;

                break;
            }

            // create a subtree to hold both the new leaf and the old leaf
            if let Some(sibling_key) = self.get_node_key(&node)? {
                if sibling_key == key {
                    break;
                }

                let i = depth / 8;
                let j = depth % 8;

                let mut node_bit = (key[i] >> (7 - j)) & 1;
                let mut sibling_bit = (sibling_key[i] >> (7 - j)) & 1;

                while node_bit == sibling_bit {
                    depth += 1;

                    let i = depth / 8;
                    let j = depth % 8;

                    node_bit = (key[i] >> (7 - j)) & 1;
                    sibling_bit = (sibling_key[i] >> (7 - j)) & 1;

                    opening.push(Hash::default());
                }

                let children = SmtChildren {
                    left: if node_bit == 0 { leaf } else { node },
                    right: if node_bit == 0 { node } else { leaf },
                };

                node = children.parent::<H>();

                self.insert_children(&node, &children)?;

                is_leaf = true;

                break;
            }
        }

        anyhow::ensure!(is_leaf, "inconsistent tree state; the root {root:x?} traversed up to {node:x?}, but that node isn't a leaf");

        while let Some(sibling) = opening.pop() {
            depth -= 1;

            let i = depth / 8;
            let j = depth % 8;

            let bit = (key[i] >> (7 - j)) & 1;

            let children = SmtChildren {
                left: if bit == 0 { node } else { sibling },
                right: if bit == 0 { sibling } else { node },
            };

            node = children.parent::<H>();

            self.insert_children(&node, &children)?;
        }

        Ok(node)
    }
}
