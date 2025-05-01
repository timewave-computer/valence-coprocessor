use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, types::Base64, Object, OpenApi};
use serde_json::Value;
use valence_coprocessor::{DomainData, ProgramData};
use valence_coprocessor_sp1::Sp1ZkVm;

use crate::{data::ServiceBackend, Context, Registry, ValenceWasm};

use super::{try_str_to_hash, Api};

#[derive(Object, Debug)]
pub struct RegisterProgramRequest {
    /// A Base64 WASM encoded library.
    pub lib: Base64<Vec<u8>>,

    /// A Base64 circuit encoded prover.
    pub circuit: Base64<Vec<u8>>,

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

    /// Base64 code for the WASM domain library.
    pub lib: Base64<Vec<u8>>,
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

    /// Logs of the operation.
    pub log: Vec<String>,
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

    /// Logs of the operation.
    pub log: Vec<String>,
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

    /// Returns the storage data associated with the program.
    #[oai(path = "/registry/program/:program/storage", method = "get")]
    pub async fn storage(
        &self,
        program: Path<String>,
        data: Data<&ServiceBackend>,
        vm: Data<&ValenceWasm>,
        zkvm: Data<&Sp1ZkVm>,
    ) -> poem::Result<Json<ProgramStorageResponse>> {
        let program = try_str_to_hash(&program)?;
        let ctx = Context::init(program, data.clone(), vm.clone(), zkvm.clone());

        let data = ctx.get_storage()?.unwrap_or_default();
        let data = Base64(data);
        let log = ctx.get_log()?;

        Ok(Json(ProgramStorageResponse { data, log }))
    }

    /// Computes the program proof.
    #[oai(path = "/registry/program/:program/prove", method = "post")]
    pub async fn program_prove(
        &self,
        program: Path<String>,
        data: Data<&ServiceBackend>,
        vm: Data<&ValenceWasm>,
        zkvm: Data<&Sp1ZkVm>,
        request: Json<ProgramProveRequest>,
    ) -> poem::Result<Json<ProgramProveResponse>> {
        let program = try_str_to_hash(&program)?;
        let ctx = Context::init(program, data.clone(), vm.clone(), zkvm.clone());

        let proof = match ctx.get_program_proof(request.args.clone()) {
            Ok(p) => p,
            Err(e) => {
                return Ok(Json(ProgramProveResponse {
                    proof: Base64(vec![]),
                    outputs: Base64(vec![]),
                    log: vec![format!("Error computing the proof: {e}")],
                }));
            }
        };

        let log = ctx.get_log()?;

        Ok(Json(ProgramProveResponse {
            proof: Base64(proof.proof),
            outputs: Base64(proof.outputs),
            log,
        }))
    }

    /// Returns the program verifying key.
    #[oai(path = "/registry/program/:program/vk", method = "get")]
    pub async fn program_vk(
        &self,
        program: Path<String>,
        data: Data<&ServiceBackend>,
        vm: Data<&ValenceWasm>,
        zkvm: Data<&Sp1ZkVm>,
    ) -> poem::Result<Json<ProgramVkResponse>> {
        let program = try_str_to_hash(&program)?;
        let ctx = Context::init(program, data.clone(), vm.clone(), zkvm.clone());
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
        data: Data<&ServiceBackend>,
        vm: Data<&ValenceWasm>,
        zkvm: Data<&Sp1ZkVm>,
        args: Json<Value>,
    ) -> poem::Result<Json<ProgramEntrypointResponse>> {
        let program = try_str_to_hash(&program)?;
        let ctx = Context::init(program, data.clone(), vm.clone(), zkvm.clone());
        let ret = ctx.entrypoint(args.0)?;
        let log = ctx.get_log()?;

        Ok(Json(ProgramEntrypointResponse { ret, log }))
    }

    /// Get the latest proven block for the domain.
    #[oai(path = "/registry/domain/:domain/latest", method = "get")]
    pub async fn domain_latest(
        &self,
        domain: Path<String>,
        data: Data<&ServiceBackend>,
        vm: Data<&ValenceWasm>,
        zkvm: Data<&Sp1ZkVm>,
    ) -> poem::Result<Json<Value>> {
        let id = DomainData::identifier_from_parts(&domain);
        let ctx = Context::init(id, data.clone(), vm.clone(), zkvm.clone());

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
        data: Data<&ServiceBackend>,
        vm: Data<&ValenceWasm>,
        zkvm: Data<&Sp1ZkVm>,
        args: Json<Value>,
    ) -> poem::Result<Json<Value>> {
        let id = DomainData::identifier_from_parts(&domain);
        let ctx = Context::init(id, data.clone(), vm.clone(), zkvm.clone());

        ctx.add_domain_block(&domain, args.0)?;

        let log = ctx.get_log()?;

        Ok(Json(serde_json::json!({
            "log": log
        })))
    }
}
