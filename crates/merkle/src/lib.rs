#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

mod boilerplate;
mod mutate;
mod types;
mod verify;

pub use types::*;
