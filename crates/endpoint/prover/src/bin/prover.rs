use std::{
    env,
    net::{SocketAddr, TcpListener},
};

use clap::Parser;
use rand::RngCore as _;
use tracing_subscriber::{fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};
use valence_coprocessor::{Blake3Hasher, Hasher as _};
use valence_coprocessor_prover::{pool::Pool, types::Task};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Bind to the provided socket
    #[arg(short, long, value_name = "SOCKET", default_value = "0.0.0.0:37282")]
    bind: SocketAddr,

    /// Keys cache capacity
    #[arg(short, long, value_name = "CACHE", default_value_t = 20)]
    cache: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli { bind, cache } = Cli::parse();

    let filter_layer = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = fmt::layer().with_target(false);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    tracing::info!("initializing pool...");

    let pool = Pool::new(cache).run();

    tracing::info!("binding to `{bind}`...");

    let listener = TcpListener::bind(bind)?;

    tracing::info!("accepting connections...");

    let secret = env::var("VALENCE_PROVER_SECRET").unwrap_or_default();

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("error accepting new connection: {e}");
                continue;
            }
        };

        let mut stream = match tungstenite::accept(stream) {
            Ok(s) => s,
            Err(e) => {
                tracing::debug!("error on websocket handshake: {e}");
                continue;
            }
        };

        let challenge = rand::rng().next_u32().to_le_bytes().to_vec();
        let expected = Blake3Hasher::hash(&[secret.as_bytes(), challenge.as_slice()].concat());

        if let Err(e) = stream.send(challenge.into()) {
            tracing::warn!("error submitting challenge: {e}");
            continue;
        }

        let challenge = match stream.read() {
            Ok(m) => m.into_data().to_vec(),
            Err(e) => {
                tracing::warn!("error receiving challenge: {e}");
                continue;
            }
        };

        if challenge != expected {
            tracing::warn!("invalid challenge; discarding connection...");
            continue;
        }

        if let Err(e) = pool.send(Task::Conn(stream)) {
            tracing::error!("error submitting connection to the pool: {e}");
        }
    }

    tracing::info!("shutting down...");

    Ok(())
}
