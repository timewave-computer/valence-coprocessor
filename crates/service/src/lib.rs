mod config;

pub use config::*;
pub mod api;

use valence_coprocessor::ExecutionContext;
use valence_coprocessor_rocksdb::RocksBackend;
use valence_coprocessor_sp1::{Sp1Hasher, Sp1ZkVm};

pub type ValenceWasm =
    valence_coprocessor_wasm::host::ValenceWasm<Sp1Hasher, RocksBackend, Sp1ZkVm>;
pub type Context = ExecutionContext<Sp1Hasher, RocksBackend, ValenceWasm, Sp1ZkVm>;
pub type Registry = valence_coprocessor::Registry<RocksBackend>;
