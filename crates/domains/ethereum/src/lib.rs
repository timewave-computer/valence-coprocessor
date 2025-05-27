#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;

#[cfg(feature = "circuit")]
mod circuit;

#[cfg(feature = "controller")]
mod controller;

/// A Ethereum domain definition.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Ethereum;

/// A Ethereum state proof.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "msgpacker", derive(msgpacker::MsgPacker))]
pub struct EthereumStateProof {
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
    /// Leaf key.
    pub key: Vec<u8>,

    /// Leaf value.
    pub value: Vec<u8>,
}
