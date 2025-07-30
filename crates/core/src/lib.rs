#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod context;
mod domain;
mod registry;
mod vm;
mod zkvm;

#[cfg(feature = "std")]
mod data;

#[cfg(feature = "std")]
mod historical;

#[cfg(feature = "std")]
pub mod utils;

#[cfg(feature = "mocks")]
pub mod mocks;

pub use context::*;
pub use domain::*;
pub use registry::*;
pub use vm::*;
pub use zkvm::*;

#[cfg(feature = "std")]
pub use data::*;

#[cfg(feature = "std")]
pub use historical::*;

pub use valence_coprocessor_merkle::*;
pub use valence_coprocessor_types::*;
