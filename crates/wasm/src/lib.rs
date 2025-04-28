#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod abi;

#[cfg(feature = "std")]
pub mod host;

/// Host module identifier.
pub const HOST_LIB: &str = "valence";
