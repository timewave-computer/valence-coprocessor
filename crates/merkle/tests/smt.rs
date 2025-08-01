use proptest::collection;
use valence_coprocessor::{CompoundOpeningBuilder, MemorySmt};
use valence_coprocessor_merkle::Opening;
use valence_coprocessor_types::{Blake3Hasher, Hasher};

use proptest::prelude::*;

#[test]
fn single_node_opening() -> anyhow::Result<()> {
    let context = "poem";
    let data = b"Two roads diverged in a wood, and I took the one less traveled by";

    let tree = MemorySmt::default();
    let key = Blake3Hasher::key(context, &[]);

    let root = MemorySmt::empty_tree_root();
    let root = tree.insert(root, &key, data)?;
    let proof = tree.get_opening(root, &key)?.unwrap();

    assert!(MemorySmt::verify(&proof, &root, &key, data));

    Ok(())
}

#[test]
fn double_node_opening() -> anyhow::Result<()> {
    let context = "poem";

    let data = [
        b"Hope is the thing with feathers".to_vec(),
        b"Shall I compare thee to a summer's day?".to_vec(),
    ];

    let key = Blake3Hasher::key(context, &data[0]);
    let keyp = Blake3Hasher::key(context, &data[1]);

    // assert the first bit is not a collision
    assert_ne!(key[0] >> 7, keyp[0] >> 7);

    let tree = MemorySmt::default();
    let root = MemorySmt::empty_tree_root();

    let root = tree.insert(root, &key, &data[0])?;
    let root = tree.insert(root, &keyp, &data[1])?;

    let proofs = [
        tree.get_opening(root, &key)?.unwrap(),
        tree.get_opening(root, &keyp)?.unwrap(),
    ];

    assert!(MemorySmt::verify(&proofs[0], &root, &key, &data[0]));
    assert!(MemorySmt::verify(&proofs[1], &root, &keyp, &data[1]));

    Ok(())
}

#[test]
fn double_one_bit_collision() -> anyhow::Result<()> {
    let context = "poem";
    let data = b"And miles to go before I sleep.";
    let collision = &[0x00, 0x00, 0x00];

    let key = Blake3Hasher::key(context, data);
    let keyp = Blake3Hasher::key(context, collision);

    // assert the test case generates a collision on first bits
    assert_eq!(key[0] >> 7, keyp[0] >> 7);
    assert_ne!(key[0] >> 6, keyp[0] >> 6);

    let tree = MemorySmt::default();
    let root = MemorySmt::empty_tree_root();

    let root = tree.insert(root, &key, data)?;
    let root = tree.insert(root, &keyp, collision)?;

    let proofs = [
        tree.get_opening(root, &key)?.unwrap(),
        tree.get_opening(root, &keyp)?.unwrap(),
    ];

    assert_eq!(proofs[0].len(), 2);
    assert_eq!(proofs[1].len(), 2);

    assert!(MemorySmt::verify(&proofs[0], &root, &key, data));
    assert!(MemorySmt::verify(&proofs[1], &root, &keyp, collision));

    Ok(())
}

#[test]
fn double_two_bit_collision() -> anyhow::Result<()> {
    let context = "poem";
    let data = b"And miles to go before I sleep.";
    let collision = &[0x00, 0x00, 0x02];

    let key = Blake3Hasher::key(context, data);
    let keyp = Blake3Hasher::key(context, collision);

    // assert the test case generates a collision on first bits
    assert_eq!(key[0] >> 7, keyp[0] >> 7);
    assert_eq!(key[0] >> 6, keyp[0] >> 6);
    assert_ne!(key[0] >> 5, keyp[0] >> 5);

    let tree = MemorySmt::default();
    let root = MemorySmt::empty_tree_root();

    let root = tree.insert(root, &key, data)?;
    let root = tree.insert(root, &keyp, collision)?;

    let proofs = [
        tree.get_opening(root, &key)?.unwrap(),
        tree.get_opening(root, &keyp)?.unwrap(),
    ];

    assert_eq!(proofs[0].len(), 3);
    assert_eq!(proofs[1].len(), 3);

    assert!(MemorySmt::verify(&proofs[0], &root, &key, data));
    assert!(MemorySmt::verify(&proofs[1], &root, &keyp, collision));

    Ok(())
}

#[test]
fn double_long_collision() -> anyhow::Result<()> {
    let context = "poem";
    let data = b"And miles to go before I sleep.";
    let collision = &[0x25, 0x80, 0x30];

    let key = Blake3Hasher::key(context, data);
    let keyp = Blake3Hasher::key(context, collision);

    // assert the test case generates a collision on first bits
    assert_eq!(key[0], keyp[0]);
    assert_eq!(key[1] >> 5, keyp[1] >> 5);
    assert_ne!(key[1] >> 4, keyp[1] >> 4);

    let tree = MemorySmt::default();
    let root = MemorySmt::empty_tree_root();

    let root = tree.insert(root, &key, data)?;
    let root = tree.insert(root, &keyp, collision)?;

    let proofs = [
        tree.get_opening(root, &key)?.unwrap(),
        tree.get_opening(root, &keyp)?.unwrap(),
    ];

    assert_eq!(proofs[0].len(), 12);
    assert_eq!(proofs[1].len(), 12);

    assert!(MemorySmt::verify(&proofs[0], &root, &key, data));
    assert!(MemorySmt::verify(&proofs[1], &root, &keyp, collision));

    Ok(())
}

#[test]
fn complex_tree() -> anyhow::Result<()> {
    let context = "poem";
    let mask = 0b11100000u8;

    let data = [
        &[0x00, 0x00, 0x09],
        &[0x00, 0x00, 0x19],
        &[0x00, 0x00, 0x03],
        &[0x00, 0x00, 0x05],
    ];

    let keys = [
        Blake3Hasher::key(context, data[0]),
        Blake3Hasher::key(context, data[1]),
        Blake3Hasher::key(context, data[2]),
        Blake3Hasher::key(context, data[3]),
    ];

    assert_eq!(keys[0][0] & mask, 0b10000000u8);
    assert_eq!(keys[1][0] & mask, 0b11000000u8);
    assert_eq!(keys[2][0] & mask, 0b10100000u8);
    assert_eq!(keys[3][0] & mask, 0b01000000u8);

    let tree = MemorySmt::default();
    let root = MemorySmt::empty_tree_root();

    let mut proofs = [
        Opening::default(),
        Opening::default(),
        Opening::default(),
        Opening::default(),
    ];

    // R = 0

    let root = tree.insert(root, &keys[0], data[0])?;

    proofs[0] = tree.get_opening(root, &keys[0])?.unwrap();

    assert_eq!(proofs[0].len(), 1);

    assert!(MemorySmt::verify(&proofs[0], &root, &keys[0], data[0]));

    //   R
    //  / \
    // _   o
    //    / \
    //   0   1

    let root = tree.insert(root, &keys[1], data[1])?;

    proofs[0] = tree.get_opening(root, &keys[0])?.unwrap();
    proofs[1] = tree.get_opening(root, &keys[1])?.unwrap();

    assert_eq!(proofs[0].len(), 2);
    assert_eq!(proofs[1].len(), 2);

    assert!(MemorySmt::verify(&proofs[0], &root, &keys[0], data[0]));
    assert!(MemorySmt::verify(&proofs[1], &root, &keys[1], data[1]));

    //   R
    //  / \
    // _   o
    //    / \
    //   o   1
    //  / \
    // 0   2

    let root = tree.insert(root, &keys[2], data[2])?;

    proofs[0] = tree.get_opening(root, &keys[0])?.unwrap();
    proofs[1] = tree.get_opening(root, &keys[1])?.unwrap();
    proofs[2] = tree.get_opening(root, &keys[2])?.unwrap();

    assert_eq!(proofs[0].len(), 3);
    assert_eq!(proofs[1].len(), 2);
    assert_eq!(proofs[2].len(), 3);

    assert!(MemorySmt::verify(&proofs[0], &root, &keys[0], data[0]));
    assert!(MemorySmt::verify(&proofs[1], &root, &keys[1], data[1]));
    assert!(MemorySmt::verify(&proofs[2], &root, &keys[2], data[2]));

    //   R
    //  / \
    // 3   o
    //    / \
    //   o   1
    //  / \
    // 0   2

    let root = tree.insert(root, &keys[3], data[3])?;

    proofs[0] = tree.get_opening(root, &keys[0])?.unwrap();
    proofs[1] = tree.get_opening(root, &keys[1])?.unwrap();
    proofs[2] = tree.get_opening(root, &keys[2])?.unwrap();
    proofs[3] = tree.get_opening(root, &keys[3])?.unwrap();

    assert!(MemorySmt::verify(&proofs[0], &root, &keys[0], data[0]));
    assert!(MemorySmt::verify(&proofs[1], &root, &keys[1], data[1]));
    assert!(MemorySmt::verify(&proofs[2], &root, &keys[2], data[2]));
    assert!(MemorySmt::verify(&proofs[3], &root, &keys[3], data[3]));

    Ok(())
}

#[test]
fn deep_opening() -> anyhow::Result<()> {
    let n = [1778514084u32, 252724253, 45104643];

    let ctx = "property";
    let root = MemorySmt::empty_tree_root();
    let tree = MemorySmt::default();

    // R = 0

    let data = n[0].to_le_bytes();
    let k0 = Blake3Hasher::key(ctx, &data);
    let root = tree.insert(root, &k0, &data)?;

    let p0 = tree.get_opening(root, &k0)?.unwrap();

    assert_eq!(p0.len(), 1);

    assert!(MemorySmt::verify(&p0, &root, &k0, &n[0].to_le_bytes()));

    //   R
    //  / \
    // 1   0

    let data = n[1].to_le_bytes();
    let k1 = Blake3Hasher::key(ctx, &data);
    let root = tree.insert(root, &k1, &data)?;

    let p0 = tree.get_opening(root, &k0)?.unwrap();
    let p1 = tree.get_opening(root, &k1)?.unwrap();

    assert_eq!(p0.len(), 1);
    assert_eq!(p1.len(), 2);

    assert!(MemorySmt::verify(&p0, &root, &k0, &n[0].to_le_bytes()));
    assert!(MemorySmt::verify(&p1, &root, &k1, &n[1].to_le_bytes()));

    //     R
    //    / \
    //   1   o
    //      / \
    //     o   _
    //    / \
    //   o   _
    //  / \
    // 0   2

    let data = n[2].to_le_bytes();
    let k2 = Blake3Hasher::key(ctx, &data);
    let root = tree.insert(root, &k2, &data)?;

    let p0 = tree.get_opening(root, &k0)?.unwrap();
    let p1 = tree.get_opening(root, &k1)?.unwrap();
    let p2 = tree.get_opening(root, &k2)?.unwrap();

    assert_eq!(p0.len(), 4);
    assert_eq!(p1.len(), 2);
    assert_eq!(p2.len(), 4);

    assert!(MemorySmt::verify(&p0, &root, &k0, &n[0].to_le_bytes()));
    assert!(MemorySmt::verify(&p1, &root, &k1, &n[1].to_le_bytes()));
    assert!(MemorySmt::verify(&p2, &root, &k2, &n[2].to_le_bytes()));

    Ok(())
}

#[test]
fn compound_opening() -> anyhow::Result<()> {
    let context = "poem";
    let mask = 0b11100000u8;

    let ns = ["sm1", "sm2"];

    let data = [
        &[0x00, 0x00, 0x09],
        &[0x00, 0x00, 0x19],
        &[0x00, 0x00, 0x03],
        &[0x00, 0x00, 0x05],
    ];

    let keys = [
        Blake3Hasher::key(context, data[0]),
        Blake3Hasher::key(context, data[1]),
        Blake3Hasher::key(context, data[2]),
        Blake3Hasher::key(context, data[3]),
    ];

    assert_eq!(keys[0][0] & mask, 0b10000000u8);
    assert_eq!(keys[1][0] & mask, 0b11000000u8);
    assert_eq!(keys[2][0] & mask, 0b10100000u8);
    assert_eq!(keys[3][0] & mask, 0b01000000u8);

    let mut roots = [MemorySmt::empty_tree_root(); 2];
    let mut tree = MemorySmt::default().with_namespace(ns[0]);

    let mut proofs = [
        Opening::default(),
        Opening::default(),
        Opening::default(),
        Opening::default(),
    ];

    // R = 0

    tree = tree.with_namespace(ns[0]);
    roots[0] = tree.insert(roots[0], &keys[0], data[0])?;
    proofs[0] = tree.get_opening(roots[0], &keys[0])?.unwrap();

    assert_eq!(proofs[0].len(), 1);

    assert!(MemorySmt::verify(&proofs[0], &roots[0], &keys[0], data[0]));

    //   R
    //  / \
    // _   o
    //    / \
    //   0   1

    tree = tree.with_namespace(ns[0]);
    roots[0] = tree.insert(roots[0], &keys[1], data[1])?;

    proofs[0] = tree.get_opening(roots[0], &keys[0])?.unwrap();
    proofs[1] = tree.get_opening(roots[0], &keys[1])?.unwrap();

    assert_eq!(proofs[0].len(), 2);
    assert_eq!(proofs[1].len(), 2);

    assert!(MemorySmt::verify(&proofs[0], &roots[0], &keys[0], data[0]));
    assert!(MemorySmt::verify(&proofs[1], &roots[0], &keys[1], data[1]));

    // R = 2

    tree = tree.with_namespace(ns[1]);
    roots[1] = tree.insert(roots[1], &keys[2], data[2])?;
    proofs[2] = tree.get_opening(roots[1], &keys[2])?.unwrap();

    assert_eq!(proofs[2].len(), 1);

    assert!(MemorySmt::verify(&proofs[2], &roots[1], &keys[2], data[2]));

    //   R
    //  / \
    // 3   2

    tree = tree.with_namespace(ns[1]);
    roots[1] = tree.insert(roots[1], &keys[3], data[3])?;

    proofs[2] = tree.get_opening(roots[1], &keys[2])?.unwrap();
    proofs[3] = tree.get_opening(roots[1], &keys[3])?.unwrap();

    assert_eq!(proofs[2].len(), 1);
    assert_eq!(proofs[3].len(), 2);

    assert!(MemorySmt::verify(&proofs[2], &roots[1], &keys[2], data[2]));
    assert!(MemorySmt::verify(&proofs[3], &roots[1], &keys[3], data[3]));

    //   R
    //  / \
    // _   o
    //    / \
    //   R   1
    //  / \
    // 3   2

    tree = tree.with_namespace(ns[0]);
    roots[0] = tree.insert_compound(roots[0], &keys[0], roots[1])?;

    let compound = CompoundOpeningBuilder::new(roots[0])
        .with_tree(ns[0], keys[0])
        .with_tree(ns[1], keys[3])
        .opening(tree)?;

    assert!(MemorySmt::verify_compound(&compound, &roots[0], data[3]));

    Ok(())
}

proptest! {
    #[test]
    fn memory_property_check(numbers in collection::vec(0u32..u32::MAX, 1..100)) {
        let context = "property";

        let tree = MemorySmt::default();
        let mut root = MemorySmt::empty_tree_root();
        let mut values = Vec::with_capacity(numbers.len());

        for n in numbers {
            let data = n.to_le_bytes();
            let key = Blake3Hasher::key(context, &data);

            values.push(data);

            root = tree.insert(root, &key, &data).unwrap();

            let proof = tree.get_opening(root, &key).unwrap().unwrap();

            assert!(MemorySmt::verify(&proof, &root, &key, &data));
        }

        for v in values {
            let key = Blake3Hasher::key(context, &v);
            let proof = tree.get_opening(root, &key).unwrap().unwrap();

            assert!(MemorySmt::verify(&proof, &root, &key, &v));
        }
    }
}
