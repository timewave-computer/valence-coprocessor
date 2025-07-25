use std::net::TcpStream;

use msgpacker::{MsgPacker, Unpackable};
use serde::{Deserialize, Serialize};
use sp1_core_executor::SP1ReduceProof;
use sp1_sdk::{SP1Proof, SP1ProofWithPublicValues, SP1VerifyingKey};
use sp1_stark::{baby_bear_poseidon2::BabyBearPoseidon2, StarkVerifyingKey};
use tungstenite::WebSocket;
use valence_coprocessor::{Base64, Hash, Proof};

use crate::client::Client;

/// A circuit definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, MsgPacker)]
pub enum Circuit {
    /// A cached circuit identifier.
    Identifier(Hash),

    /// An ELF circuit definition.
    Elf {
        /// Custom identifier of the circuit.
        identifier: Hash,

        /// ELF bytes.
        bytes: String,
    },
}

impl From<Hash> for Circuit {
    fn from(id: Hash) -> Self {
        Self::Identifier(id)
    }
}

/// Jobs that can be accepted by a worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, MsgPacker)]
pub enum Request {
    /// SP1 groth16 proof
    Sp1Proof {
        /// Proving circuit
        circuit: Circuit,
        /// Circuit witnesses (base64)
        witnesses: String,
        /// Target proof type
        t: ProofType,
        /// A base64 encoded Vec<RecursiveProof>
        recursive: String,
    },

    /// Get the SP1 verifying key.
    Sp1GetVerifyingKey {
        /// Proving circuit
        circuit: Circuit,
    },

    /// Close the connection
    Close,
}

/// Possible states resulting of a proof request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, MsgPacker)]
pub enum Response {
    /// Successfully executed a command without output.
    Ack,

    /// The provided circuit proving key was not found in the cache.
    ///
    /// The service should provide the full proving key.
    ProvingKeyNotCached,

    /// The proof result (base64)
    Proof(String),

    /// The verifying key (base64)
    VerifyingKey(String),

    /// An error has occurred.
    Err(String),
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Task {
    /// Connection request
    Conn(WebSocket<TcpStream>),

    /// Quit the worker thread
    Quit,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RecursiveProof {
    pub proof: SP1ReduceProof<BabyBearPoseidon2>,
    pub vk: StarkVerifyingKey<BabyBearPoseidon2>,
}

impl RecursiveProof {
    /// Encode a sequence of proofs into a base64 string.
    pub fn encode(proofs: Vec<Self>) -> String {
        serde_cbor::to_vec(&proofs).map(Base64::encode).unwrap()
    }

    /// Decoes a sequence of proofs from a base64 string.
    pub fn decode<B>(encoded: B) -> anyhow::Result<Vec<Self>>
    where
        B: AsRef<str>,
    {
        let bytes = Base64::decode(encoded)?;
        let proofs = serde_cbor::from_slice(&bytes)?;

        Ok(proofs)
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ProofRequestBuilder {
    circuit: Hash,
    t: ProofType,
    witnesses: Vec<u8>,
    recursive: Vec<RecursiveProof>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, MsgPacker)]
pub enum ProofType {
    Compressed,

    #[default]
    Groth16,
}

impl ProofRequestBuilder {
    pub fn new(circuit: Hash) -> Self {
        Self {
            circuit,
            ..Default::default()
        }
    }

    pub fn with_recursive_proof(
        mut self,
        proof: SP1ProofWithPublicValues,
        vk: SP1VerifyingKey,
    ) -> anyhow::Result<Self> {
        let proof = match proof.proof {
            SP1Proof::Compressed(p) => *p,
            _ => anyhow::bail!("unsupported proof type"),
        };

        self.recursive.push(RecursiveProof { proof, vk: vk.vk });

        Ok(self)
    }

    pub fn with_witnesses<W>(mut self, witnesses: W) -> Self
    where
        W: AsRef<[u8]>,
    {
        self.witnesses = witnesses.as_ref().to_vec();
        self
    }

    pub fn with_type(mut self, t: ProofType) -> Self {
        self.t = t;
        self
    }

    pub fn prove<F>(self, client: &Client, elf: F) -> anyhow::Result<Proof>
    where
        F: FnOnce(&Hash) -> anyhow::Result<Vec<u8>>,
    {
        client.get_sp1_proof(self.circuit, self.t, &self.witnesses, &self.recursive, elf)
    }
}
