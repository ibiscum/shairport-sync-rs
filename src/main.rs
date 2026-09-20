mod audio;
mod config;
mod runtime;

use clap::Parser;
use config::{AppConfig, Cli};
use shairplay::RaopServer;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        error!("{e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    init_tracing();

    let cli = Cli::parse();
    let cfg = AppConfig::load(&cli)?;

    let handler = audio::make_handler(&cfg);

    let mut builder = RaopServer::builder()
        .name(cfg.name.clone())
        .port(cfg.port)
        .max_clients(cfg.max_clients);

    if let Some(password) = cfg.password {
        builder = builder.password(password);
    }

    let mut server = builder
        .build(handler)
        .map_err(|e| format!("failed to build RAOP server: {e}"))?;

    server
        .start()
        .await
        .map_err(|e| format!("failed to start RAOP server: {e}"))?;

    info!(
        name = cfg.name,
        port = cfg.port,
        backend = ?cfg.backend,
        "shairport-sync-rs started"
    );

    runtime::run_until_shutdown(&mut server).await
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
