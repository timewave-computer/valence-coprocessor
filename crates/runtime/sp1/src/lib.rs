#![cfg_attr(not(feature = "std"), no_std)]

use sha2_v0_10_8::{Digest, Sha256};
use valence_coprocessor::{Hash, Hasher};

#[cfg(feature = "host")]
mod host;

#[cfg(feature = "host")]
pub use host::*;

// disabled for 5.0.0
//#[cfg(feature = "ark-groth16")]
//mod groth16;
//
//#[cfg(feature = "ark-groth16")]
//pub use groth16::*;

#[derive(Debug, Clone)]
pub struct Sp1Hasher;

impl Sp1Hasher {
    /// Prefix for data hash.
    pub const DATA_PREFIX: &[u8] = &[0x00];

    /// Prefix for node hash.
    pub const MERGE_PREFIX: &[u8] = &[0x01];
}

impl Hasher for Sp1Hasher {
    fn key(context: &str, data: &[u8]) -> Hash {
        let mut hasher = Sha256::new();

        hasher.update(context);
        hasher.update(data);

        hasher.finalize().into()
    }

    fn hash(data: &[u8]) -> Hash {
        let mut hasher = Sha256::new();

        hasher.update(Self::DATA_PREFIX);
        hasher.update(data);

        hasher.finalize().into()
    }

    fn merge(a: &Hash, b: &Hash) -> Hash {
        let mut hasher = Sha256::new();

        hasher.update(Self::MERGE_PREFIX);
        hasher.update(a);
        hasher.update(b);

        hasher.finalize().into()
    }

    fn digest<'a>(data: impl IntoIterator<Item = &'a [u8]>) -> Hash {
        let mut hasher = Sha256::new();

        hasher.update(Self::DATA_PREFIX);

        data.into_iter().for_each(|d| {
            hasher.update(d);
        });

        hasher.finalize().into()
    }
}
