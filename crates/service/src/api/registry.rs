use flume::Sender;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, types::Base64, Object, OpenApi};
use serde_json::{json, Value};
use valence_coprocessor::{DomainData, ProgramData};
use valence_coprocessor_sp1::Sp1ZkVm;

use crate::{worker::Job, Historical, Registry, ValenceWasm};

use super::{try_str_to_hash, Api};

#[derive(Object, Debug)]
pub struct RegisterProgramRequest {
    /// A Base64 WASM encoded library.
    pub lib: Base64<Vec<u8>>,

    /// A Base64 circuit encoded prover.
    pub circuit: Base64<Vec<u8>>,

    /// Optional nonce to affect the program id.
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

    /// Base64 code for the WASM domain library.
    pub lib: Base64<Vec<u8>>,
}

#[derive(Object, Debug)]
pub struct RegisterDomainResponse {
    /// The allocated domain id as hex.
    pub domain: String,
}

#[derive(Object, Debug)]
pub struct ProgramStorageFileRequest {
    /// Path of the program file.
    pub path: String,
}

#[derive(Object, Debug)]
pub struct ProgramStorageFileResponse {
    /// Base64 encoded contents of the file
    pub data: Base64<Vec<u8>>,
}

#[derive(Object, Debug)]
pub struct ProgramDomainsResponse {
    /// Domains associated with the program
    pub domains: Vec<String>,
}

#[derive(Object, Debug)]
pub struct ProgramRawStorageResponse {
    /// Raw storage associated with the program as base64.
    pub data: Base64<Vec<u8>>,

    /// Logs of the operation.
    pub log: Vec<String>,
}

#[derive(Object, Debug)]
pub struct ProgramProveRequest {
    /// Arguments of the Valence program.
    pub args: Value,

    /// Optional callback payload.
    pub payload: Option<Value>,
}

#[derive(Object, Debug)]
pub struct ProgramVkResponse {
    /// The verifying key in base64.
    pub base64: Base64<Vec<u8>>,

    /// Logs of the operation.
    pub log: Vec<String>,
}

#[derive(Object, Debug)]
pub struct ProgramEntrypointRequest {
    /// Arguments of the Valence program.
    pub args: Value,
}

#[derive(Object, Debug)]
pub struct ProgramEntrypointResponse {
    /// Return value of the entrypoint.
    pub ret: Value,

    /// Logs of the operation.
    pub log: Vec<String>,
}

#[OpenApi]
impl Api {
    /// Service stats.
    #[oai(path = "/stats", method = "get")]
    pub async fn stats(&self, pool: Data<&Sender<Job>>) -> poem::Result<Json<Value>> {
        Ok(Json(json!({
            "workers": pool.receiver_count().saturating_sub(1),
            "queued": pool.len(),
        })))
    }

    /// Register a new program, returning its allocated id.
    #[oai(path = "/registry/program", method = "post")]
    pub async fn registry_program(
        &self,
        registry: Data<&Registry>,
        vm: Data<&ValenceWasm>,
        zkvm: Data<&Sp1ZkVm>,
        request: Json<RegisterProgramRequest>,
    ) -> poem::Result<Json<RegisterProgramResponse>> {
        let program = ProgramData {
            lib: request.lib.to_vec(),
            circuit: request.circuit.to_vec(),
            nonce: request.nonce.unwrap_or(0),
        };

        let program = registry.register_program(*vm, *zkvm, program)?;
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
        vm: Data<&ValenceWasm>,
        request: Json<RegisterDomainRequest>,
    ) -> poem::Result<Json<RegisterDomainResponse>> {
        let domain = DomainData {
            name: request.name.clone(),
            lib: request.lib.to_vec(),
        };

        let domain = registry.register_domain(*vm, domain)?;
        let domain = RegisterDomainResponse {
            domain: hex::encode(domain),
        };

        Ok(Json(domain))
    }

    /// Returns the raw storage of the program.
    #[oai(path = "/registry/program/:program/storage/raw", method = "get")]
    pub async fn storage_raw(
        &self,
        program: Path<String>,
        historical: Data<&Historical>,
    ) -> poem::Result<Json<ProgramRawStorageResponse>> {
        let program = try_str_to_hash(&program)?;
        let ctx = historical.context(program);

        let data = ctx.get_raw_storage()?.unwrap_or_default();
        let data = Base64(data);
        let log = ctx.get_log()?;

        Ok(Json(ProgramRawStorageResponse { data, log }))
    }

    /// Returns a file from the storage of the program.
    #[oai(path = "/registry/program/:program/storage/fs", method = "post")]
    pub async fn storage_file(
        &self,
        program: Path<String>,
        historical: Data<&Historical>,
        request: Json<ProgramStorageFileRequest>,
    ) -> poem::Result<Json<ProgramStorageFileResponse>> {
        let path = request.0.path;

        tracing::debug!("received file request for path `{path}`...");

        let program = try_str_to_hash(&program)?;
        let ctx = historical.context(program);

        let data = ctx.get_storage_file(&path)?;
        let data = Base64(data);

        Ok(Json(ProgramStorageFileResponse { data }))
    }

    /// Computes the program proof.
    #[oai(path = "/registry/program/:program/prove", method = "post")]
    pub async fn program_prove(
        &self,
        program: Path<String>,
        pool: Data<&Sender<Job>>,
        request: Json<ProgramProveRequest>,
    ) -> poem::Result<Json<Value>> {
        let program = try_str_to_hash(&program)?;
        let ProgramProveRequest { args, payload } = request.0;

        pool.send(Job::Prove {
            program,
            args,
            payload,
        })
        .map_err(|e| anyhow::anyhow!("failed to submit prove job: {e}"))?;

        Ok(Json(json!({"status": "received"})))
    }

    /// Returns the program verifying key.
    #[oai(path = "/registry/program/:program/vk", method = "get")]
    pub async fn program_vk(
        &self,
        program: Path<String>,
        historical: Data<&Historical>,
    ) -> poem::Result<Json<ProgramVkResponse>> {
        let program = try_str_to_hash(&program)?;
        let ctx = historical.context(program);

        let vk = ctx.get_program_verifying_key()?;
        let log = ctx.get_log()?;

        Ok(Json(ProgramVkResponse {
            base64: Base64(vk),
            log,
        }))
    }

    /// Computes the program proof.
    #[oai(path = "/registry/program/:program/entrypoint", method = "post")]
    pub async fn program_entrypoint(
        &self,
        program: Path<String>,
        historical: Data<&Historical>,
        args: Json<Value>,
    ) -> poem::Result<Json<ProgramEntrypointResponse>> {
        let program = try_str_to_hash(&program)?;
        let ctx = historical.context(program);

        let ret = ctx.entrypoint(args.0)?;
        let log = ctx.get_log()?;

        Ok(Json(ProgramEntrypointResponse { ret, log }))
    }

    /// Get the latest proven block for the domain.
    #[oai(path = "/registry/domain/:domain/latest", method = "get")]
    pub async fn domain_latest(
        &self,
        domain: Path<String>,
        historical: Data<&Historical>,
    ) -> poem::Result<Json<Value>> {
        let id = DomainData::identifier_from_parts(&domain);
        let ctx = historical.context(id);

        let latest = ctx.get_latest_block(&domain)?;
        let latest = serde_json::to_value(latest)
            .map_err(|e| anyhow::anyhow!("failed to convert latest block: {e}"))?;

        Ok(Json(latest))
    }

    /// Adds a new block to the domain.
    #[oai(path = "/registry/domain/:domain", method = "post")]
    pub async fn domain_add_block(
        &self,
        domain: Path<String>,
        historical: Data<&Historical>,
        args: Json<Value>,
    ) -> poem::Result<Json<Value>> {
        let log = historical.add_domain_block(&domain, args.0)?;

        Ok(Json(serde_json::json!({
            "log": log
        })))
    }
}
