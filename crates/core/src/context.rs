use alloc::{rc::Rc, vec::Vec};

use msgpacker::Unpackable as _;
use serde_json::Value;

use crate::{
    Blake3Hasher, DataBackend, DomainData, DomainOpening, Hash, Hasher, Proof, Registry, Smt,
    StateProof, ValidatedDomainBlock, Vm, Witness, WitnessCoprocessor, ZkVm,
};

pub use buf_fs::{File, FileSystem};

/// Execution context with blake3 hasher.
pub type Blake3Context<D> = ExecutionContext<Blake3Hasher, D>;

struct ExecutionContextInner<H, D>
where
    H: Hasher,
    D: DataBackend,
{
    data: D,
    registry: Registry<D>,
    historical: Smt<D, H>,
    historical_root: Hash,
    controller: Hash,

    #[cfg(feature = "std")]
    log: ::std::sync::Mutex<Vec<String>>,
}

/// Execution context for a Valence controller.
pub struct ExecutionContext<H, D>
where
    H: Hasher,
    D: DataBackend,
{
    inner: Rc<ExecutionContextInner<H, D>>,
}

impl<H, D> Clone for ExecutionContext<H, D>
where
    H: Hasher,
    D: DataBackend,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<H, D> ExecutionContext<H, D>
where
    H: Hasher,
    D: DataBackend,
{
    /// Data backend prefix for the historical SMT.
    pub const PREFIX_SMT: &[u8] = b"smt-historical";

    /// Data backend prefix for the latest block of a domain.
    pub const PREFIX_BLOCK: &[u8] = b"smt-domain-block";

    /// Data backend prefix for the context controller data.
    pub const PREFIX_CONTROLLER: &[u8] = b"context-controller";

    /// Controller function name to get witnesses.
    pub const CONTROLLER_GET_WITNESSES: &str = "get_witnesses";

    /// Controller function name to get state proofs.
    pub const CONTROLLER_GET_STATE_PROOF: &str = "get_state_proof";

    /// Controller function name to validate blocks.
    pub const CONTROLLER_VALIDATE_BLOCK: &str = "validate_block";

    /// Controller function name to the entrypoint.
    pub const CONTROLLER_ENTRYPOINT: &str = "entrypoint";

    /// Returns the controller being executed.
    pub fn controller(&self) -> &Hash {
        &self.inner.controller
    }

    /// Returns a zkVM circuit.
    pub fn get_zkvm(&self) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.registry.get_zkvm(&self.inner.controller)
    }

    /// Returns a controller.
    pub fn get_controller(&self, controller: &Hash) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.registry.get_controller(controller)
    }

    /// Returns a domain controller.
    pub fn get_domain_controller(&self, domain: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let domain = DomainData::identifier_from_parts(domain);

        self.inner.registry.get_controller(&domain)
    }

    /// Compute the ZK proof of the provided circuit.
    pub fn get_proof<VM, ZK>(&self, vm: &VM, zkvm: &ZK, args: Value) -> anyhow::Result<Proof>
    where
        VM: Vm<H, D>,
        ZK: ZkVm<Hasher = H>,
    {
        let controller = self.controller();

        tracing::debug!("computing controller proof for `{:x?}`...", controller);

        let witnesses = vm.execute(self, controller, Self::CONTROLLER_GET_WITNESSES, args)?;

        tracing::debug!("inner controller executed; parsing...");

        let witnesses: Vec<Witness> = serde_json::from_value(witnesses)?;

        tracing::debug!("witnesses computed from controller...");

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

        zkvm.prove(self, witness)
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
        tracing::debug!("fetching state proof for `{domain}` with {args:?}...");

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
        let witnesses = vm.execute(
            self,
            &self.inner.controller,
            Self::CONTROLLER_GET_WITNESSES,
            args,
        )?;

        Ok(serde_json::from_value(witnesses)?)
    }

    /// Returns the controller storage.
    pub fn get_storage(&self) -> anyhow::Result<FileSystem> {
        let raw = self.get_raw_storage()?;

        match raw {
            Some(r) => Ok(FileSystem::from_raw_device_unchecked(r)),
            None => FileSystem::new(256 * 1024 * 1024),
        }
    }

    /// Overrides the controller storage.
    pub fn set_storage(&self, fs: &FileSystem) -> anyhow::Result<()> {
        let fs = fs.try_to_raw_device()?;

        self.set_raw_storage(&fs)
    }

    /// Returns the controller storage file from the given path.
    pub fn get_storage_file(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        self.get_storage()
            .and_then(|mut fs| fs.open(path))
            .map(|f| f.contents)
    }

    /// Overrides the controller storage file.
    pub fn set_storage_file(&self, path: &str, contents: &[u8]) -> anyhow::Result<()> {
        let mut fs = self.get_storage()?;

        tracing::debug!("saving storage file to path `{path}`");

        if let Err(e) = fs.save(File::new(path.into(), contents.to_vec(), true)) {
            tracing::debug!("error saving storage file to path `{path}`: {e}");
        }

        self.set_storage(&fs)
    }

    /// Returns the controller raw storage.
    pub fn get_raw_storage(&self) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner
            .data
            .get_bulk(Self::PREFIX_CONTROLLER, &self.inner.controller)
    }

    /// Overrides the controller raw storage.
    pub fn set_raw_storage(&self, storage: &[u8]) -> anyhow::Result<()> {
        self.inner
            .data
            .set_bulk(Self::PREFIX_CONTROLLER, &self.inner.controller, storage)
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

    /// Calls the entrypoint of the controller with the provided arguments.
    pub fn entrypoint<VM>(&self, vm: &VM, args: Value) -> anyhow::Result<Value>
    where
        VM: Vm<H, D>,
    {
        vm.execute(self, self.controller(), Self::CONTROLLER_ENTRYPOINT, args)
    }
}

impl<H, D> ExecutionContext<H, D>
where
    H: Hasher,
    D: DataBackend,
{
    /// Initializes a new execution context.
    #[allow(dead_code)]
    pub(crate) fn init(controller: Hash, historical_root: Hash, data: D) -> Self {
        Self {
            inner: Rc::new(ExecutionContextInner {
                data: data.clone(),
                historical: Smt::from(data.clone()),
                historical_root,
                registry: Registry::from(data.clone()),
                controller,

                #[cfg(feature = "std")]
                log: Vec::with_capacity(10).into(),
            }),
        }
    }
}
