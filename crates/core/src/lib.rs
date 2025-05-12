#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod context;
mod data;
mod domain;
mod hash;
mod registry;
mod smt;
mod vm;
mod zkvm;

#[cfg(feature = "std")]
mod historical;

#[cfg(feature = "std")]
pub mod utils;

#[cfg(feature = "mocks")]
pub mod mocks;

pub use context::*;
pub use data::*;
pub use domain::*;
pub use hash::*;
pub use registry::*;
pub use smt::*;
pub use vm::*;
pub use zkvm::*;

#[cfg(feature = "std")]
pub use historical::*;
