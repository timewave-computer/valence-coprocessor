use alloc::{rc::Rc, vec::Vec};

use msgpacker::Unpackable as _;
use serde_json::Value;

use crate::{
    Blake3Hasher, DataBackend, DomainData, DomainOpening, Hash, Hasher, ProvenProgram, Registry,
    Smt, StateProof, ValidatedDomainBlock, Vm, Witness, WitnessCoprocessor, ZkVm,
};

pub use buf_fs::{File, FileSystem};

/// Execution context with blake3 hasher.
pub type Blake3Context<D, M, Z> = ExecutionContext<Blake3Hasher, D, M, Z>;

struct ExecutionContextInner<H, D, M, Z>
where
    H: Hasher,
    D: DataBackend,
    M: Vm<H, D, Z>,
    Z: ZkVm<Hasher = H>,
{
    data: D,
    registry: Registry<D>,
    historical: Smt<D, H>,
    historical_root: Hash,
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
    Z: ZkVm<Hasher = H>,
{
    inner: Rc<ExecutionContextInner<H, D, M, Z>>,
}

impl<H, D, M, Z> Clone for ExecutionContext<H, D, M, Z>
where
    H: Hasher,
    D: DataBackend,
    M: Vm<H, D, Z>,
    Z: ZkVm<Hasher = H>,
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
    Z: ZkVm<Hasher = H>,
{
    /// Data backend prefix for the historical SMT.
    pub const PREFIX_SMT: &[u8] = b"smt-historical";

    /// Data backend prefix for the latest block of a domain.
    pub const PREFIX_BLOCK: &[u8] = b"smt-domain-block";

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

    /// Compute the ZK proof of the provided program.
    pub fn get_program_proof(&self, args: Value) -> anyhow::Result<ProvenProgram> {
        let library = self.library();

        tracing::debug!("computing library proof for `{:x?}`...", library);

        let witnesses = self
            .inner
            .vm
            .execute(self, library, Self::LIB_GET_WITNESSES, args)?;

        tracing::debug!("inner library executed; parsing...");

        let witnesses: Vec<Witness> = serde_json::from_value(witnesses)?;

        tracing::debug!("witnesses computed from library...");

        let root = self.inner.historical_root;
        let proofs = witnesses
            .iter()
            .filter_map(|w| match w {
                Witness::StateProof(p) => Some(p),
                _ => None,
            })
            .map(|p| {
                let key = H::key(&p.domain, &p.root);

                self.inner
                    .historical
                    .get_opening(root, &key)?
                    .map(|opening| DomainOpening {
                        domain: p.domain.clone(),
                        root: p.root,
                        payload: p.payload.clone(),
                        opening,
                    })
                    .ok_or_else(|| anyhow::anyhow!("failed to compute the domain proof"))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let witness = WitnessCoprocessor {
            root,
            proofs,
            witnesses,
        };

        tracing::debug!("co-processor witnesses computed...");

        self.inner.zkvm.prove(self, witness)
    }

    /// Returns the program verifying key.
    pub fn get_program_verifying_key(&self) -> anyhow::Result<Vec<u8>> {
        self.inner.zkvm.verifying_key(self)
    }

    /// Computes a state proof with the provided arguments.
    pub fn get_state_proof(&self, domain: &str, args: Value) -> anyhow::Result<StateProof> {
        tracing::debug!("fetching state proof for `{domain}` with {args:?}...");

        let domain = DomainData::identifier_from_parts(domain);
        let proof = self
            .inner
            .vm
            .execute(self, &domain, Self::LIB_GET_STATE_PROOF, args)?;

        tracing::debug!("state proof fetched from domain.");

        Ok(serde_json::from_value(proof)?)
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
    pub fn get_storage(&self) -> anyhow::Result<FileSystem> {
        let raw = self.get_raw_storage()?;

        match raw {
            Some(r) => Ok(FileSystem::from_raw_device_unchecked(r)),
            None => FileSystem::new(256 * 1024 * 1024),
        }
    }

    /// Overrides the library storage.
    pub fn set_storage(&self, fs: &FileSystem) -> anyhow::Result<()> {
        let fs = fs.try_to_raw_device()?;

        self.set_raw_storage(&fs)
    }

    /// Returns the library storage file from the given path.
    pub fn get_storage_file(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        self.get_storage()
            .and_then(|mut fs| fs.open(path))
            .map(|f| f.contents)
    }

    /// Overrides the library storage file.
    pub fn set_storage_file(&self, path: &str, contents: &[u8]) -> anyhow::Result<()> {
        let mut fs = self.get_storage()?;

        tracing::debug!("saving storage file to path `{path}`");

        if let Err(e) = fs.save(File::new(path.into(), contents.to_vec(), true)) {
            tracing::debug!("error saving storage file to path `{path}`: {e}");
        }

        self.set_storage(&fs)
    }

    /// Returns the library raw storage.
    pub fn get_raw_storage(&self) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner
            .data
            .get_bulk(Self::PREFIX_LIB, &self.inner.library)
    }

    /// Overrides the library raw storage.
    pub fn set_raw_storage(&self, storage: &[u8]) -> anyhow::Result<()> {
        self.inner
            .data
            .set_bulk(Self::PREFIX_LIB, &self.inner.library, storage)
            .map(|_| ())
    }

    /// Returns the last included block for the provided domain.
    pub fn get_latest_block(&self, domain: &str) -> anyhow::Result<Option<ValidatedDomainBlock>> {
        let domain = DomainData::identifier_from_parts(domain);
        let block = self.inner.data.get(Self::PREFIX_BLOCK, &domain)?;
        let block = block
            .map(|b| ValidatedDomainBlock::unpack(&b).map(|(_, b)| b))
            .transpose()
            .map_err(|e| anyhow::anyhow!("failed to parse validated block: {e}"))?;

        Ok(block)
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
    D: DataBackend,
    M: Vm<H, D, Z>,
    Z: ZkVm<Hasher = H>,
{
    /// Initializes a new execution context.
    #[allow(dead_code)]
    pub(crate) fn init(library: Hash, historical_root: Hash, data: D, vm: M, zkvm: Z) -> Self {
        Self {
            inner: Rc::new(ExecutionContextInner {
                data: data.clone(),
                historical: Smt::from(data.clone()),
                historical_root,
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
