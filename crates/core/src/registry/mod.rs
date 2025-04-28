use alloc::{collections::BTreeSet, vec::Vec};

use crate::{DataBackend, Hash, Hasher, Vm, ZkVM};

use zerocopy::{IntoBytes as _, TryFromBytes};

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
    /// Data backend prefix for domain data.
    pub const PREFIX_DOMAIN: &[u8] = b"registry-domain";

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
        Z: ZkVM,
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
        Z: ZkVM,
    {
        let id = domain.identifier();
        let DomainData { lib, .. } = domain;

        self.data.set(Self::PREFIX_LIB, &id, &lib)?;

        vm.updated(&id);

        Ok(id)
    }

    /// Returns the list of linked domains of the program.
    pub fn get_program_domains(&self, program: &Hash) -> anyhow::Result<BTreeSet<Hash>> {
        let domains = self
            .data
            .get(Self::PREFIX_DOMAIN, program)?
            .unwrap_or_default();

        let hashes = <[Hash]>::try_ref_from_bytes(&domains)
            .map_err(|_| anyhow::anyhow!("failed reading the stored domains"))?
            .to_vec();

        Ok(hashes.into_iter().collect())
    }

    /// Links a program to be submitted to the given domains.
    pub fn program_link(&self, program: &Hash, domains: &[Hash]) -> anyhow::Result<()> {
        let mut domains_list = self.get_program_domains(program)?;

        domains_list.extend(domains);

        let domains: Vec<_> = domains_list.iter().copied().collect();
        let domains = domains.as_bytes();

        self.data.set(Self::PREFIX_DOMAIN, program, domains)?;

        Ok(())
    }

    /// Unlink a program to no longer be submitted to the given domains.
    pub fn program_unlink(&self, program: &Hash, domains: &[Hash]) -> anyhow::Result<()> {
        let mut domains_list = self.get_program_domains(program)?;

        for d in domains {
            domains_list.remove(d);
        }

        let domains: Vec<_> = domains_list.iter().copied().collect();
        let domains = domains.as_bytes();

        if domains.is_empty() {
            self.data.remove(Self::PREFIX_DOMAIN, program)?;
        } else {
            self.data.set(Self::PREFIX_DOMAIN, program, domains)?;
        }

        Ok(())
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
