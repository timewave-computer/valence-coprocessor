use alloc::vec::Vec;

use crate::{DataBackend, ExecutionContext, Hash, Hasher, ProvenProgram, Vm, Witness};

/// A zkVM definition.
pub trait ZkVM: Sized {
    /// Prove a given program.
    ///
    /// ## Arguments
    ///
    /// - `ctx`: Execution context to fetch the library bytes from.
    /// - `program`: Program unique identifier.
    /// - `witnesses`: Circuit arguments.
    fn prove<H, D, M>(
        &self,
        ctx: &ExecutionContext<H, D, M, Self>,
        witnesses: Vec<Witness>,
    ) -> anyhow::Result<ProvenProgram>
    where
        H: Hasher,
        D: DataBackend,
        M: Vm<H, D, Self>;

    /// Returns the verifying key for the given program.
    ///
    /// ## Arguments
    ///
    /// - `ctx`: Execution context to fetch the library bytes from.
    /// - `program`: Program unique identifier.
    fn verifying_key<H, D, M>(
        &self,
        ctx: &ExecutionContext<H, D, M, Self>,
    ) -> anyhow::Result<Vec<u8>>
    where
        H: Hasher,
        D: DataBackend,
        M: Vm<H, D, Self>;

    /// A notification that the program has been updated.
    fn updated(&self, program: &Hash);
}
