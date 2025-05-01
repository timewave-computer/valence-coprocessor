#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use valence_coprocessor::Hash;

#[cfg(feature = "circuit")]
mod circuit;

#[cfg(feature = "lib")]
mod library;

/// A Ethereum domain definition.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Ethereum;

/// A Ethereum state proof.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "msgpacker", derive(msgpacker::MsgPacker))]
pub struct EthereumStateProof {
    /// The root of the opening.
    pub root: Hash,

    /// The Merkle opening to the root
    pub opening: Vec<Vec<u8>>,

    /// The leaf key.
    pub key: Vec<u8>,

    /// The leaf value.
    pub value: Vec<u8>,
}

/// A proven key value opening to the state root.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "msgpacker", derive(msgpacker::MsgPacker))]
pub struct EthereumCircuitOutput {
    /// Root of the key-value opening.
    pub root: Hash,
    /// Leaf key.
    pub key: Vec<u8>,
    /// Leaf value.
    pub value: Vec<u8>,
}
