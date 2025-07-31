#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

mod boilerplate;
mod compound;
mod mutate;
mod smt;
mod verify;

pub use compound::*;
pub use smt::*;
