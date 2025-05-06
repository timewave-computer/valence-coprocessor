use std::sync::{Arc, Mutex};

use lru::LruCache;
use serde_json::Value;
use valence_coprocessor::{DataBackend, ExecutionContext, Hash, Hasher, Vm, ZkVm};
use wasmtime::{Engine, Linker, Module, Store};

use crate::HOST_LIB;

pub mod valence;

pub struct Runtime<H, D, Z>
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVm,
{
    pub args: Value,
    pub ret: Option<Value>,
    pub ctx: ExecutionContext<H, D, ValenceWasm<H, D, Z>, Z>,
    pub log: Vec<String>,
    pub panic: Option<String>,
}

impl<H, D, Z> Runtime<H, D, Z>
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVm,
{
    /// Creates a new runtime with the underlying context.
    pub fn new(ctx: ExecutionContext<H, D, ValenceWasm<H, D, Z>, Z>, args: Value) -> Self {
        Self {
            args,
            ret: None,
            ctx,
            log: Vec::with_capacity(10),
            panic: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValenceWasm<H, D, Z>
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVm,
{
    engine: Engine,
    linker: Linker<Runtime<H, D, Z>>,
    libs: Arc<Mutex<LruCache<Hash, Module>>>,
}

impl<H, D, Z> ValenceWasm<H, D, Z>
where
    H: Hasher + 'static,
    D: DataBackend + 'static,
    Z: ZkVm + 'static,
{
    /// Creates a new instance of the VM.
    pub fn new(capacity: usize) -> anyhow::Result<Self> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);

        linker.func_wrap(HOST_LIB, "panic", valence::panic)?;
        linker.func_wrap(HOST_LIB, "args", valence::args)?;
        linker.func_wrap(HOST_LIB, "ret", valence::ret)?;
        linker.func_wrap(HOST_LIB, "get_raw_storage", valence::get_raw_storage)?;
        linker.func_wrap(HOST_LIB, "set_raw_storage", valence::set_raw_storage)?;
        linker.func_wrap(HOST_LIB, "get_library", valence::get_library)?;
        linker.func_wrap(HOST_LIB, "get_domain_proof", valence::get_domain_proof)?;
        linker.func_wrap(HOST_LIB, "get_latest_block", valence::get_latest_block)?;
        linker.func_wrap(HOST_LIB, "get_state_proof", valence::get_state_proof)?;
        linker.func_wrap(HOST_LIB, "http", valence::http)?;
        linker.func_wrap(HOST_LIB, "log", valence::log)?;

        let capacity = std::num::NonZeroUsize::new(capacity)
            .ok_or_else(|| anyhow::anyhow!("invalid capacity"))?;
        let libs = LruCache::new(capacity);
        let libs = Arc::new(Mutex::new(libs));

        Ok(Self {
            engine,
            linker,
            libs,
        })
    }
}

impl<H, D, Z> Vm<H, D, Z> for ValenceWasm<H, D, Z>
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVm,
{
    fn execute(
        &self,
        ctx: &ExecutionContext<H, D, Self, Z>,
        lib: &Hash,
        f: &str,
        args: Value,
    ) -> anyhow::Result<Value> {
        tracing::debug!(
            "executing library {lib:x?}, {f}({})",
            serde_json::to_string(&args)?
        );

        let runtime = Runtime {
            args,
            ret: None,
            ctx: ctx.clone(),
            log: Vec::with_capacity(10),
            panic: None,
        };

        let mut store = Store::new(&self.engine, runtime);

        let instance = self
            .libs
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking libs: {e}"))?
            .try_get_or_insert(*lib, || {
                ctx.get_lib(lib)?
                    .ok_or_else(|| anyhow::anyhow!("lib not found"))
                    .and_then(|b| Module::from_binary(&self.engine, &b))
            })
            .and_then(|i| self.linker.instantiate(&mut store, i))?;

        tracing::debug!("library loaded...");

        instance
            .get_typed_func::<(), ()>(&mut store, f)?
            .call(&mut store, ())?;

        tracing::debug!("function called...");

        let Runtime { ret, log, .. } = store.into_data();

        ctx.extend_log(log)?;

        Ok(ret.unwrap_or_default())
    }

    fn updated(&self, lib: &Hash) {
        match self.libs.lock() {
            Ok(mut m) => {
                m.pop(lib);
            }
            Err(e) => tracing::error!("error locking libs: {e}"),
        }
    }
}
