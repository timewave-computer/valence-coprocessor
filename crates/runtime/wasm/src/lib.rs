#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod abi;
pub use valence_coprocessor as core;

#[cfg(feature = "std")]
pub mod host;

pub mod portable;

/// Host controller identifier.
pub const HOST_CONTROLLER: &str = "valence";

// Re-export portable HTTP types
pub use portable::{HttpClient, HttpRequest, HttpResponse, HttpError, HttpMethod, get, post, put, delete, patch, head, post_json_value};
