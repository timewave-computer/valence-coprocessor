use alloc::vec::Vec;

use crate::{DataBackend, Hash, Hasher, Vm, ZkVm};

mod types;

pub use types::*;

// TODO define an owner of the program and domain, accepting mutation to its data only when a valid
// signature is provided.

/// Program registry repository.
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
    /// Data backend prefix for lib data.
    pub const PREFIX_LIB: &[u8] = b"registry-lib";

    /// Data backend prefix for zkVM data.
    pub const PREFIX_CIRCUIT: &[u8] = b"registry-circuit";

    /// Register a new program, returning its identifier.
    pub fn register_program<M, H, Z>(
        &self,
        vm: &M,
        zk_vm: &Z,
        program: ProgramData,
    ) -> anyhow::Result<Hash>
    where
        M: Vm<H, D, Z>,
        H: Hasher,
        Z: ZkVm<Hasher = H>,
    {
        let id = program.identifier();
        let ProgramData { lib, circuit, .. } = program;

        self.data.set(Self::PREFIX_LIB, &id, &lib)?;
        self.data.set(Self::PREFIX_CIRCUIT, &id, &circuit)?;

        vm.updated(&id);
        zk_vm.updated(&id);

        Ok(id)
    }

    /// Register a new domain, returning its identifier.
    pub fn register_domain<M, H, Z>(&self, vm: &M, domain: DomainData) -> anyhow::Result<Hash>
    where
        M: Vm<H, D, Z>,
        H: Hasher,
        Z: ZkVm<Hasher = H>,
    {
        let id = domain.identifier();
        let DomainData { lib, .. } = domain;

        self.data.set(Self::PREFIX_LIB, &id, &lib)?;

        vm.updated(&id);

        Ok(id)
    }

    /// Returns the associated library, if present.
    pub fn get_lib(&self, id: &Hash) -> anyhow::Result<Option<Vec<u8>>> {
        self.data.get(Self::PREFIX_LIB, id)
    }

    /// Returns the associated circuit, if present.
    pub fn get_zkvm(&self, id: &Hash) -> anyhow::Result<Option<Vec<u8>>> {
        self.data.get(Self::PREFIX_CIRCUIT, id)
    }
}
