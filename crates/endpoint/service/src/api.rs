use flume::Sender;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, types::Base64, Object, OpenApi};
use serde_json::{json, Value};
use valence_coprocessor::Hash;
use valence_coprocessor::{ControllerData, DomainData};

use crate::{worker::Job, Historical, Registry, ServiceVm, ServiceZkVm};

pub struct Api;

#[derive(Object, Debug)]
pub struct RegisterControllerRequest {
    /// A Base64 WASM encoded controller.
    pub controller: Base64<Vec<u8>>,

    /// A Base64 circuit encoded prover.
    pub circuit: Base64<Vec<u8>>,

    /// Optional nonce to affect the controller id.
    #[oai(default)]
    pub nonce: Option<u64>,
}

#[derive(Object, Debug)]
pub struct RegisterControllerResponse {
    /// The allocated controller id as hex.
    pub controller: String,
}

#[derive(Object, Debug)]
pub struct RegisterDomainRequest {
    /// Unique name identifier for the domain.
    pub name: String,

    /// Base64 code for the WASM domain controller.
    pub controller: Base64<Vec<u8>>,
}

#[derive(Object, Debug)]
pub struct RegisterDomainResponse {
    /// The allocated domain id as hex.
    pub domain: String,
}

#[derive(Object, Debug)]
pub struct ControllerStorageFileRequest {
    /// Path of the controller file.
    pub path: String,
}

#[derive(Object, Debug)]
pub struct ControllerStorageFileResponse {
    /// Base64 encoded contents of the file
    pub data: Base64<Vec<u8>>,
}

#[derive(Object, Debug)]
pub struct ControllerDomainsResponse {
    /// Domains associated with the controller
    pub domains: Vec<String>,
}

#[derive(Object, Debug)]
pub struct ControllerRawStorageResponse {
    /// Raw storage associated with the controller as base64.
    pub data: Base64<Vec<u8>>,

    /// Logs of the operation.
    pub log: Vec<String>,
}

#[derive(Object, Debug)]
pub struct ControllerProveRequest {
    /// Arguments of the Valence controller.
    pub args: Value,

    /// Optional callback payload.
    pub payload: Option<Value>,
}

#[derive(Object, Debug)]
pub struct ControllerVkResponse {
    /// The verifying key in base64.
    pub base64: Base64<Vec<u8>>,

    /// Logs of the operation.
    pub log: Vec<String>,
}

#[derive(Object, Debug)]
pub struct ControllerEntrypointRequest {
    /// Arguments of the Valence controller.
    pub args: Value,
}

#[derive(Object, Debug)]
pub struct ControllerEntrypointResponse {
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

    /// Register a new controller, returning its allocated id.
    #[oai(path = "/registry/controller", method = "post")]
    pub async fn registry_controller(
        &self,
        registry: Data<&Registry>,
        vm: Data<&ServiceVm>,
        zkvm: Data<&ServiceZkVm>,
        request: Json<RegisterControllerRequest>,
    ) -> poem::Result<Json<RegisterControllerResponse>> {
        let controller = ControllerData {
            controller: request.controller.to_vec(),
            circuit: request.circuit.to_vec(),
            nonce: request.nonce.unwrap_or(0),
        };

        let controller = registry.register_controller(*vm, *zkvm, controller)?;
        let controller = RegisterControllerResponse {
            controller: hex::encode(controller),
        };

        Ok(Json(controller))
    }

    /// Register a new domain, returning its allocated id.
    #[oai(path = "/registry/domain", method = "post")]
    pub async fn register_domain(
        &self,
        registry: Data<&Registry>,
        vm: Data<&ServiceVm>,
        request: Json<RegisterDomainRequest>,
    ) -> poem::Result<Json<RegisterDomainResponse>> {
        let domain = DomainData {
            name: request.name.clone(),
            controller: request.controller.to_vec(),
        };

        let domain = registry.register_domain(*vm, domain)?;
        let domain = RegisterDomainResponse {
            domain: hex::encode(domain),
        };

        Ok(Json(domain))
    }

    /// Returns the raw storage of the controller.
    #[oai(path = "/registry/controller/:controller/storage/raw", method = "get")]
    pub async fn storage_raw(
        &self,
        controller: Path<String>,
        historical: Data<&Historical>,
    ) -> poem::Result<Json<ControllerRawStorageResponse>> {
        let controller = try_str_to_hash(&controller)?;
        let ctx = historical.context(controller);

        let data = ctx.get_raw_storage()?.unwrap_or_default();
        let data = Base64(data);
        let log = ctx.get_log()?;

        Ok(Json(ControllerRawStorageResponse { data, log }))
    }

    /// Returns a file from the storage of the controller.
    #[oai(path = "/registry/controller/:controller/storage/fs", method = "post")]
    pub async fn storage_file(
        &self,
        controller: Path<String>,
        historical: Data<&Historical>,
        request: Json<ControllerStorageFileRequest>,
    ) -> poem::Result<Json<ControllerStorageFileResponse>> {
        let path = request.0.path;

        tracing::debug!("received file request for path `{path}`...");

        let controller = try_str_to_hash(&controller)?;
        let ctx = historical.context(controller);

        let data = ctx.get_storage_file(&path)?;
        let data = Base64(data);

        Ok(Json(ControllerStorageFileResponse { data }))
    }

    /// Computes the controller proof.
    #[oai(path = "/registry/controller/:controller/prove", method = "post")]
    pub async fn controller_prove(
        &self,
        controller: Path<String>,
        pool: Data<&Sender<Job>>,
        request: Json<ControllerProveRequest>,
    ) -> poem::Result<Json<Value>> {
        let controller = try_str_to_hash(&controller)?;
        let ControllerProveRequest { args, payload } = request.0;

        pool.send(Job::Prove {
            controller,
            args,
            payload,
        })
        .map_err(|e| anyhow::anyhow!("failed to submit prove job: {e}"))?;

        Ok(Json(json!({"status": "received"})))
    }

    /// Returns the controller verifying key.
    #[oai(path = "/registry/controller/:controller/vk", method = "get")]
    pub async fn controller_vk(
        &self,
        controller: Path<String>,
        historical: Data<&Historical>,
        zkvm: Data<&ServiceZkVm>,
    ) -> poem::Result<Json<ControllerVkResponse>> {
        let controller = try_str_to_hash(&controller)?;
        let ctx = historical.context(controller);

        let vk = ctx.get_verifying_key(*zkvm)?;
        let log = ctx.get_log()?;

        Ok(Json(ControllerVkResponse {
            base64: Base64(vk),
            log,
        }))
    }

    /// Computes the controller proof.
    #[oai(path = "/registry/controller/:controller/entrypoint", method = "post")]
    pub async fn controller_entrypoint(
        &self,
        controller: Path<String>,
        historical: Data<&Historical>,
        vm: Data<&ServiceVm>,
        args: Json<Value>,
    ) -> poem::Result<Json<ControllerEntrypointResponse>> {
        let controller = try_str_to_hash(&controller)?;
        let ctx = historical.context(controller);

        let ret = ctx.entrypoint(*vm, args.0)?;
        let log = ctx.get_log()?;

        Ok(Json(ControllerEntrypointResponse { ret, log }))
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
        vm: Data<&ServiceVm>,
        args: Json<Value>,
    ) -> poem::Result<Json<Value>> {
        let log = historical.add_domain_block(*vm, &domain, args.0)?;

        Ok(Json(serde_json::json!({
            "log": log
        })))
    }
}

fn try_str_to_hash(hash: &str) -> anyhow::Result<Hash> {
    let bytes =
        hex::decode(hash).map_err(|e| anyhow::anyhow!("error converting str to hash: {e}"))?;

    Hash::try_from(bytes).map_err(|_| anyhow::anyhow!("error converting bytes to hash"))
}
