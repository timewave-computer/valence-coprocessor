#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(not(feature = "std"))]
pub mod abi;

#[cfg(feature = "std")]
pub mod host;

/// Host module identifier.
pub const HOST_MODULE: &str = "valence";
