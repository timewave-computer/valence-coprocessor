use alloc::vec::Vec;
use valence_coprocessor_types::{ControllerData, DomainData};

use crate::{DataBackend, ExecutionContext, Hash, Hasher, Permission, Vm, ZkVm};

/// Artifacts repository.
pub struct Registry<D: DataBackend> {
    data: D,
}

impl<D: DataBackend + Clone> Clone for Registry<D> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}

impl<D: DataBackend> Registry<D> {
    /// Data backend prefix for controller data.
    pub const PREFIX_CONTROLLER: &[u8] = b"registry-controller";

    /// Data backend prefix for zkVM data.
    pub const PREFIX_CIRCUIT: &[u8] = b"registry-circuit";

    /// Register a new controller, returning its identifier.
    pub fn register_controller<M, H, Z>(
        &self,
        vm: &M,
        zkvm: &Z,
        ctx: &ExecutionContext<H, D>,
        controller: ControllerData,
    ) -> anyhow::Result<Hash>
    where
        M: Vm<H, D>,
        H: Hasher,
        Z: ZkVm<Hasher = H>,
    {
        let id = controller.identifier();

        ctx.ensure(&Permission::CircuitControllerWrite(id))?;

        let ControllerData {
            controller,
            circuit,
            ..
        } = controller;

        self.data.set(Self::PREFIX_CONTROLLER, &id, &controller)?;
        self.data.set(Self::PREFIX_CIRCUIT, &id, &circuit)?;

        vm.updated(&id);
        zkvm.updated(&id);

        Ok(id)
    }

    /// Register a new domain, returning its identifier.
    pub fn register_domain<M, H, Z>(
        &self,
        vm: &M,
        zkvm: &Z,
        ctx: &ExecutionContext<H, D>,
        domain: DomainData,
    ) -> anyhow::Result<Hash>
    where
        M: Vm<H, D>,
        H: Hasher,
        Z: ZkVm<Hasher = H>,
    {
        let id = domain.identifier();

        ctx.ensure(&Permission::CircuitControllerWrite(id))?;

        let DomainData {
            controller,
            circuit,
            ..
        } = domain;

        self.data.set(Self::PREFIX_CONTROLLER, &id, &controller)?;
        self.data.set(Self::PREFIX_CIRCUIT, &id, &circuit)?;

        vm.updated(&id);
        zkvm.updated(&id);

        Ok(id)
    }

    /// Returns the associated controller, if present.
    pub fn get_controller(&self, id: &Hash) -> anyhow::Result<Option<Vec<u8>>> {
        self.data.get(Self::PREFIX_CONTROLLER, id)
    }

    /// Returns the associated circuit, if present.
    pub fn get_zkvm(&self, id: &Hash) -> anyhow::Result<Option<Vec<u8>>> {
        self.data.get(Self::PREFIX_CIRCUIT, id)
    }
}

impl<D: DataBackend> From<D> for Registry<D> {
    fn from(data: D) -> Self {
        Self { data }
    }
}
