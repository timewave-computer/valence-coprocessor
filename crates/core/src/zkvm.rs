use alloc::vec::Vec;

use crate::{DataBackend, ExecutionContext, Hasher, ModuleVM, ProvenProgram, Witness};

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
        witnesses: Vec<Witness>,
    ) -> anyhow::Result<ProvenProgram>
    where
        H: Hasher,
        D: DataBackend,
        M: ModuleVM<H, D, Self>;
}
