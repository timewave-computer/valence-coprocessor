use alloc::{string::String, vec, vec::Vec};
use msgpacker::{MsgPacker, Packable as _, Unpackable};
use serde::{Deserialize, Serialize};

use crate::{Base64, Blake3Hasher, DataBackend, Hash, Hasher};

use super::Registry;

/// The unique identifier of a domain that is supported by Valence programs.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DomainData {
    /// Name of the domain
    pub name: String,
    /// Controller used to compute the domain functions.
    pub controller: Vec<u8>,
}

impl DomainData {
    /// Prefix for the domain identifier hash.
    pub const ID_PREFIX: &[u8] = b"domain";

    /// Creates a new domain with the provided name a identifier.
    pub fn new(name: String) -> Self {
        Self {
            name,
            controller: vec![],
        }
    }

    /// Associates the provided controller with the domain.
    pub fn with_controller(mut self, controller: Vec<u8>) -> Self {
        self.controller = controller;
        self
    }

    /// Generates an unique identifier for the domain.
    ///
    /// The controller definition can be hot swapped so it is not part of the identifier
    /// computation.
    pub fn identifier(&self) -> Hash {
        Self::identifier_from_parts(&self.name)
    }

    /// Computes the domain identifier from its parts.
    pub fn identifier_from_parts(name: &str) -> Hash {
        Blake3Hasher::digest([Self::ID_PREFIX, name.as_bytes()])
    }
}

/// Controller data of the registry.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ControllerData {
    /// Controller containing the witness computation functions.
    pub controller: Vec<u8>,
    /// Circuit containing the transition function.
    pub circuit: Vec<u8>,
    /// Deployed nonce value.
    pub nonce: u64,
}

impl ControllerData {
    /// Prefix for the controller identifier hash.
    pub const ID_PREFIX: &[u8] = b"controller";

    /// Generates an unique identifier for the controller.
    ///
    /// The controller file does not extend the security properties of the zkVM controller so it is
    /// not included within the scope of identification, as it can be freely replaced without
    /// causing breaking changes.
    pub fn identifier(&self) -> Hash {
        Self::identifier_from_parts(&self.circuit, self.nonce)
    }

    /// Computes the controller identifier from its parts.
    pub fn identifier_from_parts(circuit: &[u8], nonce: u64) -> Hash {
        Blake3Hasher::digest([Self::ID_PREFIX, circuit, &nonce.to_le_bytes()])
    }

    /// Set the controller execution definition.
    pub fn with_controller(mut self, controller: Vec<u8>) -> Self {
        self.controller = controller;
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

/// A domain-specific state proof.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker)]
pub struct StateProof {
    /// Domain of the proof.
    pub domain: String,

    /// Domain root of the proof.
    pub root: Hash,

    /// An arbitrary payload for the block.
    pub payload: Vec<u8>,

    /// A serialized, domain-specific proof.
    pub proof: Vec<u8>,
}

/// A circuit witness data obtained via Valence API.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker)]
pub enum Witness {
    /// A domain opening of a state argument to its root.
    StateProof(StateProof),

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

    /// Returns the state proof, if the correct variation is met.
    pub fn as_state_proof(&self) -> Option<&StateProof> {
        match self {
            Witness::StateProof(p) => Some(p),
            _ => None,
        }
    }
}

/// A ZK proven circuit.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, MsgPacker)]
pub struct Proof {
    /// The base64 encoded ZK proof.
    pub proof: String,

    /// The base64 encoded public inputs of the proof.
    pub inputs: String,
}

impl Proof {
    /// Encodes the arguments and returns a new proven circuit instance.
    pub fn new<P, I>(proof: P, inputs: I) -> Self
    where
        P: AsRef<[u8]>,
        I: AsRef<[u8]>,
    {
        Self {
            proof: Base64::encode(proof.as_ref()),
            inputs: Base64::encode(inputs.as_ref()),
        }
    }

    /// Encodes the proven circuit into base64.
    pub fn to_base64(&self) -> String {
        let bytes = self.pack_to_vec();

        Base64::encode(bytes)
    }

    /// Try to parse the proven circuit from a base64 string.
    pub fn try_from_base64<B: AsRef<str>>(b64: B) -> anyhow::Result<Self> {
        let bytes = Base64::decode(b64)?;

        Ok(Self::unpack(&bytes)
            .map_err(|e| anyhow::anyhow!("failed to unpack proof: {e}"))?
            .1)
    }

    /// Decodes the base64 bytes of the proof and public inputs.
    pub fn decode(&self) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
        let proof = Base64::decode(&self.proof)?;
        let inputs = Base64::decode(&self.inputs)?;

        Ok((proof, inputs))
    }
}

/// A domain validated block
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker)]
pub struct ValidatedBlock {
    /// A block associated number.
    pub number: u64,

    /// The hash root of the block.
    pub root: Hash,

    /// Block blob payload.
    pub payload: Vec<u8>,
}

/// A domain validated block
#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker,
)]
pub struct ValidatedDomainBlock {
    /// The domain associated with the block.
    pub domain: Hash,

    /// A block associated number.
    pub number: u64,

    /// The hash root of the block.
    pub root: Hash,

    /// SMT key to index the payload.
    pub key: Hash,

    /// Block blob payload.
    pub payload: Vec<u8>,
}

impl<D: DataBackend> From<D> for Registry<D> {
    fn from(data: D) -> Self {
        Self { data }
    }
}

/// A confirmation of an added block.
#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker,
)]
pub struct BlockAdded {
    /// Domain to which the block was added.
    pub domain: String,
    /// Historical SMT root prior to the mutation.
    pub prev_smt: Hash,
    /// Historical SMT root after the mutation.
    pub smt: Hash,
    /// Controller execution log.
    pub log: Vec<String>,
    /// Block data.
    pub block: ValidatedDomainBlock,
}

#[test]
fn proof_base64_encode_works() {
    let proof_bytes = b"foo";
    let inputs = b"bar";

    let proof = Proof::new(proof_bytes, inputs);

    let p = proof.to_base64();
    let p = Proof::try_from_base64(p).unwrap();

    assert_eq!(proof, p);

    let (p, i) = proof.decode().unwrap();

    assert_eq!(p, proof_bytes);
    assert_eq!(i, inputs);
}
