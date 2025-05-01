use alloc::{rc::Rc, vec::Vec};

use base64::{engine::general_purpose::STANDARD as Base64, Engine as _};
use serde_json::Value;

use crate::{
    Blake3Hasher, DataBackend, DomainData, Hash, Hasher, ProvenProgram, Registry, Smt, SmtOpening,
    ValidatedBlock, Vm, Witness, ZkVm,
};

/// Execution context with blake3 hasher.
pub type Blake3Context<D, M, Z> = ExecutionContext<Blake3Hasher, D, M, Z>;

struct ExecutionContextInner<H, D, M, Z>
where
    H: Hasher,
    D: DataBackend,
    M: Vm<H, D, Z>,
    Z: ZkVm,
{
    data: D,
    registry: Registry<D>,
    historical: Smt<D, H>,
    vm: M,
    zkvm: Z,
    library: Hash,

    #[cfg(feature = "std")]
    log: ::std::sync::Mutex<Vec<String>>,
}

/// Execution context for a Valence library.
pub struct ExecutionContext<H, D, M, Z>
where
    H: Hasher,
    D: DataBackend,
    M: Vm<H, D, Z>,
    Z: ZkVm,
{
    inner: Rc<ExecutionContextInner<H, D, M, Z>>,
}

impl<H, D, M, Z> Clone for ExecutionContext<H, D, M, Z>
where
    H: Hasher,
    D: DataBackend,
    M: Vm<H, D, Z>,
    Z: ZkVm,
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
    M: Vm<H, D, Z>,
    Z: ZkVm,
{
    /// Data backend prefix for the historical SMT.
    pub const PREFIX_SMT: &[u8] = b"smt-historical";

    /// Data backend prefix for the latest block of a domain.
    pub const PREFIX_BLOCK: &[u8] = b"smt-domain-block";

    /// Data backend prefix for historical root associated domain.
    pub const PREFIX_SMT_ROOT: &[u8] = b"context-smt-root";

    /// Data backend prefix for the context library data.
    pub const PREFIX_LIB: &[u8] = b"context-library";

    /// Library function name to get witnesses.
    pub const LIB_GET_WITNESSES: &str = "get_witnesses";

    /// Library function name to get state proofs.
    pub const LIB_GET_STATE_PROOF: &str = "get_state_proof";

    /// Library function name to validate blocks.
    pub const LIB_VALIDATE_BLOCK: &str = "validate_block";

    /// Library function name to the entrypoint.
    pub const LIB_ENTRYPOINT: &str = "entrypoint";

    /// Returns the library being executed.
    pub fn library(&self) -> &Hash {
        &self.inner.library
    }

    /// Returns a zkVM circuit.
    pub fn get_zkvm(&self) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.registry.get_zkvm(&self.inner.library)
    }

    /// Returns a library.
    pub fn get_lib(&self, lib: &Hash) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.registry.get_lib(lib)
    }

    /// Returns a domain library.
    pub fn get_domain_lib(&self, domain: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let domain = DomainData::identifier_from_parts(domain);

        self.inner.registry.get_lib(&domain)
    }

    /// Computes a domain opening for the target root.
    pub fn get_domain_proof(&self, domain: &str) -> anyhow::Result<Option<SmtOpening>> {
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
    pub fn get_program_proof(&self, args: Value) -> anyhow::Result<ProvenProgram> {
        let library = self.library();

        tracing::debug!("computing library proof for `{:x?}`...", library);

        let witnesses = self
            .inner
            .vm
            .execute(self, library, Self::LIB_GET_WITNESSES, args)?;

        tracing::debug!("inner library executed; parsing...");

        let witnesses = serde_json::from_value(witnesses)?;

        tracing::debug!("witnesses computed from library...");

        self.inner.zkvm.prove(self, witnesses)
    }

    /// Returns the program verifying key.
    pub fn get_program_verifying_key(&self) -> anyhow::Result<Vec<u8>> {
        self.inner.zkvm.verifying_key(self)
    }

    /// Computes a state proof with the provided arguments.
    pub fn get_state_proof(&self, domain: &str, args: Value) -> anyhow::Result<Vec<u8>> {
        let domain = DomainData::identifier_from_parts(domain);
        let proof = self
            .inner
            .vm
            .execute(self, &domain, Self::LIB_GET_STATE_PROOF, args)?;

        let proof = proof.as_str().ok_or_else(|| {
            anyhow::anyhow!(
                "the domain library didn't return a valid state proof base64 representation"
            )
        })?;

        Base64
            .decode(proof)
            .map_err(|e| anyhow::anyhow!("error decoding the proof bytes: {e}"))
    }

    /// Get the program witness data for the ZK circuit.
    pub fn get_program_witnesses(&self, args: Value) -> anyhow::Result<Vec<Witness>> {
        let witnesses =
            self.inner
                .vm
                .execute(self, &self.inner.library, Self::LIB_GET_WITNESSES, args)?;

        Ok(serde_json::from_value(witnesses)?)
    }

    /// Returns the library storage.
    pub fn get_storage(&self) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.data.get(Self::PREFIX_LIB, &self.inner.library)
    }

    /// Overrides the library storage.
    pub fn set_storage(&self, storage: &[u8]) -> anyhow::Result<()> {
        self.inner
            .data
            .set(Self::PREFIX_LIB, &self.inner.library, storage)
            .map(|_| ())
    }

    /// Returns the most recent historical SMT for the provided domain.
    pub fn get_domain_smt(&self, domain: &str) -> anyhow::Result<Hash> {
        let domain = DomainData::identifier_from_parts(domain);
        let smt = self.inner.data.get(Self::PREFIX_SMT_ROOT, &domain)?;
        let smt = smt
            .map(|b| Hash::try_from(b.as_slice()))
            .transpose()?
            .unwrap_or_else(|| Smt::<D, H>::empty_tree_root());

        Ok(smt)
    }

    /// Returns the last included block for the provided domain.
    pub fn get_latest_block(&self, domain: &str) -> anyhow::Result<Option<ValidatedBlock>> {
        let domain = DomainData::identifier_from_parts(domain);
        let block = self.inner.data.get(Self::PREFIX_BLOCK, &domain)?;

        Ok(block.map(|b| serde_json::from_slice(&b)).transpose()?)
    }

    /// Adds a new block to the provided domain.
    pub fn add_domain_block(&self, domain: &str, args: Value) -> anyhow::Result<()> {
        let id = DomainData::identifier_from_parts(domain);
        let block = self
            .inner
            .vm
            .execute(self, &id, Self::LIB_VALIDATE_BLOCK, args)?;

        let block: ValidatedBlock = serde_json::from_value(block)?;

        let smt = self.get_domain_smt(domain)?;
        let smt = self
            .inner
            .historical
            .insert(smt, domain, &block.root, block.payload.clone())?;

        self.inner.data.set(Self::PREFIX_SMT_ROOT, &id, &smt)?;

        let latest = self.get_latest_block(domain)?;

        match latest {
            // all domains are assumed to be monotonically increasing.
            // Solana, the common exception, has `slot`.
            Some(b) if b.number > block.number => (),
            _ => {
                let block = serde_json::to_vec(&block)?;

                self.inner.data.set(Self::PREFIX_BLOCK, &id, &block)?;
            }
        }

        Ok(())
    }

    #[cfg(feature = "std")]
    /// Returns the internal logs of the context.
    pub fn get_log(&self) -> anyhow::Result<Vec<String>> {
        self.inner
            .log
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to lock logs: {e}"))
            .map(|l| l.clone())
    }

    #[cfg(feature = "std")]
    /// Replaces the internal logs of the context.
    pub fn extend_log<I>(&self, log: I) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = String>,
    {
        self.inner
            .log
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to lock logs: {e}"))?
            .extend(log);

        Ok(())
    }

    /// Calls the entrypoint of the library with the provided arguments.
    pub fn entrypoint(&self, args: Value) -> anyhow::Result<Value> {
        self.inner
            .vm
            .execute(self, self.library(), Self::LIB_ENTRYPOINT, args)
    }
}

impl<H, D, M, Z> ExecutionContext<H, D, M, Z>
where
    H: Hasher,
    D: DataBackend + Clone,
    M: Vm<H, D, Z>,
    Z: ZkVm,
{
    /// Initializes a new execution context.
    pub fn init(library: Hash, data: D, vm: M, zkvm: Z) -> Self {
        Self {
            inner: Rc::new(ExecutionContextInner {
                data: data.clone(),
                historical: Smt::from(data.clone()),
                registry: Registry::from(data.clone()),
                vm,
                zkvm,
                library,

                #[cfg(feature = "std")]
                log: Vec::with_capacity(10).into(),
            }),
        }
    }
}

#[cfg(feature = "mocks")]
impl<H, D, M, Z> ExecutionContext<H, D, M, Z>
where
    H: Hasher,
    D: DataBackend,
    M: Vm<H, D, Z>,
    Z: ZkVm,
{
    /// Executes an arbitrary library function.
    pub fn execute_lib(&self, lib: &Hash, f: &str, args: Value) -> anyhow::Result<Value> {
        self.inner.vm.execute(self, lib, f, args)
    }

    /// Computes an arbitrary program proof.
    pub fn execute_proof(&self, witnesses: Vec<Witness>) -> anyhow::Result<ProvenProgram> {
        self.inner.zkvm.prove(self, witnesses)
    }
}
