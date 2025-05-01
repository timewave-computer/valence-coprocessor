use alloc::vec::Vec;
use valence_coprocessor::Hash;

/// A Ethereum domain definition.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Ethereum;

/// A Ethereum state proof.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "msgpacker", derive(msgpacker::MsgPacker))]
pub struct EthereumStateProof {
    /// The root of the opening.
    pub root: Hash,

    /// The Merkle opening to the root
    pub opening: Vec<Vec<u8>>,

    /// The leaf key.
    pub key: Vec<u8>,

    /// The leaf value.
    pub value: Vec<u8>,
}

/// A proven key value opening to the state root.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "msgpacker", derive(msgpacker::MsgPacker))]
pub struct EthereumCircuitOutput {
    /// Root of the key-value opening.
    pub root: Hash,
    /// Leaf key.
    pub key: Vec<u8>,
    /// Leaf value.
    pub value: Vec<u8>,
}

#[cfg(feature = "ethereum-circuit")]
mod circuit {
    use msgpacker::Unpackable as _;
    use valence_coprocessor::Witness;
    use valence_zk_proofs::merkle::types::MerkleVerifiable as _;
    use valence_zk_proofs_ethereum::merkle_lib::types::EthereumMerkleProof;

    use crate::DomainCircuit;

    use super::*;

    impl DomainCircuit for Ethereum {
        type Output = EthereumCircuitOutput;

        fn verify(proof: &Witness) -> Option<Self::Output> {
            let proof = proof.as_state_proof()?;
            let proof = EthereumStateProof::unpack(proof).ok()?.1;

            let EthereumStateProof {
                root,
                opening,
                key,
                value,
            } = proof;

            let proof = EthereumMerkleProof {
                proof: opening,
                key,
                value,
            };

            proof.verify(&root).ok()?;

            Some(EthereumCircuitOutput {
                root,
                key: proof.key,
                value: proof.value,
            })
        }
    }
}

#[cfg(feature = "ethereum-lib")]
mod lib {
    use msgpacker::Packable as _;
    use serde_json::Value;

    use crate::DomainLibrary;

    use super::*;

    impl DomainLibrary for Ethereum {
        const ID: &str = "ethereum-alpha";

        fn state_proof_bytes(&self, args: Value) -> anyhow::Result<Vec<u8>> {
            // TODO fetch the state proof from some RPC
            let proof: EthereumStateProof = serde_json::from_value(args)?;

            Ok(proof.pack_to_vec())
        }
    }
}
