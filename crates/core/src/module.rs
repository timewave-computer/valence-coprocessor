use serde_json::Value;

use crate::{DataBackend, ExecutionContext, Hash, Hasher, ZkVM};

/// A module VM definition.
pub trait ModuleVM<H, D, Z>: Sized
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
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
    fn execute(
        &self,
        ctx: &ExecutionContext<H, D, Self, Z>,
        module: &Hash,
        f: &str,
        args: Value,
    ) -> anyhow::Result<Value>;
}
