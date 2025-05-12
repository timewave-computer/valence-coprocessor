#![no_std]

use alloc::vec;
use serde_json::Value;
use valence_coprocessor::{DomainCircuit, DomainLibrary, Hasher as _, StateProof, ValidatedBlock};
use valence_coprocessor_sp1::Sp1Hasher;

extern crate alloc;

pub const ID: &str = "domain";

/// A domain definition.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Domain;

impl Domain {
    pub fn new_block(number: u64, value: u64) -> ValidatedBlock {
        let payload = value.to_le_bytes().to_vec();
        let root = Sp1Hasher::hash(&payload);

        ValidatedBlock {
            number,
            root,
            payload,
        }
    }
}

impl DomainLibrary for Domain {
    const ID: &str = ID;

    fn state_proof(&self, args: Value) -> anyhow::Result<StateProof> {
        let value = args
            .get("value")
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("value not provided"))?;

        let payload = value.to_le_bytes().to_vec();
        let root = Sp1Hasher::hash(&payload);
        let domain = ID.into();
        let proof = vec![];

        Ok(StateProof {
            domain,
            root,
            payload,
            proof,
        })
    }
}

impl DomainCircuit for Domain {
    type Output = u64;

    fn verify(proof: &StateProof) -> anyhow::Result<Self::Output> {
        let value = TryFrom::try_from(proof.payload.as_slice())?;
        let value = u64::from_le_bytes(value);
        let root = Sp1Hasher::hash(&proof.payload);

        anyhow::ensure!(ID == proof.domain);
        anyhow::ensure!(root == proof.root);

        Ok(value)
    }
}

#[test]
fn domain_is_consistent() {
    let value = 378249u64;
    let proof = Domain
        .state_proof(serde_json::json!({
            "value": value
        }))
        .unwrap();
    let val = Domain::verify(&proof).unwrap();

    assert_eq!(value, val);
}
