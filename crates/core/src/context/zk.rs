use alloc::vec::Vec;
use serde_json::Value;
use valence_coprocessor_types::{DataBackend, DomainData, Hasher, StateProof, Witness};

use crate::{ExecutionContext, Vm, WitnessCoprocessor, ZkVm};

impl<H, D> ExecutionContext<H, D>
where
    H: Hasher,
    D: DataBackend,
{
    /// Controller function name to get witnesses.
    pub const CONTROLLER_GET_WITNESSES: &str = "get_witnesses";

    /// Controller function name to get state proofs.
    pub const CONTROLLER_GET_STATE_PROOF: &str = "get_state_proof";

    /// Computes the circuit witnesses.
    pub fn get_circuit_witnesses<VM>(&self, vm: &VM, args: Value) -> anyhow::Result<Vec<Witness>>
    where
        VM: Vm<H, D>,
    {
        let controller = self.controller();

        tracing::debug!("computing controller witnesses for `{:x?}`...", controller);

        let witnesses = vm.execute(self, controller, Self::CONTROLLER_GET_WITNESSES, args)?;

        tracing::trace!("inner controller executed; parsing `{witnesses:?}`...");

        let witnesses = serde_json::from_value(witnesses)?;

        tracing::debug!("witnesses vector parsed...");

        Ok(witnesses)
    }

    /// Compute the ZK proof of the provided circuit.
    pub fn get_coprocessor_witness(
        &self,
        witnesses: Vec<Witness>,
    ) -> anyhow::Result<WitnessCoprocessor> {
        WitnessCoprocessor::try_from_witnesses::<H, D>(
            self.data.clone(),
            self.historical,
            witnesses,
        )
    }

    /// Returns the circuit verifying key.
    pub fn get_verifying_key<ZK>(&self, zkvm: &ZK) -> anyhow::Result<Vec<u8>>
    where
        ZK: ZkVm<Hasher = H>,
    {
        zkvm.verifying_key(self)
    }

    /// Computes a state proof with the provided arguments.
    pub fn get_state_proof<VM>(
        &self,
        vm: &VM,
        domain: &str,
        args: Value,
    ) -> anyhow::Result<StateProof>
    where
        VM: Vm<H, D>,
    {
        tracing::debug!("fetching state proof for `{domain}`...");
        tracing::trace!("args {args:?}...");

        let domain = DomainData::identifier_from_parts(domain);
        let proof = vm.execute(self, &domain, Self::CONTROLLER_GET_STATE_PROOF, args)?;

        tracing::debug!("state proof fetched from domain.");

        Ok(serde_json::from_value(proof)?)
    }

    /// Get the witnesses from the controller, to the ZK circuit.
    pub fn get_witnesses<VM>(&self, vm: &VM, args: Value) -> anyhow::Result<Vec<Witness>>
    where
        VM: Vm<H, D>,
    {
        let witnesses = vm.execute(self, &self.controller, Self::CONTROLLER_GET_WITNESSES, args)?;

        Ok(serde_json::from_value(witnesses)?)
    }
}
