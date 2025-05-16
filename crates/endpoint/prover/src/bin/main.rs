use std::net::{SocketAddr, TcpListener};

use clap::Parser;
use tracing_subscriber::{fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};
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

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("error accepting new connection: {e}");
                continue;
            }
        };

        let stream = match tungstenite::accept(stream) {
            Ok(s) => s,
            Err(e) => {
                tracing::debug!("error on websocket handshake: {e}");
                continue;
            }
        };

        if let Err(e) = pool.send(Task::Conn(stream)) {
            tracing::error!("error submitting connection to the pool: {e}");
        }
    }

    tracing::info!("shutting down...");

    Ok(())
}
