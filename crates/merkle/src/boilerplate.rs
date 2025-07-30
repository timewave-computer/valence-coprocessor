use core::{marker::PhantomData, ops::Deref, slice};

use alloc::vec::Vec;
use valence_coprocessor_types::{DataBackend, Hash, Hasher};
use zerocopy::{IntoBytes as _, TryFromBytes as _};

use crate::{Opening, Smt, SmtChildren};

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

    /// Returns a stateless empty root to be used for newly allocated sparse Merkle trees.
    ///
    /// This is a cryptographic stateless computation and won't touch the data backend.
    pub fn empty_tree_root() -> Hash {
        Hash::default()
    }

    /// Returns `true` if the provided node is associated with a leaf key.
    pub fn is_leaf(&self, node: &Hash) -> anyhow::Result<bool> {
        Ok(node == &Hash::default() || self.has_node_key(node)?)
    }

    pub(crate) fn get_children(&self, parent: &Hash) -> anyhow::Result<Option<SmtChildren>> {
        let data = match self.d.get(Self::PREFIX_NODE, parent)? {
            Some(d) => d,
            None => return Ok(None),
        };

        let c = SmtChildren::try_read_from_bytes(data.as_slice())
            .map_err(|_| anyhow::anyhow!("inconsistent children bytes"))?;

        Ok(Some(c))
    }

    pub(crate) fn insert_children(
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

    pub(crate) fn remove_children(&self, parent: &Hash) -> anyhow::Result<Option<SmtChildren>> {
        let data = match self.d.remove(Self::PREFIX_NODE, parent)? {
            Some(d) => d,
            None => return Ok(None),
        };

        let c = SmtChildren::try_read_from_bytes(data.as_slice())
            .map_err(|_| anyhow::anyhow!("inconsistent children bytes"))?;

        Ok(Some(c))
    }

    pub(crate) fn remove_node_key(&self, node: &Hash) -> anyhow::Result<Option<Hash>> {
        self.d
            .remove(Self::PREFIX_KEY, node)?
            .map(Hash::try_from)
            .transpose()
            .map_err(|_| anyhow::anyhow!("failed to read hash from smt nodes"))
    }

    pub(crate) fn get_node_key(&self, node: &Hash) -> anyhow::Result<Option<Hash>> {
        self.d
            .get(Self::PREFIX_KEY, node)?
            .map(|o| o.try_into())
            .transpose()
            .map_err(|_| anyhow::anyhow!("error converting bytes to hash"))
    }

    pub(crate) fn has_node_key(&self, node: &Hash) -> anyhow::Result<bool> {
        self.d.has(Self::PREFIX_KEY, node)
    }

    pub(crate) fn insert_node_key(&self, node: &Hash, key: &Hash) -> anyhow::Result<Option<Hash>> {
        Ok(self
            .d
            .set(Self::PREFIX_KEY, node, key)?
            .map(|o| o.try_into().unwrap_or_default()))
    }

    pub(crate) fn remove_key_data(&self, key: &Hash) -> anyhow::Result<Option<Vec<u8>>> {
        self.d.remove(Self::PREFIX_DATA, key)
    }

    pub(crate) fn insert_key_data(
        &self,
        key: &Hash,
        data: &[u8],
    ) -> anyhow::Result<Option<Vec<u8>>> {
        self.d.set(Self::PREFIX_DATA, key, data)
    }

    /// Returns the payload of the provided domain root on the historical SMT.
    pub fn get_key_data(&self, key: &Hash) -> anyhow::Result<Option<Vec<u8>>> {
        self.d.get(Self::PREFIX_DATA, key)
    }
}

impl Opening {
    /// Creates a new Merkle opening proof from a path.
    pub fn new(path: Vec<Hash>) -> Self {
        Self { path }
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
