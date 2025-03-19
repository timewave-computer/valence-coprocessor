use valence_coprocessor_core::Hash;

#[cfg(feature = "memory")]
mod memory;

#[cfg(feature = "memory")]
pub use memory::*;

mod smt;

pub use smt::*;

pub trait TreeBackend {
    fn insert_children(&mut self, parent: &Hash, children: &SmtChildren) -> bool;
    fn get_children(&self, parent: &Hash) -> Option<SmtChildren>;
    fn remove_children(&mut self, parent: &Hash) -> Option<SmtChildren>;

    fn insert_node_key(&mut self, node: &Hash, leaf: &Hash) -> bool;
    fn has_node_key(&self, node: &Hash) -> bool;
    fn get_node_key(&self, node: &Hash) -> Option<Hash>;
    fn remove_node_key(&mut self, node: &Hash) -> Option<Hash>;

    fn insert_key_data(&mut self, key: &Hash, data: Vec<u8>) -> bool;
    fn get_key_data(&self, key: &Hash) -> Option<Vec<u8>>;
    fn remove_key_data(&mut self, key: &Hash) -> Option<Vec<u8>>;
}
