#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use valence_coprocessor_core::Hash;

mod smt;

#[cfg(feature = "memory")]
mod memory;

#[cfg(feature = "rocksdb")]
mod rocksdb;

#[cfg(test)]
mod tests;

/// A data backend for sparse Merkle tree implementations.
///
/// Its implementation details may involve caching node connections to facilitate more streamlined
/// delivery of Merkle openings to the consumer tree application.
///
/// The trait is agnostic to specific details of the tree implementation and should solely focus on
/// managing persistence of relationships while making no assumptions whatsoever about the
/// underlying tree structure itself.
pub trait TreeBackend {
    /// Appends a relationship from the parent node to its children within a binary tree
    /// structure, returning true if a prior relationship from the parent node was overwritten.
    fn insert_children(&mut self, parent: &Hash, children: &SmtChildren) -> anyhow::Result<bool>;

    /// Fetches the children linked to the provided parent node.
    fn get_children(&self, parent: &Hash) -> anyhow::Result<Option<SmtChildren>>;

    /// Removes a parent-children relationship from the storage, returning it.
    fn remove_children(&mut self, parent: &Hash) -> anyhow::Result<Option<SmtChildren>>;

    /// Assign a leaf key to a tree node, logically converting the node into a leaf node,
    /// returning `true` if a prior relationship of the provided node was overwritten.
    fn insert_node_key(&mut self, node: &Hash, leaf: &Hash) -> anyhow::Result<bool>;

    /// Returns `true` if the provided node is associated with a leaf key.
    fn has_node_key(&self, node: &Hash) -> anyhow::Result<bool>;

    /// Fetches the associated leaf key of the node.
    fn get_node_key(&self, node: &Hash) -> anyhow::Result<Option<Hash>>;

    /// Removes a node to leaf key association from the node, returning it.
    fn remove_node_key(&mut self, node: &Hash) -> anyhow::Result<Option<Hash>>;

    /// Assign a leaf data to a leaf key, returning `true` if a prior relationship of the
    /// provided key to a leaf data was overwritten.
    fn insert_key_data(&mut self, key: &Hash, data: Vec<u8>) -> anyhow::Result<bool>;

    /// Fetches the associated leaf data to the provided leaf key.
    fn get_key_data(&self, key: &Hash) -> anyhow::Result<Option<Vec<u8>>>;

    /// Removes a leaf key data association, returning it.
    fn remove_key_data(&mut self, key: &Hash) -> anyhow::Result<Option<Vec<u8>>>;
}

pub use smt::*;

#[cfg(feature = "memory")]
pub use memory::*;

#[cfg(feature = "rocksdb")]
pub use rocksdb::*;
