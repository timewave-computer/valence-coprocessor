#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod abi;
pub use valence_coprocessor as core;

#[cfg(feature = "std")]
pub mod host;

/// Host controller identifier.
pub const HOST_CONTROLLER: &str = "valence";
