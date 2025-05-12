use msgpacker::Unpackable as _;
use valence_coprocessor::{DomainCircuit, StateProof};
use valence_zk_proofs::merkle::types::MerkleVerifiable as _;
use valence_zk_proofs_ethereum::merkle_lib::types::EthereumMerkleProof;

use crate::{Ethereum, EthereumCircuitOutput, EthereumStateProof};

impl DomainCircuit for Ethereum {
    type Output = EthereumCircuitOutput;

    fn verify(proof: &StateProof) -> anyhow::Result<Self::Output> {
        let root = proof.root;
        let proof = EthereumStateProof::unpack(&proof.proof)
            .map_err(|e| anyhow::anyhow!("failed to deserialize ethereum state proof: {e}"))?
            .1;

        let EthereumStateProof {
            opening,
            key,
            value,
        } = proof;

        let proof = EthereumMerkleProof {
            proof: opening,
            key,
            value,
        };

        anyhow::ensure!(proof.verify(&root)?);

        Ok(EthereumCircuitOutput {
            key: proof.key,
            value: proof.value,
        })
    }
}
