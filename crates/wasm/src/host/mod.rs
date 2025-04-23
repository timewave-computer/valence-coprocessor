use std::sync::{Arc, Mutex};

use lru::LruCache;
use serde_json::Value;
use valence_coprocessor::{DataBackend, ExecutionContext, Hash, Hasher, ModuleVM, ZkVM};
use wasmtime::{Engine, Linker, Module, Store};

use crate::HOST_MODULE;

pub mod valence;

pub struct Runtime {
    pub args: Value,
    pub ret: Option<Value>,
    pub storage: Option<Vec<u8>>,
    pub panic: Option<String>,
}

impl From<Value> for Runtime {
    fn from(args: Value) -> Self {
        Self {
            args,
            ret: None,
            storage: None,
            panic: None,
        }
    }
}

#[derive(Debug)]
pub struct ValenceWasm {
    engine: Engine,
    linker: Linker<Runtime>,
    modules: Arc<Mutex<LruCache<Hash, Module>>>,
}

impl ValenceWasm {
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

impl ModuleVM for ValenceWasm {
    fn execute<H, D, Z>(
        &self,
        ctx: &ExecutionContext<H, D, Self, Z>,
        module: &Hash,
        f: &str,
        args: Value,
    ) -> anyhow::Result<Value>
    where
        H: Hasher,
        D: DataBackend,
        Z: ZkVM,
    {
        let storage = ctx.get_program_storage()?;
        let runtime = Runtime {
            args,
            ret: None,
            storage,
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

        let Runtime { storage, ret, .. } = store.into_data();

        if let Some(storage) = storage {
            ctx.set_program_storage(&storage)?;
        }

        Ok(ret.unwrap_or_default())
    }
}
