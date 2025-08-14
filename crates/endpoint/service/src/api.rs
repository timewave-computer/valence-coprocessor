use std::path::PathBuf;

use flume::Sender;
use poem::{http::StatusCode, web::Data, Error as PoemError};
use poem_openapi::{param::Path, payload::Json, types::Base64, Object, OpenApi};
use serde_json::{json, Value};
use valence_coprocessor::{BlockAdded, Hash, HistoricalUpdate, ValidatedDomainBlock};
use valence_coprocessor::{ControllerData, DomainData};

use crate::Context;
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

    /// A Base64 circuit encoded prover.
    pub circuit: Base64<Vec<u8>>,
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
pub struct ControllerStorageFileStoreRequest {
    /// Path of the controller file.
    ///
    /// Note: This is a FAT-16 filesystem, so extensions must have max 3 characters.
    pub path: String,

    /// Base64 contents of the file.
    pub contents: String,
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
pub struct ControllerWitnessesResponse {
    /// The vector of computed witnesses.
    pub witnesses: Value,

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
pub struct ControllerCircuitResponse {
    /// The circuit bytecode in base64.
    pub base64: Base64<Vec<u8>>,
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

#[derive(Object, Debug)]
pub struct DomainAddBlockResponse {
    /// Domain to which the block was added.
    pub domain: String,
    /// Historical SMT root prior to the mutation.
    pub prev_smt: Hash,
    /// Historical SMT root after the mutation.
    pub smt: Hash,
    /// Controller execution log.
    pub log: Vec<String>,
    /// A block associated number.
    pub number: u64,
    /// The hash root of the block.
    pub root: Hash,
    /// Block blob payload.
    pub payload: Vec<u8>,
}

#[OpenApi]
impl Api {
    /// Service stats.
    #[oai(path = "/stats", method = "get")]
    pub async fn stats(&self) -> poem::Result<Json<Value>> {
        const VERSION: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

        Ok(Json(json!({
            "version": VERSION,
        })))
    }

    /// Register a new controller, returning its allocated id.
    #[oai(path = "/registry/controller", method = "post")]
    pub async fn registry_controller(
        &self,
        registry: Data<&Registry>,
        vm: Data<&ServiceVm>,
        zkvm: Data<&ServiceZkVm>,
        ctx: Data<&Context>,
        request: Json<RegisterControllerRequest>,
    ) -> poem::Result<Json<RegisterControllerResponse>> {
        let controller = ControllerData {
            controller: request.controller.to_vec(),
            circuit: request.circuit.to_vec(),
            nonce: request.nonce.unwrap_or(0),
        };

        let controller = registry
            .register_controller(*vm, *zkvm, *ctx, controller)
            .map_err(perr)?;
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
        zkvm: Data<&ServiceZkVm>,
        ctx: Data<&Context>,
        request: Json<RegisterDomainRequest>,
    ) -> poem::Result<Json<RegisterDomainResponse>> {
        let domain = DomainData {
            name: request.name.clone(),
            controller: request.controller.to_vec(),
            circuit: request.circuit.to_vec(),
        };

        let domain = registry
            .register_domain(*vm, *zkvm, *ctx, domain)
            .map_err(perr)?;
        let domain = RegisterDomainResponse {
            domain: hex::encode(domain),
        };

        Ok(Json(domain))
    }

    /// Returns the raw storage of the controller.
    #[oai(path = "/registry/controller/:controller/storage/raw", method = "get")]
    pub async fn get_storage_raw(
        &self,
        controller: Path<String>,
        ctx: Data<&Context>,
    ) -> poem::Result<Json<String>> {
        let controller = try_str_to_hash(&controller).map_err(perr)?;
        let ctx = ctx.clone().with_controller(controller);

        let data = ctx.get_raw_storage().map_err(perr)?.unwrap_or_default();
        let data = valence_coprocessor::Base64::encode(data);

        Ok(Json(data))
    }

    /// Replaces the raw storage of the controller.
    #[oai(path = "/registry/controller/:controller/storage/raw", method = "post")]
    pub async fn set_storage_raw(
        &self,
        controller: Path<String>,
        ctx: Data<&Context>,
        base64: Json<String>,
    ) -> poem::Result<Json<Value>> {
        let controller = try_str_to_hash(&controller).map_err(perr)?;
        let ctx = ctx.clone().with_controller(controller);
        let data = valence_coprocessor::Base64::decode(&*base64).map_err(perr)?;

        ctx.set_raw_storage(&data).map_err(perr)?;

        Ok(Json(serde_json::json!({
            "success": true
        })))
    }

    /// Returns a file from the storage of the controller.
    #[oai(path = "/registry/controller/:controller/storage/fs", method = "post")]
    pub async fn get_storage_file(
        &self,
        controller: Path<String>,
        ctx: Data<&Context>,
        request: Json<ControllerStorageFileRequest>,
    ) -> poem::Result<Json<Value>> {
        let path = request.0.path;

        tracing::debug!("received file request for path `{path}`...");

        let controller = try_str_to_hash(&controller).map_err(perr)?;
        let ctx = ctx.clone().with_controller(controller);
        let data = ctx.get_storage_file(&path).map_err(perr)?;
        let data = data.map(valence_coprocessor::Base64::encode);

        Ok(Json(json!(data)))
    }

    /// Computes the witnesses for a controller proof.
    #[oai(path = "/registry/controller/:controller/witnesses", method = "post")]
    pub async fn controller_witnesses(
        &self,
        controller: Path<String>,
        ctx: Data<&Context>,
        vm: Data<&ServiceVm>,
        request: Json<ControllerProveRequest>,
    ) -> poem::Result<Json<ControllerWitnessesResponse>> {
        let ControllerProveRequest { args, .. } = request.0;

        let controller = try_str_to_hash(&controller).map_err(perr)?;
        let ctx = ctx.clone().with_controller(controller);
        let witnesses = ctx.get_circuit_witnesses(*vm, args).map_err(perr)?;
        let witnesses = ctx.get_coprocessor_witness(witnesses).map_err(perr)?;
        let witnesses = serde_json::to_value(witnesses).unwrap_or_default();
        let log = ctx.get_log().unwrap_or_default();

        Ok(Json(ControllerWitnessesResponse { witnesses, log }))
    }

    /// Computes the controller proof.
    #[oai(path = "/registry/controller/:controller/prove", method = "post")]
    pub async fn controller_prove(
        &self,
        controller: Path<String>,
        pool: Data<&Sender<Job>>,
        vm: Data<&ServiceVm>,
        ctx: Data<&Context>,
        request: Json<ControllerProveRequest>,
    ) -> poem::Result<Json<Value>> {
        let ControllerProveRequest { args, payload } = request.0;

        let controller = try_str_to_hash(&controller).map_err(perr)?;
        let ctx = ctx.clone().with_controller(controller);
        let witnesses = ctx.get_circuit_witnesses(*vm, args).map_err(perr)?;
        let witness = ctx.get_coprocessor_witness(witnesses).map_err(perr)?;

        tracing::debug!("coprocessor witness computed; submitting job...");

        pool.send(Job::Prove {
            circuit: controller,
            witness,
            payload,
        })
        .map_err(|e| anyhow::anyhow!("failed to submit prove job: {e}"))
        .map_err(perr)?;

        Ok(Json(json!({"status": "received"})))
    }

    /// Computes the controller proof for the provided co-processor root.
    #[oai(path = "/registry/controller/:controller/prove/:root", method = "post")]
    pub async fn controller_prove_root(
        &self,
        controller: Path<String>,
        root: Path<String>,
        pool: Data<&Sender<Job>>,
        vm: Data<&ServiceVm>,
        ctx: Data<&Context>,
        request: Json<ControllerProveRequest>,
    ) -> poem::Result<Json<Value>> {
        let ControllerProveRequest { args, payload } = request.0;

        let controller = try_str_to_hash(&controller).map_err(perr)?;
        let root = try_str_to_hash(&root).map_err(perr)?;
        let ctx = ctx
            .clone()
            .with_controller(controller)
            .with_historical(root);

        let witnesses = ctx.get_circuit_witnesses(*vm, args).map_err(perr)?;
        let witness = ctx.get_coprocessor_witness(witnesses).map_err(perr)?;

        tracing::debug!("coprocessor witness computed; submitting job...");

        pool.send(Job::Prove {
            circuit: controller,
            witness,
            payload,
        })
        .map_err(|e| anyhow::anyhow!("failed to submit prove job: {e}"))
        .map_err(perr)?;

        Ok(Json(json!({"status": "received"})))
    }

    /// Returns the controller verifying key.
    #[oai(path = "/registry/controller/:controller/vk", method = "get")]
    pub async fn controller_vk(
        &self,
        controller: Path<String>,
        ctx: Data<&Context>,
        zkvm: Data<&ServiceZkVm>,
    ) -> poem::Result<Json<ControllerVkResponse>> {
        let controller = try_str_to_hash(&controller).map_err(perr)?;
        let ctx = ctx.clone().with_controller(controller);

        let vk = ctx.get_verifying_key(*zkvm).map_err(perr)?;
        let log = ctx.get_log().map_err(perr)?;

        Ok(Json(ControllerVkResponse {
            base64: Base64(vk),
            log,
        }))
    }

    /// Returns the controller circuit bytecode.
    #[oai(path = "/registry/controller/:controller/circuit", method = "get")]
    pub async fn controller_circuit(
        &self,
        controller: Path<String>,
        ctx: Data<&Context>,
    ) -> poem::Result<Json<ControllerCircuitResponse>> {
        let controller = try_str_to_hash(&controller).map_err(perr)?;
        let ctx = ctx.clone().with_controller(controller);
        let circuit = ctx
            .get_zkvm()
            .map_err(perr)?
            .ok_or_else(|| anyhow::anyhow!("no circuit data available"))
            .map_err(perr)?;

        Ok(Json(ControllerCircuitResponse {
            base64: Base64(circuit),
        }))
    }

    /// Returns the controller runtime bytecode.
    #[oai(path = "/registry/controller/:controller/runtime", method = "get")]
    pub async fn controller_runtime(
        &self,
        controller: Path<String>,
        ctx: Data<&Context>,
    ) -> poem::Result<Json<ControllerCircuitResponse>> {
        let controller = try_str_to_hash(&controller).map_err(perr)?;
        let ctx = ctx.clone().with_controller(controller);
        let circuit = ctx
            .get_controller(&controller)
            .map_err(perr)?
            .ok_or_else(|| anyhow::anyhow!("no runtime data available"))
            .map_err(perr)?;

        Ok(Json(ControllerCircuitResponse {
            base64: Base64(circuit),
        }))
    }

    /// Calls the controller entrypoint.
    #[oai(path = "/registry/controller/:controller/entrypoint", method = "post")]
    pub async fn controller_entrypoint(
        &self,
        controller: Path<String>,
        ctx: Data<&Context>,
        vm: Data<&ServiceVm>,
        args: Json<Value>,
    ) -> poem::Result<Json<ControllerEntrypointResponse>> {
        tracing::debug!(
            "received entrypoint request for `{}` with {:?}",
            controller.as_str(),
            &args.0
        );

        let controller = try_str_to_hash(&controller).map_err(perr)?;
        let ctx = ctx.clone().with_controller(controller);

        let ret = ctx.entrypoint(*vm, args.0).map_err(perr)?;
        let log = ctx.get_log().map_err(perr)?;

        Ok(Json(ControllerEntrypointResponse { ret, log }))
    }

    /// Get the latest proven block for the domain.
    #[oai(path = "/registry/domain/:domain/latest", method = "get")]
    pub async fn domain_latest(
        &self,
        domain: Path<String>,
        ctx: Data<&Context>,
    ) -> poem::Result<Json<Value>> {
        let id = DomainData::identifier_from_parts(&domain);
        let ctx = ctx.clone().with_controller(id);
        let coprocessor = ctx.get_historical();

        let ValidatedDomainBlock {
            domain,
            number,
            root,
            payload,
        } = ctx
            .get_latest_block(&domain)
            .map_err(perr)?
            .ok_or_else(|| anyhow::anyhow!("no block data available for the domain"))?;

        Ok(Json(json!({
            "coprocessor": hex::encode(coprocessor),
            "domain": hex::encode(domain),
            "number": number,
            "root": hex::encode(root),
            "payload": hex::encode(payload),
        })))
    }

    /// Adds a new block to the domain.
    #[oai(path = "/registry/domain/:domain", method = "post")]
    pub async fn domain_add_block(
        &self,
        domain: Path<String>,
        historical: Data<&Historical>,
        vm: Data<&ServiceVm>,
        args: Json<Value>,
    ) -> poem::Result<Json<DomainAddBlockResponse>> {
        tracing::debug!("adding domain block for {}...", &*domain);

        let BlockAdded {
            domain,
            prev_smt,
            smt,
            log,
            block,
        } = historical
            .add_domain_block(*vm, &domain, args.0)
            .map_err(perr)?;

        let ValidatedDomainBlock {
            number,
            root,
            payload,
            ..
        } = block;

        Ok(Json(DomainAddBlockResponse {
            domain,
            prev_smt,
            smt,
            log,
            number,
            root,
            payload,
        }))
    }

    /// Co-processor root.
    #[oai(path = "/historical", method = "get")]
    pub async fn root(&self, historical: Data<&Historical>) -> poem::Result<Json<Value>> {
        let historical = historical.current();
        let historical = hex::encode(historical);

        Ok(Json(json!({
            "root": historical,
        })))
    }

    /// Get the historical update for the provided historical tree root.
    #[oai(path = "/historical/:root", method = "get")]
    pub async fn historical_update(
        &self,
        root: Path<String>,
        historical: Data<&Historical>,
    ) -> poem::Result<Json<Value>> {
        let root = try_str_to_hash(&root).map_err(perr)?;
        let update = historical.get_historical_update(&root).map_err(perr)?;
        let HistoricalUpdate {
            uuid,
            root,
            previous,
            block,
        } = match update {
            Some(u) => u,
            None if root == Hash::default() => HistoricalUpdate::default(),
            None => return Err(r404()),
        };

        Ok(Json(json!({
            "uuid": hex::encode(uuid),
            "root": hex::encode(root),
            "previous": hex::encode(previous),
            "block": block,
        })))
    }

    /// Get the historical proof for the provided domain.
    #[oai(path = "/historical/:domain/:number", method = "get")]
    pub async fn historical_proof(
        &self,
        domain: Path<String>,
        number: Path<String>,
        historical: Data<&Historical>,
    ) -> poem::Result<Json<Value>> {
        let number = number.parse().map_err(perr)?;
        let proof = historical
            .get_block_proof_for_domain(&domain, number)
            .map_err(perr)?;

        Ok(Json(json!(proof)))
    }

    /// Get a set of historical proofs for the provided interval
    #[oai(path = "/historical/bulk/:from/:to", method = "get")]
    pub async fn historical_proof_bulk(
        &self,
        from: Path<String>,
        to: Path<String>,
        historical: Data<&Historical>,
    ) -> poem::Result<Json<Value>> {
        tracing::debug!(
            "historical bulk proof request received from `{}` to `{}`...",
            from.as_str(),
            to.as_str()
        );

        let from = try_str_to_hash(&from).map_err(perr)?;
        let to = try_str_to_hash(&to).map_err(perr)?;

        // skip current update
        let from = historical
            .get_historical_update_from_previous(&from)
            .map_err(perr)?
            .ok_or_else(r400)?
            .root;

        tracing::debug!("previous root `{}`...", hex::encode(from));

        let from = historical
            .get_historical_update(&from)
            .map_err(perr)?
            .ok_or_else(r400)?;
        let mut to = historical
            .get_historical_update(&to)
            .map_err(perr)?
            .ok_or_else(r400)?;

        tracing::debug!(
            "historical range set from `{}` to `{}`...",
            hex::encode(from.block.root),
            hex::encode(to.block.root),
        );

        let mut updates = Vec::with_capacity(500);

        while from.uuid <= to.uuid {
            tracing::debug!(
                "fetch block proof for root `{}` on block `{}` for domain `{}`...",
                hex::encode(to.root),
                to.block.number,
                hex::encode(to.block.domain),
            );

            let proof = Historical::get_historical_transition_proof_with_data(
                historical.data().clone(),
                &to.root,
            )?;

            updates.push(proof);

            to = match historical
                .get_historical_update(&to.previous)
                .map_err(perr)?
            {
                Some(u) => u,
                None if to.root == from.root => break,
                _ => return Err(r500()),
            };
        }

        updates.reverse();

        tracing::debug!("provided `{}` updates.", updates.len());

        Ok(Json(serde_json::to_value(updates).unwrap_or_default()))
    }

    /// Returns the raw storage of the circuit.
    #[oai(path = "/circuit/storage/raw", method = "get")]
    pub async fn circuit_storage_raw_get(&self, ctx: Data<&Context>) -> poem::Result<Json<Value>> {
        let data = ctx
            .get_raw_storage()
            .map_err(perr)?
            .map(valence_coprocessor::Base64::encode);

        Ok(Json(json!(data)))
    }

    /// Replaces the raw storage of the circuit.
    #[oai(path = "/circuit/storage/raw", method = "post")]
    pub async fn circuit_storage_raw_post(
        &self,
        ctx: Data<&Context>,
        base64: Json<String>,
    ) -> poem::Result<Json<Value>> {
        let data = valence_coprocessor::Base64::decode(&*base64).map_err(perr)?;

        ctx.set_raw_storage(&data).map_err(perr)?;

        Ok(Json(Value::Null))
    }

    /// Returns a file from the storage of the circuit.
    ///
    /// Note: This is a FAT-16 filesystem, so extensions must have max 3 characters.
    #[oai(path = "/circuit/storage/fs", method = "get")]
    pub async fn circuit_storage_fs_get(
        &self,
        ctx: Data<&Context>,
        request: Json<ControllerStorageFileRequest>,
    ) -> poem::Result<Json<Value>> {
        let path = request.0.path;

        tracing::debug!("received file request for path `{path}`...");

        let data = ctx.get_storage_file(&path).map_err(perr)?;
        let data = data.map(valence_coprocessor::Base64::encode);

        Ok(Json(json!(data)))
    }

    /// Replaces a file from the storage of the circuit.
    ///
    /// Note: This is a FAT-16 filesystem, so extensions must have max 3 characters.
    #[oai(path = "/circuit/storage/fs", method = "post")]
    pub async fn circuit_storage_fs_post(
        &self,
        ctx: Data<&Context>,
        request: Json<ControllerStorageFileStoreRequest>,
    ) -> poem::Result<Json<Value>> {
        let ControllerStorageFileStoreRequest { path, contents } = request.0;

        tracing::debug!("received file store for path `{path}`...");

        if PathBuf::from(&path)
            .extension()
            .is_some_and(|ext| ext.len() > 3)
        {
            return Err(PoemError::from_string(
                "file extensions must have at most 3 characters",
                StatusCode::BAD_REQUEST,
            ));
        }

        let contents = valence_coprocessor::Base64::decode(contents).map_err(perr)?;

        ctx.set_storage_file(&path, &contents).map_err(perr)?;

        Ok(Json(Value::Null))
    }

    /// Computes the witnesses for a circuit proof.
    #[oai(path = "/circuit/witnesses", method = "post")]
    pub async fn circuit_witnesses(
        &self,
        ctx: Data<&Context>,
        vm: Data<&ServiceVm>,
        request: Json<ControllerProveRequest>,
    ) -> poem::Result<Json<ControllerWitnessesResponse>> {
        let ControllerProveRequest { args, .. } = request.0;

        tracing::debug!(
            "get witnesses for `{}` with `{:?}`...",
            hex::encode(ctx.controller()),
            args
        );

        fn return_with_log<E: ToString>(
            ctx: &Context,
            err: E,
        ) -> Json<ControllerWitnessesResponse> {
            let err = err.to_string();

            tracing::debug!("failed to compute witnesses: {err}");

            Json(ControllerWitnessesResponse {
                witnesses: Value::Null,
                log: [ctx.get_log().unwrap_or_default().as_slice(), &[err]].concat(),
            })
        }

        let witnesses = match ctx.get_circuit_witnesses(*vm, args) {
            Ok(w) => w,
            Err(e) => return Ok(return_with_log(&ctx, e)),
        };

        let witnesses = match ctx.get_coprocessor_witness(witnesses) {
            Ok(w) => w,
            Err(e) => return Ok(return_with_log(&ctx, e)),
        };

        let witnesses = serde_json::to_value(witnesses).unwrap_or_default();
        let log = ctx.get_log().unwrap_or_default();

        Ok(Json(ControllerWitnessesResponse { witnesses, log }))
    }

    /// Computes the circuit proof.
    #[oai(path = "/circuit/prove", method = "post")]
    pub async fn circuit_prove(
        &self,
        pool: Data<&Sender<Job>>,
        vm: Data<&ServiceVm>,
        ctx: Data<&Context>,
        request: Json<ControllerProveRequest>,
    ) -> poem::Result<Json<Value>> {
        let ControllerProveRequest { args, payload } = request.0;

        let witnesses = ctx.get_circuit_witnesses(*vm, args).map_err(perr)?;
        let witness = ctx.get_coprocessor_witness(witnesses).map_err(perr)?;
        let circuit = *ctx.controller();

        tracing::debug!("coprocessor witness computed; submitting job...");

        pool.send(Job::Prove {
            circuit,
            witness,
            payload,
        })
        .map_err(|e| anyhow::anyhow!("failed to submit prove job: {e}"))
        .map_err(perr)?;

        Ok(Json(json!({"status": "received"})))
    }

    /// Returns the circuit verifying key.
    #[oai(path = "/circuit/vk", method = "get")]
    pub async fn circuit_vk(
        &self,
        ctx: Data<&Context>,
        zkvm: Data<&ServiceZkVm>,
    ) -> poem::Result<Json<Value>> {
        let vk = ctx.get_verifying_key(*zkvm).map_err(perr)?;
        let vk = valence_coprocessor::Base64::encode(vk);

        Ok(Json(Value::String(vk)))
    }

    /// Returns the circuit bytecode.
    #[oai(path = "/circuit/bytecode", method = "get")]
    pub async fn circuit_bytecode(&self, ctx: Data<&Context>) -> poem::Result<Json<Value>> {
        let circuit = ctx
            .get_zkvm()
            .map_err(perr)?
            .ok_or_else(|| anyhow::anyhow!("no circuit data available"))
            .map_err(perr)?;
        let circuit = valence_coprocessor::Base64::encode(circuit);

        Ok(Json(Value::String(circuit)))
    }

    /// Returns the circuit runtime bytecode.
    #[oai(path = "/circuit/runtime", method = "get")]
    pub async fn circuit_runtime(&self, ctx: Data<&Context>) -> poem::Result<Json<Value>> {
        let controller = ctx.controller();
        let runtime = ctx
            .get_controller(controller)
            .map_err(perr)?
            .ok_or_else(|| anyhow::anyhow!("no runtime data available"))
            .map_err(perr)?;
        let runtime = valence_coprocessor::Base64::encode(runtime);

        Ok(Json(Value::String(runtime)))
    }

    /// Calls the circuit entrypoint.
    #[oai(path = "/circuit/entrypoint", method = "post")]
    pub async fn circuit_entrypoint(
        &self,
        ctx: Data<&Context>,
        vm: Data<&ServiceVm>,
        args: Json<Value>,
    ) -> poem::Result<Json<ControllerEntrypointResponse>> {
        tracing::debug!("received entrypoint request for {:?}", &args.0);

        let ret = ctx.entrypoint(*vm, args.0).map_err(perr)?;
        let log = ctx.get_log().map_err(perr)?;

        Ok(Json(ControllerEntrypointResponse { ret, log }))
    }
}

fn perr<E: ToString>(err: E) -> PoemError {
    PoemError::from_string(err.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
}

fn r400() -> poem::Error {
    poem::Error::from_status(StatusCode::BAD_REQUEST)
}

fn r404() -> poem::Error {
    poem::Error::from_status(StatusCode::NOT_FOUND)
}

fn r500() -> poem::Error {
    poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
}

fn try_str_to_hash(hash: &str) -> anyhow::Result<Hash> {
    let bytes =
        hex::decode(hash).map_err(|e| anyhow::anyhow!("error converting str to hash: {e}"))?;

    Hash::try_from(bytes).map_err(|_| anyhow::anyhow!("error converting bytes to hash"))
}
