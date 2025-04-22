use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};

use crate::{Blake3Hasher, DataBackend, Hash, Hasher, SmtOpening};

use super::Registry;

/// The unique identifier of a domain that is supported by Valence programs.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DomainData {
    /// Name of the domain
    pub name: String,
    /// Module module used to compute the domain functions.
    pub module: Vec<u8>,
}

impl DomainData {
    /// Prefix for the domain identifier hash.
    pub const ID_PREFIX: &[u8] = b"domain";

    /// Generates an unique identifier for the domain.
    ///
    /// The module module definition can be hot swapped so it is not part of the identifier
    /// computation.
    pub fn identifier(&self) -> Hash {
        Self::identifier_from_parts(&self.name)
    }

    /// Computes the domain identifier from its parts.
    pub fn identifier_from_parts(name: &str) -> Hash {
        Blake3Hasher::digest([Self::ID_PREFIX, name.as_bytes()])
    }
}

/// Program data of the registry.
pub struct ProgramData {
    /// Module module containing the witness computation functions.
    pub module: Vec<u8>,
    /// ZKVM module containing the proven transition code.
    pub zkvm: Vec<u8>,
    /// Deployed nonce value.
    pub nonce: u64,
}

impl ProgramData {
    /// Prefix for the program identifier hash.
    pub const ID_PREFIX: &[u8] = b"program";

    /// Generates an unique identifier for the program.
    ///
    /// The module file does not extend the security properties of the zkVM program so it is
    /// not included within the scope of identification, as it can be freely replaced without
    /// causing breaking changes.
    pub fn identifier(&self) -> Hash {
        Self::identifier_from_parts(&self.zkvm, self.nonce)
    }

    /// Computes the program identifier from its parts.
    pub fn identifier_from_parts(zkvm: &[u8], nonce: u64) -> Hash {
        Blake3Hasher::digest([Self::ID_PREFIX, zkvm, &nonce.to_le_bytes()])
    }
}

/// A structured on-chain message, tailored for a domain.
pub struct Message {
    /// The target domain for the message.
    pub domain: Hash,
    /// The transition proof.
    pub proof: Vec<u8>,
    /// The arguments of the message.
    pub outputs: Vec<u8>,
}

/// A program witness data obtained via Valence API.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Witness {
    /// A domain opening of a state argument to its root.
    StateProof(Vec<u8>),

    /// A historical commitments opening to the root.
    DomainProof(SmtOpening),

    /// Arbitrary execution data.
    Data(Vec<u8>),
}

/// A ZK proven program.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProvenProgram {
    /// The target ZK proof.
    pub proof: Vec<u8>,
    /// The output arguments.
    pub outputs: Vec<u8>,
}

impl<D: DataBackend> From<D> for Registry<D> {
    fn from(data: D) -> Self {
        Self { data }
    }
}
