use std::sync::{Arc, Mutex};

use lru::LruCache;
use serde_json::Value;
use valence_coprocessor::{DataBackend, ExecutionContext, Hash, Hasher, ModuleVM, ZkVM};
use wasmtime::{Engine, Linker, Module, Store};

use crate::HOST_MODULE;

pub mod valence;

pub struct Runtime<H, D, Z>
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    pub args: Value,
    pub ret: Option<Value>,
    pub ctx: ExecutionContext<H, D, ValenceWasm<H, D, Z>, Z>,
    pub panic: Option<String>,
}

impl<H, D, Z> Runtime<H, D, Z>
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    /// Creates a new runtime with the underlying context.
    pub fn new(ctx: ExecutionContext<H, D, ValenceWasm<H, D, Z>, Z>, args: Value) -> Self {
        Self {
            args,
            ret: None,
            ctx,
            panic: None,
        }
    }
}

#[derive(Debug)]
pub struct ValenceWasm<H, D, Z>
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    engine: Engine,
    linker: Linker<Runtime<H, D, Z>>,
    modules: Arc<Mutex<LruCache<Hash, Module>>>,
}

impl<H, D, Z> ValenceWasm<H, D, Z>
where
    H: Hasher + 'static,
    D: DataBackend + 'static,
    Z: ZkVM + 'static,
{
    /// Creates a new instance of the VM.
    pub fn new(capacity: usize) -> anyhow::Result<Self> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);

        linker.func_wrap(HOST_MODULE, "panic", valence::panic)?;
        linker.func_wrap(HOST_MODULE, "args", valence::args)?;
        linker.func_wrap(HOST_MODULE, "ret", valence::ret)?;
        linker.func_wrap(
            HOST_MODULE,
            "get_program_storage",
            valence::get_program_storage,
        )?;
        linker.func_wrap(
            HOST_MODULE,
            "set_program_storage",
            valence::set_program_storage,
        )?;
        linker.func_wrap(HOST_MODULE, "get_program", valence::get_program)?;
        linker.func_wrap(HOST_MODULE, "get_domain_proof", valence::get_domain_proof)?;
        linker.func_wrap(HOST_MODULE, "get_state_proof", valence::get_state_proof)?;
        linker.func_wrap(HOST_MODULE, "http", valence::http)?;

        let capacity = std::num::NonZeroUsize::new(capacity)
            .ok_or_else(|| anyhow::anyhow!("invalid capacity"))?;
        let modules = LruCache::new(capacity);
        let modules = Arc::new(Mutex::new(modules));

        Ok(Self {
            engine,
            linker,
            modules,
        })
    }
}

impl<H, D, Z> ModuleVM<H, D, Z> for ValenceWasm<H, D, Z>
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    fn execute(
        &self,
        ctx: &ExecutionContext<H, D, Self, Z>,
        module: &Hash,
        f: &str,
        args: Value,
    ) -> anyhow::Result<Value> {
        let runtime = Runtime {
            args,
            ret: None,
            ctx: ctx.clone(),
            panic: None,
        };

        let mut store = Store::new(&self.engine, runtime);

        let instance = self
            .modules
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking modules: {e}"))?
            .try_get_or_insert(*module, || {
                ctx.get_module(module)?
                    .ok_or_else(|| anyhow::anyhow!("module not found"))
                    .and_then(|b| Module::from_binary(&self.engine, &b))
            })
            .and_then(|i| self.linker.instantiate(&mut store, i))?;

        instance
            .get_typed_func::<(), ()>(&mut store, f)?
            .call(&mut store, ())?;

        let Runtime { ret, .. } = store.into_data();

        Ok(ret.unwrap_or_default())
    }
}
