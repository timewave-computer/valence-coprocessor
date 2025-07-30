use alloc::vec::Vec;
use valence_coprocessor_types::{DataBackend, Hash, Hasher, HASH_LEN};

use crate::{Smt, SmtChildren};

impl<D, H> Smt<D, H>
where
    D: DataBackend,
    H: Hasher,
{
    /// Inserts a leaf into the tree.
    pub fn insert(&self, root: Hash, key: &Hash, data: &[u8]) -> anyhow::Result<Hash> {
        let mut depth = 0;

        let leaf = H::hash(data);

        self.insert_key_data(key, data)?;
        self.insert_node_key(&leaf, key)?;

        // childless node
        if root == Hash::default() {
            return Ok(leaf);
        }

        // single node tree
        if self.is_leaf(&root)? {
            let sibling_key = match self.get_node_key(&root)? {
                Some(k) => k,
                None => anyhow::bail!("inconsistent tree state; root {root:x?} is a leaf but doesn't have associated leaf key"),
            };

            let i = depth / 8;
            let j = depth % 8;

            if key == &sibling_key {
                // key depleted; replace the value
                return Ok(leaf);
            }

            let mut node_bit = (key[i] >> (7 - j)) & 1;
            let mut sibling_bit = (sibling_key[i] >> (7 - j)) & 1;

            while node_bit == sibling_bit {
                depth += 1;

                let i = depth / 8;
                let j = depth % 8;

                node_bit = (key[i] >> (7 - j)) & 1;
                sibling_bit = (sibling_key[i] >> (7 - j)) & 1;
            }

            let children = SmtChildren {
                left: if node_bit == 0 { leaf } else { root },
                right: if node_bit == 0 { root } else { leaf },
            };
            let mut root = children.parent::<H>();

            self.insert_children(&root, &children)?;

            while depth > 0 {
                depth -= 1;

                let i = depth / 8;
                let j = depth % 8;
                let bit = (key[i] >> (7 - j)) & 1;

                let sibling = Hash::default();
                let children = SmtChildren {
                    left: if bit == 0 { root } else { sibling },
                    right: if bit == 0 { sibling } else { root },
                };

                root = children.parent::<H>();

                self.insert_children(&root, &children)?;
            }

            return Ok(root);
        }

        let mut node = root;
        let mut opening = Vec::with_capacity(HASH_LEN * 8);

        // traverse until leaf
        while let Some(SmtChildren { left, right }) = self.get_children(&node)? {
            let i = depth / 8;
            let j = depth % 8;

            if i == HASH_LEN {
                anyhow::bail!("tree collision over maximum depth");
            }

            let bit = (key[i] >> (7 - j)) & 1;
            let sibling = if bit == 0 { right } else { left };

            node = if bit == 0 { left } else { right };

            opening.push(sibling);

            depth += 1;

            // empty leaf override
            if node == Hash::default() {
                let i = depth / 8;
                let j = depth % 8;
                let bit = (key[i] >> (7 - j)) & 1;

                let children = SmtChildren {
                    left: if bit == 0 { leaf } else { Hash::default() },
                    right: if bit == 0 { Hash::default() } else { leaf },
                };

                node = children.parent::<H>();

                self.insert_children(&node, &children)?;

                break;
            }

            // create a subtree to hold both the new leaf and the old leaf
            if let Some(sibling_key) = self.get_node_key(&node)? {
                if &sibling_key == key {
                    break;
                }

                let i = depth / 8;
                let j = depth % 8;

                let mut node_bit = (key[i] >> (7 - j)) & 1;
                let mut sibling_bit = (sibling_key[i] >> (7 - j)) & 1;

                while node_bit == sibling_bit {
                    depth += 1;

                    let i = depth / 8;
                    let j = depth % 8;

                    node_bit = (key[i] >> (7 - j)) & 1;
                    sibling_bit = (sibling_key[i] >> (7 - j)) & 1;

                    opening.push(Hash::default());
                }

                let children = SmtChildren {
                    left: if node_bit == 0 { leaf } else { node },
                    right: if node_bit == 0 { node } else { leaf },
                };

                node = children.parent::<H>();

                self.insert_children(&node, &children)?;

                break;
            }
        }

        while let Some(sibling) = opening.pop() {
            depth -= 1;

            let i = depth / 8;
            let j = depth % 8;

            let bit = (key[i] >> (7 - j)) & 1;

            let children = SmtChildren {
                left: if bit == 0 { node } else { sibling },
                right: if bit == 0 { sibling } else { node },
            };

            node = children.parent::<H>();

            self.insert_children(&node, &children)?;
        }

        Ok(node)
    }

    /// Removes an entire subtree along with its linked leaf keys and data.
    pub fn prune(&self, root: &Hash) -> anyhow::Result<()> {
        // TODO don't recurse here to not overflow the stack on very deep trees
        if let Some(SmtChildren { left, right }) = self.get_children(root)? {
            self.prune(&left)?;
            self.prune(&right)?;
        }

        if let Some(key) = self.remove_node_key(root)? {
            self.remove_key_data(&key)?;
        }

        self.remove_children(root)?;

        Ok(())
    }
}
