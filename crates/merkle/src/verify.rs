use alloc::vec::Vec;
use valence_coprocessor_types::{
    DataBackend, Hash, Hasher, KeyedOpening, Opening, OpeningNonMembership, Preimage, HASH_LEN,
};

use crate::{Smt, SmtChildren};

impl<D, H> Smt<D, H>
where
    D: DataBackend,
    H: Hasher,
{
    /// Computes a Merkle opening proof for the provided leaf to the root.
    pub fn get_opening(&self, root: Hash, key: &Hash) -> anyhow::Result<Option<Opening>> {
        let keyed = self.get_keyed_opening(root, key)?;
        let opening = keyed.key.filter(|k| k == key).map(|_| keyed.opening);

        Ok(opening)
    }

    /// Creates a Merkle proof of non-membership.
    pub fn get_non_membership_opening(
        &self,
        root: Hash,
        key: &Hash,
    ) -> anyhow::Result<OpeningNonMembership> {
        let keyed = self.get_keyed_opening(root, key)?;
        let preimage = if keyed.node == Hash::default() {
            Preimage::Zero
        } else {
            let key = keyed.key.unwrap_or_default();
            //.ok_or_else(|| anyhow::anyhow!("non-zero leaf should have key"))?;

            let data = self.get_key_data(&key)?.unwrap_or_default();

            Preimage::Data(data)
        };

        Ok(OpeningNonMembership {
            preimage,
            opening: keyed.opening,
        })
    }

    /// Computes a Merkle opening proof for the provided leaf to the root.
    ///
    /// Note: the returned node may not be the one with the target key. The routine will return the
    /// first leaf that matches the path provided by the key.
    pub fn get_keyed_opening(&self, root: Hash, key: &Hash) -> anyhow::Result<KeyedOpening> {
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

        let key = self.get_node_key(&leaf_node)?;
        let opening = Opening::new(opening);

        Ok(KeyedOpening {
            key,
            node: leaf_node,
            opening,
        })
    }

    /// Verifies a Merkle opening generated via [`Smt::get_opening`].
    pub fn verify(opening: &Opening, root: &Hash, key: &Hash, data: &[u8]) -> bool {
        let value = H::hash(data);

        opening.verify::<H>(root, key, &value)
    }

    /// Verifies a non-membership proof.
    pub fn verify_non_membership(
        proof: &OpeningNonMembership,
        root: &Hash,
        key: &Hash,
        data: &[u8],
    ) -> bool {
        let value = H::hash(data);

        proof.verify::<H>(root, key, &value)
    }
}
