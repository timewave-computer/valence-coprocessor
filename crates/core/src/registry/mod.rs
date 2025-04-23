use alloc::{collections::BTreeSet, vec::Vec};

use crate::{DataBackend, Hash};

use zerocopy::{IntoBytes as _, TryFromBytes};

mod types;

pub use types::*;

// TODO define an owner of the program and domain, accepting mutation to its data only when a valid
// signature is provided.

/// Program registry repository.
pub struct Registry<D: DataBackend> {
    data: D,
}

impl<D: DataBackend> Registry<D> {
    /// Data backend prefix for domain data.
    pub const PREFIX_DOMAIN: &[u8] = b"registry-domain";

    /// Data backend prefix for module data.
    pub const PREFIX_MODULE: &[u8] = b"registry-module";

    /// Data backend prefix for zkVM data.
    pub const PREFIX_ZKVM: &[u8] = b"registry-zkvm";

    /// Register a new program, returning its identifier.
    pub fn register_program(&self, program: ProgramData) -> anyhow::Result<Hash> {
        let id = program.identifier();
        let ProgramData { module, zkvm, .. } = program;

        self.data.set(Self::PREFIX_MODULE, &id, &module)?;
        self.data.set(Self::PREFIX_ZKVM, &id, &zkvm)?;

        Ok(id)
    }

    /// Register a new domain, returning its identifier.
    pub fn register_domain(&self, domain: DomainData) -> anyhow::Result<Hash> {
        let id = domain.identifier();
        let DomainData { module, .. } = domain;

        self.data.set(Self::PREFIX_MODULE, &id, &module)?;

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

    /// Returns the associated module module, if present.
    pub fn get_module(&self, id: &Hash) -> anyhow::Result<Option<Vec<u8>>> {
        self.data.get(Self::PREFIX_MODULE, id)
    }

    /// Returns the associated zkVM module, if present.
    pub fn get_zkvm(&self, id: &Hash) -> anyhow::Result<Option<Vec<u8>>> {
        self.data.get(Self::PREFIX_ZKVM, id)
    }
}
