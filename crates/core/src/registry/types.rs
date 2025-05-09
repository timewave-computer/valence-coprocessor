use alloc::{string::String, vec, vec::Vec};
use msgpacker::MsgPacker;
use serde::{Deserialize, Serialize};

use crate::{Blake3Hasher, DataBackend, Hash, Hasher, SmtOpening};

use super::Registry;

/// The unique identifier of a domain that is supported by Valence programs.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DomainData {
    /// Name of the domain
    pub name: String,
    /// Library used to compute the domain functions.
    pub lib: Vec<u8>,
}

impl DomainData {
    /// Prefix for the domain identifier hash.
    pub const ID_PREFIX: &[u8] = b"domain";

    /// Creates a new domain with the provided name a identifier.
    pub fn new(name: String) -> Self {
        Self { name, lib: vec![] }
    }

    /// Associates the provided library with the domain.
    pub fn with_lib(mut self, lib: Vec<u8>) -> Self {
        self.lib = lib;
        self
    }

    /// Generates an unique identifier for the domain.
    ///
    /// The library definition can be hot swapped so it is not part of the identifier
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
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProgramData {
    /// Library containing the witness computation functions.
    pub lib: Vec<u8>,
    /// Circuit containing the transition function.
    pub circuit: Vec<u8>,
    /// Deployed nonce value.
    pub nonce: u64,
}

impl ProgramData {
    /// Prefix for the program identifier hash.
    pub const ID_PREFIX: &[u8] = b"program";

    /// Generates an unique identifier for the program.
    ///
    /// The library file does not extend the security properties of the zkVM program so it is
    /// not included within the scope of identification, as it can be freely replaced without
    /// causing breaking changes.
    pub fn identifier(&self) -> Hash {
        Self::identifier_from_parts(&self.circuit, self.nonce)
    }

    /// Computes the program identifier from its parts.
    pub fn identifier_from_parts(circuit: &[u8], nonce: u64) -> Hash {
        Blake3Hasher::digest([Self::ID_PREFIX, circuit, &nonce.to_le_bytes()])
    }

    /// Set the library execution definition.
    pub fn with_lib(mut self, lib: Vec<u8>) -> Self {
        self.lib = lib;
        self
    }

    /// Set the zkvm execution definition.
    pub fn with_circuit(mut self, circuit: Vec<u8>) -> Self {
        self.circuit = circuit;
        self
    }

    /// Set the id computation nonce.
    pub fn with_nonce(mut self, nonce: u64) -> Self {
        self.nonce = nonce;
        self
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

impl Witness {
    /// Returns the data, if the correct variation is met.
    pub fn as_data(&self) -> Option<&[u8]> {
        match self {
            Witness::Data(d) => Some(d.as_slice()),
            _ => None,
        }
    }

    /// Returns the domain proof, if the correct variation is met.
    pub fn as_domain_proof(&self) -> Option<&SmtOpening> {
        match self {
            Witness::DomainProof(p) => Some(p),
            _ => None,
        }
    }

    /// Returns the state proof, if the correct variation is met.
    pub fn as_state_proof(&self) -> Option<&[u8]> {
        match self {
            Witness::StateProof(p) => Some(p.as_slice()),
            _ => None,
        }
    }
}

/// A ZK proven program.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, MsgPacker)]
pub struct ProvenProgram {
    /// The target ZK proof.
    pub proof: Vec<u8>,
    /// The output arguments.
    pub outputs: Vec<u8>,
}

/// A domain validated block
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker)]
pub struct ValidatedBlock {
    /// A block associated number.
    ///
    /// This is the block number of Ethereum, the slot of Solana, etc.
    pub number: u64,

    /// The hash root of the block.
    pub root: Hash,

    /// Block blob payload.
    pub payload: Vec<u8>,
}

impl<D: DataBackend> From<D> for Registry<D> {
    fn from(data: D) -> Self {
        Self { data }
    }
}
