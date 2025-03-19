use hashbrown::HashMap;
use valence_coprocessor_core::{Blake3Context, Hash};

use crate::{Smt, SmtChildren, TreeBackend};

pub type MemorySmt = Smt<MemoryBackend, Blake3Context>;

#[derive(Debug, Default, Clone)]
pub struct MemoryBackend {
    children: HashMap<Hash, SmtChildren>,
    keys: HashMap<Hash, Hash>,
    data: HashMap<Hash, Vec<u8>>,
}

impl MemoryBackend {}

impl TreeBackend for MemoryBackend {
    fn insert_children(&mut self, parent: &Hash, children: &SmtChildren) -> bool {
        self.children.insert(*parent, *children).is_some()
    }

    fn get_children(&self, parent: &Hash) -> Option<SmtChildren> {
        self.children.get(parent).copied()
    }

    fn remove_children(&mut self, parent: &Hash) -> Option<SmtChildren> {
        self.children.remove(parent)
    }

    fn insert_node_key(&mut self, node: &Hash, leaf: &Hash) -> bool {
        self.keys.insert(*node, *leaf).is_some()
    }

    fn has_node_key(&self, node: &Hash) -> bool {
        self.keys.get(node).is_some()
    }

    fn get_node_key(&self, node: &Hash) -> Option<Hash> {
        self.keys.get(node).copied()
    }

    fn remove_node_key(&mut self, node: &Hash) -> Option<Hash> {
        self.keys.remove(node)
    }

    fn insert_key_data(&mut self, key: &Hash, data: Vec<u8>) -> bool {
        self.data.insert(*key, data).is_some()
    }

    fn get_key_data(&self, key: &Hash) -> Option<Vec<u8>> {
        self.data.get(key).cloned()
    }

    fn remove_key_data(&mut self, key: &Hash) -> Option<Vec<u8>> {
        self.data.remove(key)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use valence_coprocessor_core::{Blake3Hasher, Hasher as _};

    use crate::SmtOpening;

    use super::*;

    #[test]
    fn single_node_opening() {
        let context = "poem";
        let data = b"Two roads diverged in a wood, and I took the one less traveled by";

        let mut tree = MemorySmt::default();

        let root = MemorySmt::new_tree();
        let root = tree.insert(root, context, data.to_vec());
        let proof = tree.get_opening(context, root, data).unwrap();

        assert!(MemorySmt::verify(context, &root, &proof));
    }

    #[test]
    fn double_node_opening() {
        let context = "poem";

        let data = [
            b"Hope is the thing with feathers".to_vec(),
            b"Shall I compare thee to a summer's day?".to_vec(),
        ];

        let key = Blake3Hasher::key(context, &data[0]);
        let keyp = Blake3Hasher::key(context, &data[1]);

        // assert the first bit is not a collision
        assert_ne!(key[0] >> 7, keyp[0] >> 7);

        let mut tree = MemorySmt::default();
        let root = MemorySmt::new_tree();

        let root = tree.insert(root, context, data[0].to_vec());
        let root = tree.insert(root, context, data[1].to_vec());

        let proofs = [
            tree.get_opening(context, root, &data[0]).unwrap(),
            tree.get_opening(context, root, &data[1]).unwrap(),
        ];

        assert!(MemorySmt::verify(context, &root, &proofs[0]));
        assert!(MemorySmt::verify(context, &root, &proofs[1]));
    }

    #[test]
    fn double_one_bit_collision() {
        let context = "poem";
        let data = b"And miles to go before I sleep.";
        let collision = [0x00, 0x00, 0x00];

        let key = Blake3Hasher::key(context, data);
        let keyp = Blake3Hasher::key(context, &collision);

        // assert the test case generates a collision on first bits
        assert_eq!(key[0] >> 7, keyp[0] >> 7);
        assert_ne!(key[0] >> 6, keyp[0] >> 6);

        let mut tree = MemorySmt::default();
        let root = MemorySmt::new_tree();

        let root = tree.insert(root, context, data.to_vec());
        let root = tree.insert(root, context, collision.to_vec());

        let proofs = [
            tree.get_opening(context, root, data).unwrap(),
            tree.get_opening(context, root, &collision).unwrap(),
        ];

        assert_eq!(proofs[0].opening.len(), 2);
        assert_eq!(proofs[1].opening.len(), 2);

        assert!(MemorySmt::verify(context, &root, &proofs[0]));
        assert!(MemorySmt::verify(context, &root, &proofs[1]));
    }

    #[test]
    fn double_two_bit_collision() {
        let context = "poem";
        let data = b"And miles to go before I sleep.";
        let collision = [0x00, 0x00, 0x02];

        let key = Blake3Hasher::key(context, data);
        let keyp = Blake3Hasher::key(context, &collision);

        // assert the test case generates a collision on first bits
        assert_eq!(key[0] >> 7, keyp[0] >> 7);
        assert_eq!(key[0] >> 6, keyp[0] >> 6);
        assert_ne!(key[0] >> 5, keyp[0] >> 5);

        let mut tree = MemorySmt::default();
        let root = MemorySmt::new_tree();

        let root = tree.insert(root, context, data.to_vec());
        let root = tree.insert(root, context, collision.to_vec());

        let proofs = [
            tree.get_opening(context, root, data).unwrap(),
            tree.get_opening(context, root, &collision).unwrap(),
        ];

        assert_eq!(proofs[0].opening.len(), 3);
        assert_eq!(proofs[1].opening.len(), 3);

        assert!(MemorySmt::verify(context, &root, &proofs[0]));
        assert!(MemorySmt::verify(context, &root, &proofs[1]));
    }

    #[test]
    fn double_long_collision() {
        let context = "poem";
        let data = b"And miles to go before I sleep.";
        let collision = [0x25, 0x80, 0x30];

        let key = Blake3Hasher::key(context, data);
        let keyp = Blake3Hasher::key(context, &collision);

        // assert the test case generates a collision on first bits
        assert_eq!(key[0], keyp[0]);
        assert_eq!(key[1] >> 5, keyp[1] >> 5);
        assert_ne!(key[1] >> 4, keyp[1] >> 4);

        let mut tree = MemorySmt::default();
        let root = MemorySmt::new_tree();

        let root = tree.insert(root, context, data.to_vec());
        let root = tree.insert(root, context, collision.to_vec());

        let proofs = [
            tree.get_opening(context, root, data).unwrap(),
            tree.get_opening(context, root, &collision).unwrap(),
        ];

        assert_eq!(proofs[0].opening.len(), 12);
        assert_eq!(proofs[1].opening.len(), 12);

        assert!(MemorySmt::verify(context, &root, &proofs[0]));
        assert!(MemorySmt::verify(context, &root, &proofs[1]));
    }

    #[test]
    fn complex_tree() {
        let context = "poem";
        let mask = 0b11100000u8;

        let data = [
            [0x00, 0x00, 0x09],
            [0x00, 0x00, 0x19],
            [0x00, 0x00, 0x03],
            [0x00, 0x00, 0x05],
        ];

        let keys = [
            Blake3Hasher::key(context, &data[0]),
            Blake3Hasher::key(context, &data[1]),
            Blake3Hasher::key(context, &data[2]),
            Blake3Hasher::key(context, &data[3]),
        ];

        assert_eq!(keys[0][0] & mask, 0b10000000u8);
        assert_eq!(keys[1][0] & mask, 0b11000000u8);
        assert_eq!(keys[2][0] & mask, 0b10100000u8);
        assert_eq!(keys[3][0] & mask, 0b01000000u8);

        let mut tree = MemorySmt::default();
        let root = MemorySmt::new_tree();

        let mut proofs = [
            SmtOpening::default(),
            SmtOpening::default(),
            SmtOpening::default(),
            SmtOpening::default(),
        ];

        // R = 0

        let root = tree.insert(root, context, data[0].to_vec());

        proofs[0] = tree.get_opening(context, root, &data[0]).unwrap();

        assert_eq!(proofs[0].opening.len(), 0);

        assert!(MemorySmt::verify(context, &root, &proofs[0]));

        assert_eq!(&proofs[0].data, &data[0]);

        //   R
        //  / \
        // _   o
        //    / \
        //   0   1

        let root = tree.insert(root, context, data[1].to_vec());

        proofs[0] = tree.get_opening(context, root, &data[0]).unwrap();
        proofs[1] = tree.get_opening(context, root, &data[1]).unwrap();

        assert_eq!(proofs[0].opening.len(), 2);
        assert_eq!(proofs[1].opening.len(), 2);

        assert!(MemorySmt::verify(context, &root, &proofs[0]));
        assert!(MemorySmt::verify(context, &root, &proofs[1]));

        assert_eq!(&proofs[0].data, &data[0]);
        assert_eq!(&proofs[1].data, &data[1]);

        //   R
        //  / \
        // _   o
        //    / \
        //   o   1
        //  / \
        // 0   2

        let root = tree.insert(root, context, data[2].to_vec());

        proofs[0] = tree.get_opening(context, root, &data[0]).unwrap();
        proofs[1] = tree.get_opening(context, root, &data[1]).unwrap();
        proofs[2] = tree.get_opening(context, root, &data[2]).unwrap();

        assert_eq!(proofs[0].opening.len(), 3);
        assert_eq!(proofs[1].opening.len(), 2);
        assert_eq!(proofs[2].opening.len(), 3);

        assert!(MemorySmt::verify(context, &root, &proofs[0]));
        assert!(MemorySmt::verify(context, &root, &proofs[1]));
        assert!(MemorySmt::verify(context, &root, &proofs[2]));

        assert_eq!(&proofs[0].data, &data[0]);
        assert_eq!(&proofs[1].data, &data[1]);
        assert_eq!(&proofs[2].data, &data[2]);

        //   R
        //  / \
        // 3   o
        //    / \
        //   o   1
        //  / \
        // 0   2

        let root = tree.insert(root, context, data[3].to_vec());

        proofs[0] = tree.get_opening(context, root, &data[0]).unwrap();
        proofs[1] = tree.get_opening(context, root, &data[1]).unwrap();
        proofs[2] = tree.get_opening(context, root, &data[2]).unwrap();
        proofs[3] = tree.get_opening(context, root, &data[3]).unwrap();

        assert!(MemorySmt::verify(context, &root, &proofs[0]));
        assert!(MemorySmt::verify(context, &root, &proofs[1]));
        assert!(MemorySmt::verify(context, &root, &proofs[2]));
        assert!(MemorySmt::verify(context, &root, &proofs[3]));

        assert_eq!(&proofs[0].data, &data[0]);
        assert_eq!(&proofs[1].data, &data[1]);
        assert_eq!(&proofs[2].data, &data[2]);
        assert_eq!(&proofs[3].data, &data[3]);
    }

    #[test]
    fn deep_opening() {
        let n = [1778514084u32, 252724253, 45104643];

        let ctx = "property";
        let root = MemorySmt::new_tree();
        let mut tree = MemorySmt::default();

        // R = 0

        let root = tree.insert(root, ctx, n[0].to_le_bytes().to_vec());

        let p0 = tree.get_opening(ctx, root, &n[0].to_le_bytes()).unwrap();

        assert_eq!(&p0.data, &n[0].to_le_bytes());

        assert_eq!(p0.opening.len(), 0);

        //   R
        //  / \
        // 1   0

        let root = tree.insert(root, ctx, n[1].to_le_bytes().to_vec());

        let p0 = tree.get_opening(ctx, root, &n[0].to_le_bytes()).unwrap();
        let p1 = tree.get_opening(ctx, root, &n[1].to_le_bytes()).unwrap();

        assert_eq!(&p0.data, &n[0].to_le_bytes());
        assert_eq!(&p1.data, &n[1].to_le_bytes());

        assert_eq!(p0.opening.len(), 1);
        assert_eq!(p1.opening.len(), 1);

        //     R
        //    / \
        //   1   o
        //      / \
        //     o   _
        //    / \
        //   o   _
        //  / \
        // 0   2

        let root = tree.insert(root, ctx, n[2].to_le_bytes().to_vec());

        let p0 = tree.get_opening(ctx, root, &n[0].to_le_bytes()).unwrap();
        let p1 = tree.get_opening(ctx, root, &n[1].to_le_bytes()).unwrap();
        let p2 = tree.get_opening(ctx, root, &n[2].to_le_bytes()).unwrap();

        assert_eq!(&p0.data, &n[0].to_le_bytes());
        assert_eq!(&p1.data, &n[1].to_le_bytes());
        assert_eq!(&p2.data, &n[2].to_le_bytes());

        assert_eq!(p0.opening.len(), 4);
        assert_eq!(p1.opening.len(), 1);
        assert_eq!(p2.opening.len(), 4);
    }

    proptest! {
        #[test]
        #[ignore]
        fn smt_property_check(numbers in proptest::collection::vec(0u32..u32::MAX, 1..1000)) {
            let context = "property";
            let mut tree = MemorySmt::default();
            let mut root = MemorySmt::new_tree();
            let mut values = Vec::with_capacity(numbers.len());

            for n in numbers {
                values.push(n.to_le_bytes().to_vec());

                root = tree.insert(root, context, values.last().cloned().unwrap());

                for v in &values {
                    let proof = tree.get_opening(context, root, v).unwrap();

                    assert!(MemorySmt::verify(context, &root, &proof));
                }
            }
        }
    }
}
