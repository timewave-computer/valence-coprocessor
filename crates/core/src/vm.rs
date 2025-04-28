use serde_json::Value;

use crate::{DataBackend, ExecutionContext, Hash, Hasher, ZkVM};

/// A library VM definition.
pub trait Vm<H, D, Z>: Sized
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    /// Execute a function in a library.
    ///
    /// Returns the output of the function call.
    ///
    /// ## Arguments
    ///
    /// - `ctx`: Execution context to fetch the library bytes from.
    /// - `lib`: Library unique identifier.
    /// - `f`: Function name to be called.
    /// - `args`: Arguments to be passed to the function call.
    fn execute(
        &self,
        ctx: &ExecutionContext<H, D, Self, Z>,
        lib: &Hash,
        f: &str,
        args: Value,
    ) -> anyhow::Result<Value>;

    /// A notification that the library has been updated.
    fn updated(&self, lib: &Hash);
}
