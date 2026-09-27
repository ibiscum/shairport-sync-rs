mod audio;
mod config;
mod observability;
mod runtime;

use clap::Parser;
use config::{AirPlayModeConfig, Ap1CodecConfig, Ap1EncryptionConfig, AppConfig, Cli, LogFormat};
use observability::ActivityMonitor;
use shairplay::{AirPlayMode, Ap1Codec, Ap1Encryption, RaopServer, RaopServerBuilder};
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::runtime::pairing_store::FilePairingStore;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        error!("{e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let cli = Cli::parse();
    let cfg = AppConfig::load(&cli)?;
    init_tracing(&cfg);

    let activity_interval_secs = std::env::var("SSR_ACTIVITY_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(cfg.activity_interval_secs);
    let activity_snapshot_path = std::env::var("SSR_ACTIVITY_SNAPSHOT_PATH")
        .ok()
        .or(cfg.activity_snapshot_path.clone());

    let activity_monitor = Arc::new(ActivityMonitor::new(
        activity_interval_secs,
        activity_snapshot_path,
    ));
    let activity_logger_task = activity_monitor.spawn_periodic_logger();
    let handler = audio::make_handler_with_monitor(&cfg, Arc::clone(&activity_monitor));

    let mut builder = RaopServer::builder()
        .name(cfg.name.clone())
        .port(cfg.port)
        .max_clients(cfg.max_clients);

    builder = apply_raop_protocol_config(builder, &cfg)?;

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

    let run_result = runtime::run_until_shutdown(&mut server).await;
    activity_monitor.log_snapshot("shutdown");
    activity_logger_task.abort();
    run_result
}

fn init_tracing(cfg: &AppConfig) {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,shairplay=info"));
    let log_format = std::env::var("SSR_LOG_FORMAT")
        .ok()
        .map(|v| {
            if v.eq_ignore_ascii_case("json") {
                LogFormat::Json
            } else {
                LogFormat::Text
            }
        })
        .unwrap_or(cfg.log_format);

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true);

    if matches!(log_format, LogFormat::Json) {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt_layer
                    .json()
                    .with_current_span(true)
                    .with_span_list(true),
            )
            .init();
        return;
    }

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}

fn apply_raop_protocol_config(
    mut builder: RaopServerBuilder,
    cfg: &AppConfig,
) -> Result<RaopServerBuilder, String> {
    builder = match cfg.airplay_mode {
        AirPlayModeConfig::Ap1 => builder.mode(AirPlayMode::AirPlay1),
        AirPlayModeConfig::Ap2 => builder.mode(AirPlayMode::AirPlay2),
    };

    if let Some(path) = cfg.ap2_pairing_store_path.as_deref() {
        let store = FilePairingStore::load_or_create(path)
            .map_err(|e| format!("failed to initialize AP2 pairing store: {e}"))?;
        builder = builder.pairing_store(Arc::new(store));
    }

    if let Some(rate) = cfg.raop_output_sample_rate {
        builder = builder.output_sample_rate(rate);
    }

    if let Some(channels) = cfg.raop_output_max_channels {
        builder = builder.output_max_channels(channels);
    }

    if let Some(codecs) = &cfg.ap1_codecs {
        let mapped: Vec<Ap1Codec> = codecs
            .iter()
            .map(|v| match v {
                Ap1CodecConfig::Pcm => Ap1Codec::Pcm,
                Ap1CodecConfig::Alac => Ap1Codec::Alac,
            })
            .collect();
        builder = builder.advertise_codecs(mapped);
    }

    if let Some(encryption) = &cfg.ap1_encryption {
        let mapped: Vec<Ap1Encryption> = encryption
            .iter()
            .map(|v| match v {
                Ap1EncryptionConfig::None => Ap1Encryption::None,
                Ap1EncryptionConfig::Rsa => Ap1Encryption::Rsa,
                Ap1EncryptionConfig::Fairplay => Ap1Encryption::FairPlay,
            })
            .collect();
        builder = builder.advertise_encryption(mapped);
    }

    if let Some(pin) = &cfg.ap2_pin {
        builder = builder.pin(pin.clone());
    }

    Ok(builder)
}
