#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod data;
mod hash;
mod registry;
mod smt;

pub use data::*;
pub use hash::*;
pub use registry::*;
pub use smt::*;

use alloc::vec::Vec;

use base64::{engine::general_purpose::STANDARD as Base64, Engine as _};
use serde_json::Value;

/// Execution context for a Valence program.
pub struct ExecutionContext<H, D, M, Z>
where
    H: Hasher,
    D: DataBackend,
    M: ModuleVM,
    Z: ZkVM,
{
    registry: Registry<D>,
    historical: Smt<D, H>,
    module: M,
    zkvm: Z,
    program: Hash,
}

impl<H, D, M, Z> ExecutionContext<H, D, M, Z>
where
    H: Hasher,
    D: DataBackend,
    M: ModuleVM,
    Z: ZkVM,
{
    /// Returns the program being executed.
    pub fn program(&self) -> &Hash {
        &self.program
    }
}

impl<H, D, M, Z> ExecutionContext<H, D, M, Z>
where
    H: Hasher,
    D: DataBackend + Clone,
    M: ModuleVM,
    Z: ZkVM,
{
    /// Initializes a new execution context.
    pub fn init(program: Hash, data: D, module: M, zkvm: Z) -> Self {
        Self {
            historical: Smt::from(data.clone()),
            registry: Registry::from(data.clone()),
            module,
            zkvm,
            program,
        }
    }

    /// Returns a domain module program.
    pub fn get_domain_module(&self, domain: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let domain = DomainData::identifier_from_parts(domain);

        self.registry.get_module(&domain)
    }

    /// Computes a domain opening for the target root.
    pub fn compute_domain_proof(&self, domain: &str) -> anyhow::Result<Option<SmtOpening>> {
        let domain = DomainData::identifier_from_parts(domain);
        let tree = match self.historical.get_key_root(&domain)? {
            Some(t) => t,
            None => return Ok(None),
        };

        self.historical.get_opening("historical", tree, &domain)
    }

    /// Compute the ZK proof of the provided program.
    pub fn compute_program_proof(
        &self,
        program: &Hash,
        args: Value,
    ) -> anyhow::Result<ProvenProgram> {
        let witnesses = self.module.execute(self, program, "get_witnesses", args)?;
        let witnesses = serde_json::from_value(witnesses)?;

        self.zkvm.prove(self, program, witnesses)
    }

    /// Computes a state proof with the provided arguments.
    pub fn get_state_proof(&self, domain: &str, args: Value) -> anyhow::Result<Vec<u8>> {
        let domain = DomainData::identifier_from_parts(domain);
        let proof = self
            .module
            .execute(self, &domain, "get_state_proof", args)?;

        let proof = proof.as_str().ok_or_else(|| {
            anyhow::anyhow!(
                "the domain module didn't return a valid state proof base64 representation"
            )
        })?;

        Ok(Base64.decode(proof)?)
    }

    /// Get the program witness data for the ZK circuit.
    pub fn get_program_witnesses(
        &self,
        program: &Hash,
        args: Value,
    ) -> anyhow::Result<Vec<Witness>> {
        let witnesses = self.module.execute(self, program, "get_witnesses", args)?;

        Ok(serde_json::from_value(witnesses)?)
    }
}

/// A module VM definition.
pub trait ModuleVM: Sized {
    /// Execute a function in a module.
    ///
    /// Returns the output of the function call.
    ///
    /// ## Arguments
    ///
    /// - `ctx`: Execution context to fetch the module bytes from.
    /// - `module`: Module unique identifier.
    /// - `f`: Function name to be called.
    /// - `args`: Arguments to be passed to the function call.
    fn execute<H, D, Z>(
        &self,
        ctx: &ExecutionContext<H, D, Self, Z>,
        module: &Hash,
        f: &str,
        args: Value,
    ) -> anyhow::Result<Value>
    where
        H: Hasher,
        D: DataBackend,
        Z: ZkVM;
}

/// A zkVM definition.
pub trait ZkVM: Sized {
    /// Prove a given program.
    ///
    /// ## Arguments
    ///
    /// - `ctx`: Execution context to fetch the module bytes from.
    /// - `program`: Program unique identifier.
    /// - `witnesses`: Circuit arguments.
    fn prove<H, D, M>(
        &self,
        ctx: &ExecutionContext<H, D, M, Self>,
        program: &Hash,
        witnesses: Vec<Witness>,
    ) -> anyhow::Result<ProvenProgram>
    where
        H: Hasher,
        D: DataBackend,
        M: ModuleVM;
}
