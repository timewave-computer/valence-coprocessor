use alloc::vec::Vec;
use valence_coprocessor_types::{DataBackend, Hash, Hasher, HASH_LEN};

use crate::{Opening, Smt, SmtChildren};

impl Opening {
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

impl<D, H> Smt<D, H>
where
    D: DataBackend,
    H: Hasher,
{
    /// Computes a Merkle opening proof for the provided leaf to the root.
    pub fn get_opening(&self, root: Hash, key: &Hash) -> anyhow::Result<Option<Opening>> {
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

        Ok(Some(Opening::new(opening)))
    }

    /// Verifies a Merkle opening generated via [`Smt::get_opening`].
    pub fn verify(opening: &Opening, root: &Hash, key: &Hash, data: &[u8]) -> bool {
        let value = H::hash(data);

        opening.verify::<H>(root, key, &value)
    }
}
