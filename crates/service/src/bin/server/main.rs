use poem::{listener::TcpListener, EndpointExt as _, Route};
use poem_openapi::OpenApiService;
use tracing_subscriber::{fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};
use valence_coprocessor::Registry;
use valence_coprocessor_rocksdb::RocksBackend;
use valence_coprocessor_service::{api::Api, Config, ValenceWasm};
use valence_coprocessor_sp1::Sp1ZkVm;

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
    let module = ValenceWasm::new(config.module_cache_capacity)?;
    let mode = valence_coprocessor_sp1::Mode::try_from(config.zkvm_mode.as_str())?;
    let zkvm = Sp1ZkVm::new(mode, config.zkvm_cache_capacity)?;

    tracing::info!("registry loaded...");

    let api_service = OpenApiService::new(Api, env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
        .server(format!("http://{}/api", &config.socket));
    let ui = api_service.swagger_ui();
    let app = Route::new()
        .nest("/", ui)
        .nest("/api", api_service)
        .data(registry)
        .data(rocks)
        .data(module)
        .data(zkvm);

    tracing::info!("API loaded, listening on `{}`...", &config.socket);

    poem::Server::new(TcpListener::bind(&config.socket))
        .run(app)
        .await?;

    Ok(())
}
