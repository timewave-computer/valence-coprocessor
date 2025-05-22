use serde_json::Value;

use crate::{DataBackend, ExecutionContext, Hash, Hasher};

/// A VM definition to execute controller's code.
pub trait Vm<H, D>: Clone + Sized
where
    H: Hasher,
    D: DataBackend,
{
    /// Execute a controller function.
    ///
    /// Returns the output of the function call.
    ///
    /// ## Arguments
    ///
    /// - `ctx`: Execution context to fetch the controller bytes from.
    /// - `controller`: Controller unique identifier.
    /// - `f`: Function name to be called.
    /// - `args`: Arguments to be passed to the function call.
    fn execute(
        &self,
        ctx: &ExecutionContext<H, D>,
        controller: &Hash,
        f: &str,
        args: Value,
    ) -> anyhow::Result<Value>;

    /// A notification that the controller has been updated.
    fn updated(&self, controller: &Hash);
}
