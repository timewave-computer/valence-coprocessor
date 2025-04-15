use proptest::collection;
use proptest::prelude::*;
use valence_coprocessor_core::ExecutionContext;

use crate::{Smt, TreeBackend};

#[test]
#[cfg(feature = "memory")]
fn value_replace_single() {
    let context = "value";
    let mut smt = crate::MemorySmt::default();
    let root = crate::MemorySmt::empty_tree_root();

    let v = b"Nothing in life is to be feared, it is only to be understood. ";

    for _ in 0..10 {
        let root = smt.insert(root, context, v.to_vec()).unwrap();

        assert!(smt.leaf_exists(context, root, v).unwrap());
    }
}

#[test]
#[cfg(feature = "memory")]
fn value_replace_multiple() {
    let context = "value";
    let mut smt = crate::MemorySmt::default();
    let root = crate::MemorySmt::empty_tree_root();

    let x = b"The most beautiful thing we can experience is the mysterious.";
    let v = b"Nothing in life is to be feared, it is only to be understood. ";

    let root = smt.insert(root, context, x.to_vec()).unwrap();

    for _ in 0..10 {
        let root = smt.insert(root, context, v.to_vec()).unwrap();

        assert!(smt.leaf_exists(context, root, v).unwrap());
    }
}

fn property_check<B, C>(mut tree: Smt<B, C>, numbers: Vec<u32>)
where
    B: TreeBackend,
    C: ExecutionContext,
{
    let context = "property";
    let mut root = Smt::<B, C>::empty_tree_root();
    let mut values = Vec::with_capacity(numbers.len());

    for n in numbers {
        let data = n.to_le_bytes();

        values.push(data);

        root = tree.insert(root, context, data.to_vec()).unwrap();

        let proof = tree.get_opening(context, root, &data).unwrap().unwrap();

        assert!(Smt::<B, C>::verify(context, &root, &proof));
    }

    for v in values {
        let proof = tree.get_opening(context, root, &v).unwrap().unwrap();

        assert!(Smt::<B, C>::verify(context, &root, &proof));
        assert_eq!(&v, proof.data.as_slice());
    }
}

proptest! {
    #[test]
    #[cfg(feature = "memory")]
    fn memory_property_check(numbers in collection::vec(0u32..u32::MAX, 1..100)) {
        property_check(crate::MemorySmt::default(), numbers);
    }

    #[test]
    #[cfg(feature = "rocksdb")]
    fn rocksdb_property_check(numbers in collection::vec(0u32..u32::MAX, 1..100)) {
        let path = ::tempfile::tempdir().unwrap();
        let backend = crate::RocksBackend::open(path).unwrap();
        let smt: Smt<_, valence_coprocessor_core::Blake3Context> = Smt::from(backend);

        property_check(smt, numbers);
    }
}
