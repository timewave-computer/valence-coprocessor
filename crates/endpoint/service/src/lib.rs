pub mod api;
pub mod data;
pub mod middleware;
pub mod worker;

use data::ServiceBackend;
use valence_coprocessor::ExecutionContext;
use valence_coprocessor_sp1::Sp1Hasher;

pub type ServiceVm = valence_coprocessor_wasm::host::ValenceWasm<Sp1Hasher, ServiceBackend>;
pub type Context = ExecutionContext<Sp1Hasher, ServiceBackend>;
pub type Registry = valence_coprocessor::Registry<ServiceBackend>;
pub type Historical = valence_coprocessor::Historical<Sp1Hasher, ServiceBackend>;
