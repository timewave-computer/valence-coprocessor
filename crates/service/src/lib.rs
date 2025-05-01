mod config;

pub use config::*;
pub mod api;
pub mod data;

use data::ServiceBackend;
use valence_coprocessor::ExecutionContext;
use valence_coprocessor_sp1::{Sp1Hasher, Sp1ZkVm};

pub type ValenceWasm =
    valence_coprocessor_wasm::host::ValenceWasm<Sp1Hasher, ServiceBackend, Sp1ZkVm>;
pub type Context = ExecutionContext<Sp1Hasher, ServiceBackend, ValenceWasm, Sp1ZkVm>;
pub type Registry = valence_coprocessor::Registry<ServiceBackend>;
