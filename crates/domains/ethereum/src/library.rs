use msgpacker::Packable as _;
use serde_json::Value;
use valence_coprocessor::{DomainLibrary, ValidatedBlock};
use valence_coprocessor_wasm::abi;

use super::*;

impl DomainLibrary for Ethereum {
    const ID: &str = "ethereum-alpha";

    fn state_proof_bytes(&self, args: Value) -> anyhow::Result<Vec<u8>> {
        let ValidatedBlock { root, payload, .. } = abi::get_latest_block(Self::ID)?
            .ok_or_else(|| anyhow::anyhow!("failed to fetch latest proven block of the domain"))?;

        // TODO instead, should fetch from some service
        let payload: Value = serde_json::from_slice(&payload)?;

        let opening = payload
            .get("opening")
            .map(|o| serde_json::from_value(o.clone()))
            .transpose()?
            .ok_or_else(|| anyhow::anyhow!("failed to get the opening from the payload"))?;

        let key = args
            .get("key")
            .map(|o| serde_json::from_value(o.clone()))
            .transpose()?
            .ok_or_else(|| anyhow::anyhow!("failed to get the key from the payload"))?;

        let value = payload
            .get("value")
            .map(|o| serde_json::from_value(o.clone()))
            .transpose()?
            .ok_or_else(|| anyhow::anyhow!("failed to get the value from the payload"))?;

        let proof = EthereumStateProof {
            root,
            opening,
            key,
            value,
        };

        Ok(proof.pack_to_vec())
    }
}
