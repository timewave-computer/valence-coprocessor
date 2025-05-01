use msgpacker::Packable as _;
use serde_json::Value;
use valence_coprocessor::DomainLibrary;

use super::*;

impl DomainLibrary for Ethereum {
    const ID: &str = "ethereum-alpha";

    fn state_proof_bytes(&self, args: Value) -> anyhow::Result<Vec<u8>> {
        // TODO fetch the state proof from some RPC
        let proof: EthereumStateProof = serde_json::from_value(args)?;

        Ok(proof.pack_to_vec())
    }
}
