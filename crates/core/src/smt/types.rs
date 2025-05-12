use alloc::vec::Vec;
use core::marker::PhantomData;

use zerocopy::{IntoBytes as _, TryFromBytes as _};

use crate::{DataBackend, Hash, Hasher};

use super::Smt;

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
    zerocopy::TryFromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
)]
pub struct SmtChildren {
    /// The left child associated with `0` in the key traversal.
    pub left: Hash,
    /// The right child associated with `1` in the key traversal.
    pub right: Hash,
}

impl SmtChildren {
    /// Computes the parent node in a sparse Merkle tree, given the children tuple.
    pub fn parent<H: Hasher>(&self) -> Hash {
        H::merge(&self.left, &self.right)
    }
}

impl AsRef<[u8]> for SmtChildren {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<D, H> Default for Smt<D, H>
where
    D: DataBackend + Default,
    H: Hasher,
{
    fn default() -> Self {
        Self {
            d: Default::default(),
            h: PhantomData,
        }
    }
}

impl<D, H> Clone for Smt<D, H>
where
    D: DataBackend + Clone,
    H: Hasher,
{
    fn clone(&self) -> Self {
        Self {
            d: self.d.clone(),
            h: PhantomData,
        }
    }
}

impl<D, H> From<D> for Smt<D, H>
where
    D: DataBackend,
    H: Hasher,
{
    fn from(d: D) -> Self {
        Self { d, h: PhantomData }
    }
}

impl<D, H> Smt<D, H>
where
    D: DataBackend,
    H: Hasher,
{
    pub(super) fn get_children(&self, parent: &Hash) -> anyhow::Result<Option<SmtChildren>> {
        let data = match self.d.get(Self::PREFIX_NODE, parent)? {
            Some(d) => d,
            None => return Ok(None),
        };

        let c = SmtChildren::try_read_from_bytes(data.as_slice())
            .map_err(|_| anyhow::anyhow!("inconsistent children bytes"))?;

        Ok(Some(c))
    }

    pub(super) fn insert_children(
        &self,
        parent: &Hash,
        children: &SmtChildren,
    ) -> anyhow::Result<Option<SmtChildren>> {
        let children = children.as_bytes();

        self.d
            .set(Self::PREFIX_NODE, parent, children)?
            .map(|d| {
                SmtChildren::try_read_from_bytes(d.as_slice())
                    .map_err(|_| anyhow::anyhow!("inconsistent children bytes"))
            })
            .transpose()
    }

    pub(super) fn remove_children(&self, parent: &Hash) -> anyhow::Result<Option<SmtChildren>> {
        let data = match self.d.remove(Self::PREFIX_NODE, parent)? {
            Some(d) => d,
            None => return Ok(None),
        };

        let c = SmtChildren::try_read_from_bytes(data.as_slice())
            .map_err(|_| anyhow::anyhow!("inconsistent children bytes"))?;

        Ok(Some(c))
    }

    pub(super) fn remove_node_key(&self, node: &Hash) -> anyhow::Result<Option<Hash>> {
        self.d
            .remove(Self::PREFIX_KEY, node)?
            .map(Hash::try_from)
            .transpose()
            .map_err(|_| anyhow::anyhow!("failed to read hash from smt nodes"))
    }

    pub(super) fn get_node_key(&self, node: &Hash) -> anyhow::Result<Option<Hash>> {
        self.d
            .get(Self::PREFIX_KEY, node)?
            .map(|o| o.try_into())
            .transpose()
            .map_err(|_| anyhow::anyhow!("error converting bytes to hash"))
    }

    pub(super) fn has_node_key(&self, node: &Hash) -> anyhow::Result<bool> {
        self.d.has(Self::PREFIX_KEY, node)
    }

    pub(super) fn insert_node_key(&self, node: &Hash, key: &Hash) -> anyhow::Result<Option<Hash>> {
        Ok(self
            .d
            .set(Self::PREFIX_KEY, node, key)?
            .map(|o| o.try_into().unwrap_or_default()))
    }

    pub(super) fn remove_key_data(&self, key: &Hash) -> anyhow::Result<Option<Vec<u8>>> {
        self.d.remove(Self::PREFIX_DATA, key)
    }

    pub(super) fn insert_key_data(
        &self,
        key: &Hash,
        data: &[u8],
    ) -> anyhow::Result<Option<Vec<u8>>> {
        self.d.set(Self::PREFIX_DATA, key, data)
    }
}
