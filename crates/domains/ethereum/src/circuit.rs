use msgpacker::Unpackable as _;
use valence_coprocessor::{DomainCircuit, Witness};
use valence_zk_proofs::merkle::types::MerkleVerifiable as _;
use valence_zk_proofs_ethereum::merkle_lib::types::EthereumMerkleProof;

use crate::{Ethereum, EthereumCircuitOutput, EthereumStateProof};

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
