use core::marker::PhantomData;

use alloc::vec::Vec;
use valence_coprocessor_core::{ExecutionContext, Hash, Hasher, HASH_LEN};

use crate::TreeBackend;

/// A sparse Merkle tree implementation for the Valence protocol.
///
/// This module utilizes the underlying [TreeBackend] to manage node relationships, encapsulating
/// implementation logic within its own boundaries.
///
/// The [ExecutionContext] offers the necessary cryptographic primitives, including a selection of
/// hashes for use during data insertion.
///
/// The design employs a binary sparse Merkle tree, with each node categorized as either a pure
/// node, a leaf (associated with a specific leaf key), or containing the related leaf data (a raw
/// byte vector).
///
/// Isolated from its data persistence [TreeBackend], the tree is stateless and can be deployed
/// flexibly across distributed instances of the data backend.
///
/// This design strategy aims to boost flexibility and enhance caching on the data backend, as
/// straightforward cache mechanisms like LRU can significantly improve Merkle proof opening, given
/// that certain nodes are frequently accessed.
///
/// The decoupling of tree logic from its data backend enables numerous optimization opportunities,
/// since the tree essentially functions as a traversal through nodes.
///
/// Upon inserting data into the tree, the first step is to compute the leaf key associated with
/// this data. The key method of the hasher of the execution environment (i.e. [Hasher::key]),
/// which takes a constant context string (acting as a consistent namespace for the data) and the
/// data itself, is responsible for computing the key. After the key is derived, traversal ensues
/// bit by bit, progressively moving from the most significant bit (MSB). If the current bit is 0,
/// traversal shifts to the left; if it's 1, it moves to the right.
///
/// The implementation is collision safe up to [HASH_LEN] bytes.
///
/// # Example
///
/// ```rust
/// // An ephemeral in-memory data backend
/// #[cfg(feature = "memory")]
/// {
///     use valence_smt::MemorySmt;
///
///     let context = "foo";
///     let data = b"bar";
///
///     // creates a new instance of the backend
///     let mut tree = MemorySmt::default();
///
///     // computes an empty root for inclusion
///     let root = MemorySmt::empty_tree_root();
///
///     // appends the data into the tree, returning its new Merkle root
///     let root = tree.insert(root, context, data.to_vec());
///
///     // generates a Merkle opening proof
///     let proof = tree.get_opening(context, root, data).unwrap();
///
///     // asserts that the data opens to the provided root
///     assert!(MemorySmt::verify(context, &root, &proof));
/// }
/// ```
pub struct Smt<B, C>
where
    B: TreeBackend,
    C: ExecutionContext,
{
    b: B,
    c: PhantomData<C>,
}

/// A children tuple of a parent node in the sparse Merkle tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SmtChildren {
    /// The left child associated with `0` in the key traversal.
    pub left: Hash,
    /// The right child associated with `1` in the key traversal.
    pub right: Hash,
}

impl SmtChildren {
    /// Computes the parent node in a sparse Merkle tree, given the children tuple.
    pub fn parent<C: ExecutionContext>(&self) -> Hash {
        <C as ExecutionContext>::Hasher::merge(&self.left, &self.right)
    }
}

/// A postorder traversal Merkle opening proof that opens the data to a Merkle root.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SmtOpening {
    /// Preimage of the leaf hash
    pub data: Vec<u8>,

    /// Postorder traversal siblings from leaf to root.
    pub opening: Vec<Hash>,
}

impl<B, C> Default for Smt<B, C>
where
    B: TreeBackend + Default,
    C: ExecutionContext,
{
    fn default() -> Self {
        Self {
            b: Default::default(),
            c: PhantomData,
        }
    }
}

impl<B, C> Clone for Smt<B, C>
where
    B: TreeBackend + Clone,
    C: ExecutionContext,
{
    fn clone(&self) -> Self {
        Self {
            b: self.b.clone(),
            c: PhantomData,
        }
    }
}

impl<B, C> Smt<B, C>
where
    B: TreeBackend,
    C: ExecutionContext,
{
    /// Returns a stateless empty root to be used for newly allocated sparse Merkle trees.
    ///
    /// This is a cryptographic stateless computation and won't touch the data backend.
    pub fn empty_tree_root() -> Hash {
        Hash::default()
    }

    /// Removes an entire subtree along with its linked leaf keys and data.
    pub fn prune(&mut self, root: &Hash) {
        // TODO don't recurse here to not overflow the stack on very deep trees
        if let Some(SmtChildren { left, right }) = self.b.get_children(root) {
            self.prune(&left);
            self.prune(&right);
        }

        if let Some(key) = self.b.remove_node_key(root) {
            self.b.remove_key_data(&key);
        }

        self.b.remove_children(root);
    }

    /// Computes a Merkle opening proof for the provided leaf to the root.
    ///
    /// The leaf is defined by the combination of the context and its data.
    pub fn get_opening(&self, context: &str, root: Hash, data: &[u8]) -> Option<SmtOpening> {
        let key = C::Hasher::key(context, data);
        let data = self.b.get_key_data(&key)?;

        let (mut i, mut j) = (0, 0);
        let mut leaf_node = root;
        let mut opening = Vec::with_capacity(HASH_LEN * 8);

        while let Some(SmtChildren { left, right }) = self.b.get_children(&leaf_node) {
            // is current node a leaf?
            if self.b.has_node_key(&leaf_node) {
                break;
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

        Some(SmtOpening { data, opening })
    }

    /// Verifies a proof obtained via [Smt::get_opening].
    pub fn verify(context: &str, root: &Hash, proof: &SmtOpening) -> bool {
        let key = C::Hasher::key(context, &proof.data);
        let node = C::Hasher::hash(&proof.data);
        let mut depth = proof.opening.len();

        let node = proof.opening.iter().fold(node, |node, sibling| {
            depth -= 1;

            let i = depth / 8;
            let j = depth % 8;
            let bit = (key[i] >> (7 - j)) & 1;

            if bit == 0 {
                C::Hasher::merge(&node, sibling)
            } else {
                C::Hasher::merge(sibling, &node)
            }
        });

        &node == root
    }

    /// Returns `true` if the provided node is associated with a leaf key.
    pub fn is_leaf(&self, node: &Hash) -> bool {
        self.b.has_node_key(node)
    }

    /// Inserts a leaf into the tree.
    ///
    /// The leaf key will be computed given the context and data, and will have a collision
    /// resistance up to [HASH_LEN] bytes.
    pub fn insert(&mut self, root: Hash, context: &str, data: Vec<u8>) -> Hash {
        let mut depth = 0;

        let key = C::Hasher::key(context, &data);
        let leaf = C::Hasher::hash(&data);

        self.b.insert_key_data(&key, data);
        self.b.insert_node_key(&leaf, &key);

        // childless node
        if root == Hash::default() {
            return leaf;
        }

        // single node tree
        if self.is_leaf(&root) {
            let sibling_key = match self.b.get_node_key(&root) {
                Some(k) => k,
                None => unreachable!("fallback for inconsistent tree state"),
            };

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
            }

            let children = SmtChildren {
                left: if node_bit == 0 { leaf } else { root },
                right: if node_bit == 0 { root } else { leaf },
            };
            let mut root = children.parent::<C>();

            self.b.insert_children(&root, &children);

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

                root = children.parent::<C>();

                self.b.insert_children(&root, &children);
            }

            return root;
        }

        let mut node = root;
        let mut opening = Vec::with_capacity(HASH_LEN * 8);

        // TODO there is a corrupted state where a Merkle path doesn't end in 0 or leaf;
        // in that case, this algorithm will extend the inconsistent state as it will
        // halt on a node that has no children, but also is not associated with a leaf
        // key. We might want to raise an error in that case. However, we might not want
        // to make the function fallible only due to an inconsistent state.

        // traverse until empty or leaf
        while let Some(SmtChildren { left, right }) = self.b.get_children(&node) {
            let i = depth / 8;
            let j = depth % 8;
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

                node = children.parent::<C>();

                self.b.insert_children(&node, &children);

                break;
            }

            // create a subtree to hold both the new leaf and the old leaf
            if let Some(sibling_key) = self.b.get_node_key(&node) {
                if sibling_key == key {
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

                node = children.parent::<C>();

                self.b.insert_children(&node, &children);

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

            node = children.parent::<C>();

            self.b.insert_children(&node, &children);
        }

        node
    }
}
