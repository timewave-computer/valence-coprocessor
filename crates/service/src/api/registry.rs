use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, types::Base64, Object, OpenApi};
use serde_json::Value;
use valence_coprocessor::{DomainData, ProgramData};
use valence_coprocessor_rocksdb::RocksBackend;
use valence_coprocessor_sp1::Sp1ZkVM;

use crate::{Context, Registry, ValenceWasm};

use super::{try_slice_to_hash, Api};

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
    /// The allocated program id as base64.
    pub program: Base64<Vec<u8>>,
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
    /// The allocated domain id as base64.
    pub domain: Base64<Vec<u8>>,
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
    pub domains: Vec<Base64<Vec<u8>>>,
}

#[derive(Object, Debug)]
pub struct ProgramStorageResponse {
    /// Storage data associated with the program.
    pub data: Base64<Vec<u8>>,
}

#[derive(Object, Debug)]
pub struct ProgramProveRequest {
    /// Arguments of the Valence program.
    pub args: Value,
}

#[derive(Object, Debug)]
pub struct ProgramProveResponse {
    /// The target ZK proof.
    pub proof: Base64<Vec<u8>>,

    /// The output arguments.
    pub outputs: Base64<Vec<u8>>,
}

#[OpenApi]
impl Api {
    /// Register a new program, returning its allocated id.
    #[oai(path = "/registry/program", method = "post")]
    pub async fn registry_program(
        &self,
        registry: Data<&Registry>,
        request: Json<RegisterProgramRequest>,
    ) -> poem::Result<Json<RegisterProgramResponse>> {
        let program = ProgramData {
            module: request.module.to_vec(),
            zkvm: request.zkvm.to_vec(),
            nonce: request.nonce.unwrap_or(0),
        };

        let program = registry.register_program(program)?;
        let program = RegisterProgramResponse {
            program: Base64(program.to_vec()),
        };

        Ok(Json(program))
    }

    /// Register a new domain, returning its allocated id.
    #[oai(path = "/registry/domain", method = "post")]
    pub async fn register_domain(
        &self,
        registry: Data<&Registry>,
        request: Json<RegisterDomainRequest>,
    ) -> poem::Result<Json<RegisterDomainResponse>> {
        let domain = DomainData {
            name: request.name.clone(),
            module: request.module.to_vec(),
        };

        let domain = registry.register_domain(domain)?;
        let domain = RegisterDomainResponse {
            domain: Base64(domain.to_vec()),
        };

        Ok(Json(domain))
    }

    /// Link the program to the provided domains.
    #[oai(path = "/registry/program/:program/link", method = "post")]
    pub async fn program_link(
        &self,
        registry: Data<&Registry>,
        program: Path<Base64<Vec<u8>>>,
        request: Json<ProgramLinkRequest>,
    ) -> poem::Result<Json<ProgramLinkResponse>> {
        let program = try_slice_to_hash(&program.0)?;
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
        program: Path<Base64<Vec<u8>>>,
        request: Json<ProgramUnlinkRequest>,
    ) -> poem::Result<Json<ProgramUnlinkResponse>> {
        let program = try_slice_to_hash(&program.0)?;
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
        program: Path<Base64<Vec<u8>>>,
    ) -> poem::Result<Json<ProgramDomainsResponse>> {
        let program = try_slice_to_hash(&program.0)?;

        let domains = registry.get_program_domains(&program)?;
        let domains = domains.iter().map(|d| Base64(d.to_vec())).collect();

        Ok(Json(ProgramDomainsResponse { domains }))
    }

    /// Returns the storage data associated with the program
    #[oai(path = "/registry/program/:program/storage", method = "get")]
    pub async fn program_storage(
        &self,
        program: Path<Base64<Vec<u8>>>,
        data: Data<&RocksBackend>,
        module: Data<&ValenceWasm>,
        zkvm: Data<&Sp1ZkVM>,
    ) -> poem::Result<Json<ProgramStorageResponse>> {
        let program = try_slice_to_hash(&program.0)?;
        let ctx = Context::init(program, data.clone(), module.clone(), zkvm.clone());

        let data = ctx.get_program_storage()?.unwrap_or_default();
        let data = Base64(data);

        Ok(Json(ProgramStorageResponse { data }))
    }

    /// Computes the program proof.
    #[oai(path = "/registry/program/:program/prove", method = "post")]
    pub async fn program_prove(
        &self,
        program: Path<Base64<Vec<u8>>>,
        data: Data<&RocksBackend>,
        module: Data<&ValenceWasm>,
        zkvm: Data<&Sp1ZkVM>,
        request: Json<ProgramProveRequest>,
    ) -> poem::Result<Json<ProgramProveResponse>> {
        let program = try_slice_to_hash(&program.0)?;
        let ctx = Context::init(program, data.clone(), module.clone(), zkvm.clone());
        let proof = ctx.get_program_proof(request.args.clone())?;

        Ok(Json(ProgramProveResponse {
            proof: Base64(proof.proof),
            outputs: Base64(proof.outputs),
        }))
    }
}
