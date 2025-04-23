use alloc::{rc::Rc, vec::Vec};

use base64::{engine::general_purpose::STANDARD as Base64, Engine as _};
use serde_json::Value;

use crate::{
    Blake3Hasher, DataBackend, DomainData, Hash, Hasher, ModuleVM, ProvenProgram, Registry, Smt,
    SmtOpening, Witness, ZkVM,
};

/// Execution context with blake3 hasher.
pub type Blake3Context<D, M, Z> = ExecutionContext<Blake3Hasher, D, M, Z>;

struct ExecutionContextInner<H, D, M, Z>
where
    H: Hasher,
    D: DataBackend,
    M: ModuleVM<H, D, Z>,
    Z: ZkVM,
{
    data: D,
    registry: Registry<D>,
    historical: Smt<D, H>,
    module: M,
    zkvm: Z,
    program: Hash,
}

/// Execution context for a Valence program.
pub struct ExecutionContext<H, D, M, Z>
where
    H: Hasher,
    D: DataBackend,
    M: ModuleVM<H, D, Z>,
    Z: ZkVM,
{
    inner: Rc<ExecutionContextInner<H, D, M, Z>>,
}

impl<H, D, M, Z> Clone for ExecutionContext<H, D, M, Z>
where
    H: Hasher,
    D: DataBackend,
    M: ModuleVM<H, D, Z>,
    Z: ZkVM,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<H, D, M, Z> ExecutionContext<H, D, M, Z>
where
    H: Hasher,
    D: DataBackend,
    M: ModuleVM<H, D, Z>,
    Z: ZkVM,
{
    /// Returns the program being executed.
    pub fn program(&self) -> &Hash {
        &self.inner.program
    }

    /// Returns a module program.
    pub fn get_zkvm(&self) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.registry.get_zkvm(&self.inner.program)
    }

    /// Returns a module program.
    pub fn get_module(&self, module: &Hash) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.registry.get_module(module)
    }

    /// Returns a domain module program.
    pub fn get_domain_module(&self, domain: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let domain = DomainData::identifier_from_parts(domain);

        self.inner.registry.get_module(&domain)
    }

    /// Computes a domain opening for the target root.
    pub fn compute_domain_proof(&self, domain: &str) -> anyhow::Result<Option<SmtOpening>> {
        let domain = DomainData::identifier_from_parts(domain);
        let tree = match self.inner.historical.get_key_root(&domain)? {
            Some(t) => t,
            None => return Ok(None),
        };

        self.inner
            .historical
            .get_opening("historical", tree, &domain)
    }

    /// Compute the ZK proof of the provided program.
    pub fn compute_program_proof(&self, args: Value) -> anyhow::Result<ProvenProgram> {
        let program = self.program();
        let witnesses = self
            .inner
            .module
            .execute(self, program, "get_witnesses", args)?;
        let witnesses = serde_json::from_value(witnesses)?;

        self.inner.zkvm.prove(self, witnesses)
    }

    /// Computes a state proof with the provided arguments.
    pub fn get_state_proof(&self, domain: &str, args: Value) -> anyhow::Result<Vec<u8>> {
        let domain = DomainData::identifier_from_parts(domain);
        let proof = self
            .inner
            .module
            .execute(self, &domain, "get_state_proof", args)?;

        let proof = proof.as_str().ok_or_else(|| {
            anyhow::anyhow!(
                "the domain module didn't return a valid state proof base64 representation"
            )
        })?;

        Base64
            .decode(proof)
            .map_err(|e| anyhow::anyhow!("error decoding the proof bytes: {e}"))
    }

    /// Get the program witness data for the ZK circuit.
    pub fn get_program_witnesses(
        &self,
        program: &Hash,
        args: Value,
    ) -> anyhow::Result<Vec<Witness>> {
        let witnesses = self
            .inner
            .module
            .execute(self, program, "get_witnesses", args)?;

        Ok(serde_json::from_value(witnesses)?)
    }

    /// Returns the program storage.
    pub fn get_program_storage(&self) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.data.get(b"context-program", &self.inner.program)
    }

    /// Overrides the program storage.
    pub fn set_program_storage(&self, storage: &[u8]) -> anyhow::Result<()> {
        self.inner
            .data
            .set(b"context-program", &self.inner.program, storage)
            .map(|_| ())
    }
}

impl<H, D, M, Z> ExecutionContext<H, D, M, Z>
where
    H: Hasher,
    D: DataBackend + Clone,
    M: ModuleVM<H, D, Z>,
    Z: ZkVM,
{
    /// Initializes a new execution context.
    pub fn init(program: Hash, data: D, module: M, zkvm: Z) -> Self {
        Self {
            inner: Rc::new(ExecutionContextInner {
                data: data.clone(),
                historical: Smt::from(data.clone()),
                registry: Registry::from(data.clone()),
                module,
                zkvm,
                program,
            }),
        }
    }
}

#[cfg(feature = "mocks")]
impl<H, D, M, Z> ExecutionContext<H, D, M, Z>
where
    H: Hasher,
    D: DataBackend,
    M: ModuleVM<H, D, Z>,
    Z: ZkVM,
{
    /// Executes an arbitrary program function.
    pub fn execute_module(&self, program: &Hash, f: &str, args: Value) -> anyhow::Result<Value> {
        self.inner.module.execute(self, program, f, args)
    }

    /// Computes an arbitrary program proof.
    pub fn execute_proof(&self, witnesses: Vec<Witness>) -> anyhow::Result<ProvenProgram> {
        self.inner.zkvm.prove(self, witnesses)
    }
}
