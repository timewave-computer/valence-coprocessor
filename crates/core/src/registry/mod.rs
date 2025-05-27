use alloc::vec::Vec;

use crate::{DataBackend, Hash, Hasher, Vm, ZkVm};

mod types;

pub use types::*;

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
        controller: ControllerData,
    ) -> anyhow::Result<Hash>
    where
        M: Vm<H, D>,
        H: Hasher,
        Z: ZkVm<Hasher = H>,
    {
        let id = controller.identifier();
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
    pub fn register_domain<M, H>(&self, vm: &M, domain: DomainData) -> anyhow::Result<Hash>
    where
        M: Vm<H, D>,
        H: Hasher,
    {
        let id = domain.identifier();
        let DomainData { controller, .. } = domain;

        self.data.set(Self::PREFIX_CONTROLLER, &id, &controller)?;

        vm.updated(&id);

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
