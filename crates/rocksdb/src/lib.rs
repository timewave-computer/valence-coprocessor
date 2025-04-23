use std::path::Path;

use rocksdb::{Options, SliceTransform, DB};
use valence_coprocessor::{Blake3Hasher, DataBackend, Hash, Hasher as _, HASH_LEN};

/// A RocksDB data backend.
pub struct RocksBackend {
    data: DB,
}

impl RocksBackend {
    /// Opens a new RocksDB tree backend.
    pub fn open<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut opts = Options::default();

        let st = SliceTransform::create_fixed_prefix(HASH_LEN);

        opts.set_prefix_extractor(st);
        opts.create_if_missing(true);

        let data = DB::open(&opts, path)?;

        Ok(Self { data })
    }

    /// Computes the prefix key.
    pub fn prefix(bytes: &[u8]) -> Hash {
        Blake3Hasher::digest([b"prefix", bytes])
    }
}

impl DataBackend for RocksBackend {
    fn get(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        let prefix = Self::prefix(prefix);
        let key = [&prefix, key].concat();

        Ok(self.data.get(&key)?)
    }

    fn has(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<bool> {
        let prefix = Self::prefix(prefix);
        let key = [&prefix, key].concat();

        Ok(self.data.get(&key)?.is_some())
    }

    fn remove(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        let data = self.get(prefix, key)?;
        let prefix = Self::prefix(prefix);
        let key = [&prefix, key].concat();

        self.data.delete(&key)?;

        Ok(data)
    }

    fn set(&self, prefix: &[u8], key: &[u8], data: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        let prefix = Self::prefix(prefix);
        let key = [&prefix, key].concat();
        let replaced = self.data.get(&key)?;

        self.data.put(&key, data)?;

        Ok(replaced)
    }
}

#[cfg(test)]
mod tests {
    use proptest::collection;
    use proptest::prelude::*;
    use valence_coprocessor::{Blake3Hasher, DataBackend, Hasher, Smt};

    fn property_check<D, H>(tree: Smt<D, H>, numbers: Vec<u32>)
    where
        D: DataBackend,
        H: Hasher,
    {
        let context = "property";
        let mut root = Smt::<D, H>::empty_tree_root();
        let mut values = Vec::with_capacity(numbers.len());

        for n in numbers {
            let data = n.to_le_bytes();

            values.push(data);

            root = tree.insert(root, context, &data, data.to_vec()).unwrap();

            let proof = tree.get_opening(context, root, &data).unwrap().unwrap();

            assert!(Smt::<D, H>::verify(context, &root, &proof));
        }

        for v in values {
            let proof = tree.get_opening(context, root, &v).unwrap().unwrap();

            assert!(Smt::<D, H>::verify(context, &root, &proof));
            assert_eq!(&v, proof.data.as_slice());
        }
    }

    proptest! {
        #[test]
        fn rocksdb_property_check(numbers in collection::vec(0u32..u32::MAX, 1..100)) {
            let path = ::tempfile::tempdir().unwrap();
            let backend = crate::RocksBackend::open(path).unwrap();
            let smt: Smt<_, Blake3Hasher> = Smt::from(backend);

            property_check(smt, numbers);
        }
    }
}
