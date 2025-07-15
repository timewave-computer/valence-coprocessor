use std::net::SocketAddr;

use clap::Parser;
use poem::{listener::TcpListener, EndpointExt as _, Route};
use poem_openapi::OpenApiService;
use tracing_subscriber::{fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};
use valence_coprocessor::Registry;
use valence_coprocessor_redis::RedisBackend;
use valence_coprocessor_service::{
    api::Api, data::ServiceBackend, worker::Pool, Historical, ServiceVm, ServiceZkVm,
};

#[derive(Parser)]
struct Cli {
    /// Bind to the provided socket
    #[arg(short, long, value_name = "SOCKET", default_value = "0.0.0.0:37281")]
    bind: SocketAddr,

    /// Socket to the Redis data backend. Fallback to memory data.
    #[arg(short, long, value_name = "REDIS")]
    redis: Option<SocketAddr>,

    /// Socket to the Prover service backend. Fallback to SP1 mock prover.
    #[arg(short, long, value_name = "PROVER")]
    prover: Option<SocketAddr>,

    /// Cache capacity
    #[arg(short, long, value_name = "CAPACITY", default_value_t = 100)]
    capacity: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli {
        bind,
        redis,
        prover,
        capacity,
    } = Cli::parse();

    let filter_layer = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = fmt::layer().with_target(false);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    let data = match redis {
        Some(redis) => RedisBackend::open(format!("redis://{redis}/"))?.into(),
        None => ServiceBackend::Memory(Default::default()),
    };

    tracing::info!("service backend set to `{}`...", data);

    let registry = Registry::from(data.clone());
    let vm = ServiceVm::new(capacity)?;

    let zkvm = match prover {
        Some(addr) => ServiceZkVm::service(addr)?,
        None => ServiceZkVm::mock(capacity)?,
    };

    tracing::info!("initiating historical tree...");

    let historical = Historical::load(data)?;

    tracing::info!("initiating pool...");

    let pool = Pool::new(historical.clone(), vm.clone(), zkvm.clone()).run();

    tracing::info!("registry loaded...");

    let api_service = OpenApiService::new(Api, env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
        .server(format!("http://{}/api", &bind));

    let app = Route::new()
        .nest("/", api_service.swagger_ui())
        .nest("/spec", api_service.spec_endpoint())
        .nest("/spec/yaml", api_service.spec_endpoint_yaml())
        .nest("/api", api_service)
        .data(registry)
        .data(vm)
        .data(zkvm)
        .data(historical)
        .data(pool);

    tracing::info!("API loaded, listening on `{}`...", &bind);

    poem::Server::new(TcpListener::bind(&bind)).run(app).await?;

    Ok(())
}
