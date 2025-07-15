use alloc::vec::Vec;
use msgpacker::MsgPacker;
use serde::{Deserialize, Serialize};

use crate::{DataBackend, ExecutionContext, Hash, Hasher, Opening, Proof, StateProof, Witness};

/// A domain opening co-processor witness.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker)]
pub struct DomainOpening {
    /// Proven state.
    pub proof: StateProof,

    /// Block payload.
    pub payload: Vec<u8>,

    /// Opening proof to root.
    pub opening: Opening,
}

/// A circuit witness data obtained via Valence API.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker)]
pub struct WitnessCoprocessor {
    /// Co-processor historical commitments root.
    pub root: Hash,

    /// Openings to the historical tree.
    pub proofs: Vec<DomainOpening>,

    /// Witness data for the circuit.
    pub witnesses: Vec<Witness>,
}

impl WitnessCoprocessor {
    /// Validates the co-processor witness, yielding verified state proofs & data for the circuit.
    pub fn validate<H: Hasher>(mut self) -> anyhow::Result<ValidatedWitnesses> {
        for o in &self.proofs {
            let key = H::key(&o.proof.domain, &o.proof.root);
            let value = H::hash(&o.payload);

            tracing::debug!("verifying domain opening for {key:x?}, {value:x?}");

            anyhow::ensure!(o.opening.verify::<H>(&self.root, &key, &value));
        }

        let mut proofs = self.proofs.into_iter().map(|o| o.proof);

        for w in self.witnesses.iter_mut() {
            if let Witness::StateProof(s) = w {
                *s = proofs
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("no state proof available"))?;
            }
        }

        Ok(ValidatedWitnesses {
            root: self.root,
            witnesses: self.witnesses,
        })
    }
}

/// Co-processor validated witnesses.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ValidatedWitnesses {
    /// Co-processor historical commitments root.
    pub root: Hash,

    /// Witness data for the circuit.
    pub witnesses: Vec<Witness>,
}

/// A zkVM definition.
pub trait ZkVm: Clone + Sized {
    /// Friendly hasher of the zkVM.
    type Hasher: Hasher;

    /// Prove a given circuit.
    ///
    /// ## Arguments
    ///
    /// - `ctx`: Execution context to fetch the controller bytes from.
    /// - `circuit`: Circuit unique identifier.
    /// - `witnesses`: Circuit arguments.
    fn prove<D>(
        &self,
        ctx: &ExecutionContext<Self::Hasher, D>,
        w: WitnessCoprocessor,
    ) -> anyhow::Result<Proof>
    where
        D: DataBackend;

    /// Returns the verifying key for the given circuit.
    ///
    /// ## Arguments
    ///
    /// - `ctx`: Execution context to fetch the controller bytes from.
    /// - `circuit`: Circuit unique identifier.
    fn verifying_key<D>(&self, ctx: &ExecutionContext<Self::Hasher, D>) -> anyhow::Result<Vec<u8>>
    where
        D: DataBackend;

    /// A notification that the circuit has been updated.
    fn updated(&self, circuit: &Hash);
}
