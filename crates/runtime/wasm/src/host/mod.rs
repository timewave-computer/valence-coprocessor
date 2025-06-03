use std::sync::{Arc, Mutex};

use lru::LruCache;
use serde_json::Value;
use valence_coprocessor::{DataBackend, ExecutionContext, Hash, Hasher, Vm};
use wasmtime::{Engine, Linker, Module, Store};

use crate::HOST_CONTROLLER;

pub mod valence;

pub struct Runtime<H, D, VM>
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    pub args: Value,
    pub ret: Option<Value>,
    pub ctx: ExecutionContext<H, D>,
    pub log: Vec<String>,
    pub panic: Option<String>,
    pub vm: VM,
}

impl<H, D, VM> Runtime<H, D, VM>
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    /// Creates a new runtime with the underlying context.
    pub fn new(ctx: ExecutionContext<H, D>, args: Value, vm: VM) -> Self {
        Self {
            args,
            ret: None,
            ctx,
            log: Vec::with_capacity(10),
            panic: None,
            vm,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValenceWasm<H, D>
where
    H: Hasher,
    D: DataBackend,
{
    engine: Engine,
    linker: Linker<Runtime<H, D, Self>>,
    modules: Arc<Mutex<LruCache<Hash, Module>>>,
}

impl<H, D> ValenceWasm<H, D>
where
    H: Hasher + 'static,
    D: DataBackend + 'static,
{
    /// Creates a new instance of the VM.
    pub fn new(capacity: usize) -> anyhow::Result<Self> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);

        linker.func_wrap(HOST_CONTROLLER, "panic", valence::panic)?;
        linker.func_wrap(HOST_CONTROLLER, "args", valence::args)?;
        linker.func_wrap(HOST_CONTROLLER, "ret", valence::ret)?;
        linker.func_wrap(HOST_CONTROLLER, "get_storage", valence::get_storage)?;
        linker.func_wrap(HOST_CONTROLLER, "set_storage", valence::set_storage)?;
        linker.func_wrap(
            HOST_CONTROLLER,
            "get_storage_file",
            valence::get_storage_file,
        )?;
        linker.func_wrap(
            HOST_CONTROLLER,
            "set_storage_file",
            valence::set_storage_file,
        )?;
        linker.func_wrap(HOST_CONTROLLER, "get_raw_storage", valence::get_raw_storage)?;
        linker.func_wrap(HOST_CONTROLLER, "set_raw_storage", valence::set_raw_storage)?;
        linker.func_wrap(HOST_CONTROLLER, "get_controller", valence::get_controller)?;
        linker.func_wrap(HOST_CONTROLLER, "get_historical", valence::get_historical)?;
        linker.func_wrap(
            HOST_CONTROLLER,
            "get_historical_opening",
            valence::get_historical_opening,
        )?;
        linker.func_wrap(
            HOST_CONTROLLER,
            "get_historical_payload",
            valence::get_historical_payload,
        )?;
        linker.func_wrap(
            HOST_CONTROLLER,
            "get_latest_block",
            valence::get_latest_block,
        )?;
        linker.func_wrap(HOST_CONTROLLER, "get_state_proof", valence::get_state_proof)?;
        linker.func_wrap(HOST_CONTROLLER, "http", valence::http)?;
        linker.func_wrap(HOST_CONTROLLER, "log", valence::log)?;

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

impl<H, D> Vm<H, D> for ValenceWasm<H, D>
where
    H: Hasher,
    D: DataBackend,
{
    fn execute(
        &self,
        ctx: &ExecutionContext<H, D>,
        controller: &Hash,
        f: &str,
        args: Value,
    ) -> anyhow::Result<Value> {
        tracing::debug!("executing controller {controller:x?}, {f}({:?})", args);

        let runtime = Runtime {
            args,
            ret: None,
            ctx: ctx.clone(),
            log: Vec::with_capacity(10),
            panic: None,
            vm: self.clone(),
        };

        let mut store = Store::new(&self.engine, runtime);

        let instance = self
            .modules
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking modules: {e}"))?
            .try_get_or_insert(*controller, || {
                ctx.get_controller(controller)?
                    .ok_or_else(|| anyhow::anyhow!("controller not found"))
                    .and_then(|b| Module::from_binary(&self.engine, &b))
            })
            .and_then(|i| self.linker.instantiate(&mut store, i))?;

        tracing::debug!("controller loaded...");

        instance
            .get_typed_func::<(), ()>(&mut store, f)?
            .call(&mut store, ())?;

        let Runtime { ret, log, .. } = store.into_data();

        tracing::debug!("function executed; ret `{ret:?}`...");

        ctx.extend_log(log)?;

        Ok(ret.unwrap_or_default())
    }

    fn updated(&self, controller: &Hash) {
        match self.modules.lock() {
            Ok(mut m) => {
                m.pop(controller);
            }
            Err(e) => tracing::error!("error locking modules: {e}"),
        }
    }
}
