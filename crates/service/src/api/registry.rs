use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, types::Base64, Object, OpenApi};
use serde_json::Value;
use valence_coprocessor::{DomainData, ProgramData};
use valence_coprocessor_rocksdb::RocksBackend;
use valence_coprocessor_sp1::Sp1ZkVM;

use crate::{Context, Registry, ValenceWasm};

use super::{try_str_to_hash, Api};

#[derive(Object, Debug)]
pub struct RegisterProgramRequest {
    /// A Base64 WASM encoded module.
    pub module: Base64<Vec<u8>>,

    /// A Base64 zkVM encoded prover.
    pub zkvm: Base64<Vec<u8>>,

    /// Optional nonce to affect hte program id.
    #[oai(default)]
    pub nonce: Option<u64>,
}

#[derive(Object, Debug)]
pub struct RegisterProgramResponse {
    /// The allocated program id as hex.
    pub program: String,
}

#[derive(Object, Debug)]
pub struct RegisterDomainRequest {
    /// Unique name identifier for the domain.
    pub name: String,

    /// Base64 code for the WASM module.
    pub module: Base64<Vec<u8>>,
}

#[derive(Object, Debug)]
pub struct RegisterDomainResponse {
    /// The allocated domain id as hex.
    pub domain: String,
}

#[derive(Object, Debug)]
pub struct ProgramLinkRequest {
    /// Domains to be registered.
    pub domains: Vec<String>,
}

#[derive(Object, Debug)]
pub struct ProgramLinkResponse;

#[derive(Object, Debug)]
pub struct ProgramUnlinkRequest {
    /// Domains to be de-registered.
    pub domains: Vec<String>,
}

#[derive(Object, Debug)]
pub struct ProgramUnlinkResponse;

#[derive(Object, Debug)]
pub struct ProgramDomainsResponse {
    /// Domains associated with the program
    pub domains: Vec<String>,
}

#[derive(Object, Debug)]
pub struct ProgramStorageResponse {
    /// Storage data associated with the program as base64.
    pub data: Base64<Vec<u8>>,
}

#[derive(Object, Debug)]
pub struct ProgramProveRequest {
    /// Arguments of the Valence program.
    pub args: Value,
}

#[derive(Object, Debug)]
pub struct ProgramProveResponse {
    /// The target ZK proof as base64.
    pub proof: Base64<Vec<u8>>,

    /// The output arguments as base64.
    pub outputs: Base64<Vec<u8>>,
}

#[derive(Object, Debug)]
pub struct ProgramVkResponse {
    /// The verifying key in base64.
    pub base64: Base64<Vec<u8>>,
}

#[derive(Object, Debug)]
pub struct ProgramEntrypointRequest {
    /// Arguments of the Valence program.
    pub args: Value,
}

#[OpenApi]
impl Api {
    /// Register a new program, returning its allocated id.
    #[oai(path = "/registry/program", method = "post")]
    pub async fn registry_program(
        &self,
        registry: Data<&Registry>,
        module: Data<&ValenceWasm>,
        zkvm: Data<&Sp1ZkVM>,
        request: Json<RegisterProgramRequest>,
    ) -> poem::Result<Json<RegisterProgramResponse>> {
        let program = ProgramData {
            module: request.module.to_vec(),
            zkvm: request.zkvm.to_vec(),
            nonce: request.nonce.unwrap_or(0),
        };

        let module: &ValenceWasm = &module;
        let zkvm: &Sp1ZkVM = &zkvm;
        let program = registry.register_program(module, zkvm, program)?;
        let program = RegisterProgramResponse {
            program: hex::encode(program),
        };

        Ok(Json(program))
    }

    /// Register a new domain, returning its allocated id.
    #[oai(path = "/registry/domain", method = "post")]
    pub async fn register_domain(
        &self,
        registry: Data<&Registry>,
        module: Data<&ValenceWasm>,
        request: Json<RegisterDomainRequest>,
    ) -> poem::Result<Json<RegisterDomainResponse>> {
        let domain = DomainData {
            name: request.name.clone(),
            module: request.module.to_vec(),
        };

        let module: &ValenceWasm = &module;
        let domain = registry.register_domain(module, domain)?;
        let domain = RegisterDomainResponse {
            domain: hex::encode(domain),
        };

        Ok(Json(domain))
    }

    /// Link the program to the provided domains.
    #[oai(path = "/registry/program/:program/link", method = "post")]
    pub async fn program_link(
        &self,
        registry: Data<&Registry>,
        program: Path<String>,
        request: Json<ProgramLinkRequest>,
    ) -> poem::Result<Json<ProgramLinkResponse>> {
        let program = try_str_to_hash(&program)?;
        let domains: Vec<_> = request
            .domains
            .iter()
            .map(|d| DomainData::identifier_from_parts(d))
            .collect();

        registry.program_link(&program, &domains)?;

        Ok(Json(ProgramLinkResponse))
    }

    /// Unlink the program to the provided domains.
    #[oai(path = "/registry/program/:program/unlink", method = "post")]
    pub async fn program_unlink(
        &self,
        registry: Data<&Registry>,
        program: Path<String>,
        request: Json<ProgramUnlinkRequest>,
    ) -> poem::Result<Json<ProgramUnlinkResponse>> {
        let program = try_str_to_hash(&program)?;
        let domains: Vec<_> = request
            .domains
            .iter()
            .map(|d| DomainData::identifier_from_parts(d))
            .collect();

        registry.program_unlink(&program, &domains)?;

        Ok(Json(ProgramUnlinkResponse))
    }

    /// Returns the list of hashed program domains.
    #[oai(path = "/registry/program/:program/domains", method = "get")]
    pub async fn program_domains(
        &self,
        registry: Data<&Registry>,
        program: Path<String>,
    ) -> poem::Result<Json<ProgramDomainsResponse>> {
        let program = try_str_to_hash(&program)?;

        let domains = registry.get_program_domains(&program)?;
        let domains = domains.iter().map(hex::encode).collect();

        Ok(Json(ProgramDomainsResponse { domains }))
    }

    /// Returns the storage data associated with the program
    #[oai(path = "/registry/program/:program/storage", method = "get")]
    pub async fn program_storage(
        &self,
        program: Path<String>,
        data: Data<&RocksBackend>,
        module: Data<&ValenceWasm>,
        zkvm: Data<&Sp1ZkVM>,
    ) -> poem::Result<Json<ProgramStorageResponse>> {
        let program = try_str_to_hash(&program)?;
        let ctx = Context::init(program, data.clone(), module.clone(), zkvm.clone());

        let data = ctx.get_program_storage()?.unwrap_or_default();
        let data = Base64(data);

        Ok(Json(ProgramStorageResponse { data }))
    }

    /// Computes the program proof.
    #[oai(path = "/registry/program/:program/prove", method = "post")]
    pub async fn program_prove(
        &self,
        program: Path<String>,
        data: Data<&RocksBackend>,
        module: Data<&ValenceWasm>,
        zkvm: Data<&Sp1ZkVM>,
        request: Json<ProgramProveRequest>,
    ) -> poem::Result<Json<ProgramProveResponse>> {
        let program = try_str_to_hash(&program)?;
        let ctx = Context::init(program, data.clone(), module.clone(), zkvm.clone());
        let proof = ctx.get_program_proof(request.args.clone())?;

        Ok(Json(ProgramProveResponse {
            proof: Base64(proof.proof),
            outputs: Base64(proof.outputs),
        }))
    }

    /// Returns the program verifying key.
    #[oai(path = "/registry/program/:program/vk", method = "get")]
    pub async fn program_vk(
        &self,
        program: Path<String>,
        data: Data<&RocksBackend>,
        module: Data<&ValenceWasm>,
        zkvm: Data<&Sp1ZkVM>,
    ) -> poem::Result<Json<ProgramVkResponse>> {
        let program = try_str_to_hash(&program)?;
        let ctx = Context::init(program, data.clone(), module.clone(), zkvm.clone());
        let vk = ctx.get_program_verifying_key()?;

        Ok(Json(ProgramVkResponse { base64: Base64(vk) }))
    }

    /// Computes the program proof.
    #[oai(path = "/registry/program/:program/entrypoint", method = "post")]
    pub async fn program_entrypoint(
        &self,
        program: Path<String>,
        data: Data<&RocksBackend>,
        module: Data<&ValenceWasm>,
        zkvm: Data<&Sp1ZkVM>,
        args: Json<Value>,
    ) -> poem::Result<Json<Value>> {
        let program = try_str_to_hash(&program)?;
        let ctx = Context::init(program, data.clone(), module.clone(), zkvm.clone());
        let ret = ctx.entrypoint(args.0)?;

        Ok(Json(ret))
    }
}
