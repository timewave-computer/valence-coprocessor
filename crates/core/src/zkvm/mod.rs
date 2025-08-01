use alloc::vec::Vec;
use msgpacker::MsgPacker;
use serde::{Deserialize, Serialize};
use valence_coprocessor_merkle::CompoundOpening;
use valence_coprocessor_types::{StateProof, ValidatedWitnesses, Witness};

use crate::{DataBackend, ExecutionContext, Hash, Hasher, Historical, Proof};

#[cfg(test)]
mod tests;

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

/// A domain opening co-processor witness.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker)]
pub struct DomainOpening {
    /// Proven state.
    pub proof: StateProof,

    /// Opening proof to the coprocessor root.
    pub opening: CompoundOpening,
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
    /// Attemtps to create an instance from a set of witnesses.
    ///
    /// Will compute the domain opening for every state proof.
    pub fn try_from_witnesses<H, D>(
        data: D,
        root: Hash,
        witnesses: Vec<Witness>,
    ) -> anyhow::Result<Self>
    where
        H: Hasher,
        D: DataBackend,
    {
        let proofs = witnesses
            .iter()
            .filter_map(|w| w.as_state_proof().cloned())
            .map(|proof| {
                let opening = Historical::<H, D>::get_block_proof_with_historical(
                    data.clone(),
                    root,
                    proof.domain,
                    proof.number,
                )?;

                Ok(DomainOpening { proof, opening })
            })
            .collect::<anyhow::Result<_>>()?;

        Ok(Self {
            root,
            proofs,
            witnesses,
        })
    }

    /// Validates the co-processor witness, yielding verified state proofs & data for the circuit.
    pub fn validate<H: Hasher>(mut self) -> anyhow::Result<ValidatedWitnesses> {
        let mut witnesses = self.witnesses.iter_mut();

        for p in self.proofs {
            let root = Historical::<H, ()>::compute_root(&p.opening, &p.proof.state_root);
            let domain = Historical::<H, ()>::get_domain_id(&p.opening)
                .ok_or_else(|| anyhow::anyhow!("failed to compute domain id"))?;

            anyhow::ensure!(root == self.root, "invalid opening to root");
            anyhow::ensure!(domain == p.proof.domain, "unexpected domain");

            let mut w;

            loop {
                w = witnesses
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("witnesses set depleted"))?;

                if w.as_state_proof().is_some() {
                    break;
                }
            }

            *w = Witness::StateProof(p.proof);
        }

        Ok(ValidatedWitnesses {
            root: self.root,
            witnesses: self.witnesses,
        })
    }
}
