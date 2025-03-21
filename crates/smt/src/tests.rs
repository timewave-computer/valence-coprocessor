use proptest::collection;
use proptest::prelude::*;
use valence_coprocessor_core::ExecutionContext;

use crate::{Smt, TreeBackend};

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
