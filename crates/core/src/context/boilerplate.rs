use core::marker::PhantomData;

use alloc::vec::Vec;
use serde_json::Value;
use valence_coprocessor_types::{DataBackend, DomainData, Hash, Hasher};

use crate::{ExecutionContext, Registry, Vm};

impl<H, D> ExecutionContext<H, D>
where
    H: Hasher,
    D: DataBackend,
{
    /// Data backend prefix for the context controller data.
    pub const PREFIX_CONTROLLER: &[u8] = b"context-controller";

    /// Controller function name to the entrypoint.
    pub const CONTROLLER_ENTRYPOINT: &str = "entrypoint";

    /// Initializes a new execution context.
    #[allow(dead_code)]
    pub(crate) fn init(controller: Hash, historical: Hash, data: D) -> Self {
        Self {
            data: data.clone(),
            controller,
            hasher: PhantomData,
            historical,
            registry: Registry::from(data.clone()),
            owner: None,

            #[cfg(feature = "std")]
            log: ::std::sync::Arc::new(Vec::with_capacity(10).into()),
        }
    }

    /// Returns a reference to the data backend.
    pub fn data(&self) -> &D {
        &self.data
    }

    /// Returns the controller being executed.
    pub fn controller(&self) -> &Hash {
        &self.controller
    }

    /// Returns a zkVM circuit.
    pub fn get_zkvm(&self) -> anyhow::Result<Option<Vec<u8>>> {
        self.registry.get_zkvm(&self.controller)
    }

    /// Replaces the internal controller with the provided id.
    pub fn with_controller(mut self, controller: Hash) -> Self {
        self.controller = controller;
        self
    }

    /// Replaces the internal historical with the provided root.
    pub fn with_historical(mut self, historical: Hash) -> Self {
        self.historical = historical;
        self
    }

    /// Returns a controller.
    pub fn get_controller(&self, controller: &Hash) -> anyhow::Result<Option<Vec<u8>>> {
        self.registry.get_controller(controller)
    }

    /// Returns a domain controller.
    pub fn get_domain_controller(&self, domain: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let domain = DomainData::identifier_from_parts(domain);

        self.registry.get_controller(&domain)
    }

    /// Returns the current historical SMT root.
    pub fn get_historical(&self) -> Hash {
        self.historical
    }

    #[cfg(feature = "std")]
    /// Returns the internal logs of the context.
    pub fn get_log(&self) -> anyhow::Result<Vec<String>> {
        self.log
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
        self.log
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
