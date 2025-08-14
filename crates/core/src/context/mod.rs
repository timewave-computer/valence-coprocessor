use core::marker::PhantomData;

use alloc::vec::Vec;

use crate::{Blake3Hasher, DataBackend, Hash, Hasher, Registry};

mod auth;
mod boilerplate;
mod domain;
mod storage;
mod zk;

pub use auth::*;

pub use buf_fs::{File, FileSystem};

/// Execution context with blake3 hasher.
pub type Blake3Context<D> = ExecutionContext<Blake3Hasher, D>;

/// Execution context for a Valence controller.
#[derive(Clone)]
pub struct ExecutionContext<H, D>
where
    H: Hasher,
    D: DataBackend,
{
    controller: Hash,
    data: D,
    hasher: PhantomData<H>,
    historical: Hash,
    registry: Registry<D>,
    owner: Option<Vec<u8>>,

    #[cfg(feature = "std")]
    log: ::std::sync::Arc<::std::sync::Mutex<Vec<String>>>,
}
