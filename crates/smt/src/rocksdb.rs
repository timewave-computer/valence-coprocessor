use core::ops::{Deref, DerefMut};
use std::path::Path;

use alloc::vec::Vec;
use rocksdb::{Options, SliceTransform, DB};
use valence_coprocessor_core::Hash;
use zerocopy::TryFromBytes;

use crate::{SmtChildren, TreeBackend};

/// A RocksDB implementation for the SMT backend.
pub struct RocksBackend {
    db: DB,
}

impl RocksBackend {
    /// Key prefix for node relationship data.
    pub const PREFIX_NODE: &[u8] = b"node:";

    /// Key prefix for node-leaf key relationship.
    pub const PREFIX_KEY: &[u8] = b"key :";

    /// Key prefix for node-leaf data relationship.
    pub const PREFIX_DATA: &[u8] = b"data:";

    /// Opens a new RocksDB tree backend.
    pub fn open<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut opts = Options::default();

        let st = SliceTransform::create_fixed_prefix(Self::PREFIX_NODE.len());

        opts.set_prefix_extractor(st);
        opts.create_if_missing(true);

        let db = DB::open(&opts, path)?;

        Ok(Self { db })
    }
}

impl Deref for RocksBackend {
    type Target = DB;

    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

impl DerefMut for RocksBackend {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.db
    }
}

impl TreeBackend for RocksBackend {
    fn insert_children(&mut self, parent: &Hash, children: &SmtChildren) -> anyhow::Result<bool> {
        let key = [Self::PREFIX_NODE, parent].concat();
        let replaced = self.get(&key)?.is_some();

        self.put(&key, children)?;

        Ok(replaced)
    }

    fn get_children(&self, parent: &Hash) -> anyhow::Result<Option<SmtChildren>> {
        let key = [Self::PREFIX_NODE, parent].concat();
        let bytes = match self.get(&key)? {
            Some(b) => b,
            None => return Ok(None),
        };

        let c = SmtChildren::try_read_from_bytes(bytes.as_slice())
            .map_err(|_| anyhow::anyhow!("inconsistent children bytes"))?;

        Ok(Some(c))
    }

    fn remove_children(&mut self, parent: &Hash) -> anyhow::Result<Option<SmtChildren>> {
        let children = self.get_children(parent)?;
        let key = [Self::PREFIX_NODE, parent].concat();

        self.delete(&key)?;

        Ok(children)
    }

    fn insert_node_key(&mut self, node: &Hash, leaf: &Hash) -> anyhow::Result<bool> {
        let key = [Self::PREFIX_KEY, node].concat();
        let replaced = self.get(&key)?.is_some();

        self.put(&key, leaf)?;

        Ok(replaced)
    }

    fn has_node_key(&self, node: &Hash) -> anyhow::Result<bool> {
        let key = [Self::PREFIX_KEY, node].concat();

        Ok(self.get(&key)?.is_some())
    }

    fn get_node_key(&self, node: &Hash) -> anyhow::Result<Option<Hash>> {
        let key = [Self::PREFIX_KEY, node].concat();
        let bytes = match self.get(&key)? {
            Some(b) => b,
            None => return Ok(None),
        };

        let c = Hash::try_read_from_bytes(bytes.as_slice())
            .map_err(|_| anyhow::anyhow!("inconsistent node key bytes"))?;

        Ok(Some(c))
    }

    fn remove_node_key(&mut self, node: &Hash) -> anyhow::Result<Option<Hash>> {
        let node_key = self.get_node_key(node)?;
        let key = [Self::PREFIX_NODE, node].concat();

        self.delete(&key)?;

        Ok(node_key)
    }

    fn insert_key_data(&mut self, key: &Hash, data: Vec<u8>) -> anyhow::Result<bool> {
        let key = [Self::PREFIX_DATA, key].concat();
        let replaced = self.get(&key)?.is_some();

        self.put(&key, data)?;

        Ok(replaced)
    }

    fn get_key_data(&self, key: &Hash) -> anyhow::Result<Option<Vec<u8>>> {
        let key = [Self::PREFIX_DATA, key].concat();

        Ok(self.get(&key)?)
    }

    fn remove_key_data(&mut self, key: &Hash) -> anyhow::Result<Option<Vec<u8>>> {
        let data = self.get_key_data(key)?;
        let key = [Self::PREFIX_DATA, key].concat();

        self.delete(&key)?;

        Ok(data)
    }
}

#[test]
fn rocksdb_prefix_are_uniform() {
    assert_eq!(
        RocksBackend::PREFIX_NODE.len(),
        RocksBackend::PREFIX_KEY.len()
    );

    assert_eq!(
        RocksBackend::PREFIX_NODE.len(),
        RocksBackend::PREFIX_DATA.len()
    );
}
