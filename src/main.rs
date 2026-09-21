mod audio;
mod config;
mod runtime;

use clap::Parser;
use config::{AirPlayModeConfig, Ap1CodecConfig, Ap1EncryptionConfig, AppConfig, Cli};
use shairplay::{AirPlayMode, Ap1Codec, Ap1Encryption, BindConfig, RaopServer, RaopServerBuilder};
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::runtime::pairing_store::FilePairingStore;
use crate::runtime::singleton::InstanceLock;

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

    let _instance_lock = InstanceLock::acquire(&cfg.name, cfg.port)?;

    let handler = audio::make_handler(&cfg);
    let bind = BindConfig::new().port(cfg.port).exact_port();

    let mut builder = RaopServer::builder()
        .name(cfg.name.clone())
        .bind(bind)
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

    runtime::run_until_shutdown(&mut server).await
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
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
