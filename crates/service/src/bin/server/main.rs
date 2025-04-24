use poem::{listener::TcpListener, web::Data, EndpointExt as _, Route};
use poem_openapi::{payload::Json, types::Base64, Object, OpenApi, OpenApiService};
use tracing_subscriber::{fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};
use valence_coprocessor::{DomainData, ProgramData, Registry};
use valence_coprocessor_rocksdb::RocksBackend;
use valence_coprocessor_service::Config;

struct Api;

#[derive(Object, Debug)]
struct RegisterProgramRequest {
    /// A Base64 WASM encoded module.
    module: Base64<Vec<u8>>,

    /// A Base64 zkVM encoded prover.
    zkvm: Base64<Vec<u8>>,

    /// Optional nonce to affect hte program id.
    #[oai(default)]
    nonce: Option<u64>,
}

#[derive(Object, Debug)]
struct RegisterProgramResponse {
    /// The allocated program id as base64.
    program: Base64<Vec<u8>>,
}

#[derive(Object, Debug)]
struct RegisterDomainRequest {
    /// Unique name identifier for the domain.
    name: String,

    /// Base64 code for the WASM module.
    module: Base64<Vec<u8>>,
}

#[derive(Object, Debug)]
struct RegisterDomainResponse {
    /// The allocated domain id as base64.
    domain: Base64<Vec<u8>>,
}

#[OpenApi]
impl Api {
    /// Register a new program, returning its allocated id.
    #[oai(path = "/registry/program", method = "post")]
    async fn registry_program(
        &self,
        registry: Data<&Registry<RocksBackend>>,
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
    async fn register_domain(
        &self,
        registry: Data<&Registry<RocksBackend>>,
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter_layer = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = fmt::layer().with_target(false);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    let (path, config) = Config::create_or_read_default()?;

    tracing::info!("config file loaded from `{}`...", path.display());

    let rocks = RocksBackend::open(&config.data_dir)?;

    tracing::info!("db loaded from `{}`...", config.data_dir.display());

    let registry = Registry::from(rocks.clone());

    let api_service = OpenApiService::new(Api, env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
        .server(format!("http://{}/api", &config.socket));
    let ui = api_service.swagger_ui();
    let app = Route::new()
        .nest("/", ui)
        .nest("/api", api_service)
        .data(registry);

    poem::Server::new(TcpListener::bind(&config.socket))
        .run(app)
        .await?;

    Ok(())
}
