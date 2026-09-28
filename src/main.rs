mod audio;
mod config;
mod observability;
mod runtime;

use clap::Parser;
use config::{AirPlayModeConfig, Ap1CodecConfig, Ap1EncryptionConfig, AppConfig, Cli, LogFormat};
use observability::ActivityMonitor;
use shairplay::{AirPlayMode, Ap1Codec, Ap1Encryption, RaopServer, RaopServerBuilder};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{error, info, warn};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
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

    if cfg.diagnostics.disable_resend_requests {
        warn!(
            "diagnostics.disable_resend_requests is configured but not yet wired to protocol behavior"
        );
    }
    if cfg.diagnostics.drop_this_fraction_of_audio_packets > 0.0 {
        warn!(
            drop_fraction = cfg.diagnostics.drop_this_fraction_of_audio_packets,
            "diagnostics.drop_this_fraction_of_audio_packets is configured but packet-loss simulation is not yet wired"
        );
    }
    if cfg.diagnostics.retain_cover_art {
        warn!("diagnostics.retain_cover_art is configured but metadata artwork retention is not yet wired");
    }
    if cfg.diagnostics.get_plist_metadata {
        warn!("diagnostics.get_plist_metadata is configured but plist metadata stream is not yet wired");
    }

    let activity_monitor = Arc::new(ActivityMonitor::new(
        cfg.activity_interval_secs,
        cfg.activity_snapshot_path.clone(),
    ));
    let activity_logger_task = if cfg.diagnostics.statistics {
        Some(activity_monitor.spawn_periodic_logger())
    } else {
        None
    };
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
    if let Some(task) = activity_logger_task {
        task.abort();
    }
    run_result
}

fn init_tracing(cfg: &AppConfig) {
    let default_filter = default_filter_for_verbosity(cfg.diagnostics.log_verbosity);
    let env_filter = match cfg.log_filter.as_deref() {
        Some(raw) => match EnvFilter::try_new(raw) {
            Ok(v) => v,
            Err(e) => {
                eprintln!(
                    "invalid log_filter '{}': {e}; falling back to '{}'",
                    raw, default_filter
                );
                EnvFilter::new(default_filter)
            }
        },
        None => EnvFilter::new(default_filter),
    };
    let log_format = cfg.log_format;
    let show_file_and_line = cfg.diagnostics.log_show_file_and_line;
    let use_uptime = cfg.diagnostics.log_show_time_since_startup;
    let use_delta = cfg.diagnostics.log_show_time_since_last_message;

    if use_uptime && use_delta {
        warn!(
            "both diagnostics.log_show_time_since_startup and diagnostics.log_show_time_since_last_message are enabled; using time-since-last-message"
        );
    }

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(show_file_and_line)
        .with_line_number(show_file_and_line);

    if matches!(log_format, LogFormat::Json) {
        if use_delta {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt_layer
                        .with_timer(DeltaTimer::new())
                        .json()
                        .with_current_span(true)
                        .with_span_list(true),
                )
                .init();
            return;
        }

        if use_uptime {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt_layer
                        .with_timer(UptimeTimer::new())
                        .json()
                        .with_current_span(true)
                        .with_span_list(true),
                )
                .init();
            return;
        }

        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt_layer
                    .with_timer(SystemTimer::new())
                    .json()
                    .with_current_span(true)
                    .with_span_list(true),
            )
            .init();
        return;
    }

    if use_delta {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer.with_timer(DeltaTimer::new()))
            .init();
        return;
    }

    if use_uptime {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer.with_timer(UptimeTimer::new()))
            .init();
        return;
    }

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer.with_timer(SystemTimer::new()))
        .init();
}

fn default_filter_for_verbosity(level: u8) -> &'static str {
    match level {
        0 => "info,shairplay=info",
        1 => "debug,shairplay=debug",
        _ => "trace,shairplay=trace",
    }
}

struct SystemTimer;

impl SystemTimer {
    fn new() -> Self {
        Self
    }
}

impl FormatTime for SystemTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);
        write!(w, "{}.{:03}", now.as_secs(), now.subsec_millis())
    }
}

struct UptimeTimer {
    start: Instant,
}

impl UptimeTimer {
    fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl FormatTime for UptimeTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        let elapsed = self.start.elapsed();
        write!(w, "+{}.{:03}s", elapsed.as_secs(), elapsed.subsec_millis())
    }
}

struct DeltaTimer {
    last: Mutex<Option<Instant>>,
}

impl DeltaTimer {
    fn new() -> Self {
        Self {
            last: Mutex::new(None),
        }
    }
}

impl FormatTime for DeltaTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        let now = Instant::now();
        let mut guard = match self.last.lock() {
            Ok(v) => v,
            Err(_) => return write!(w, "+0.000s"),
        };
        let delta = match *guard {
            Some(last) => now.saturating_duration_since(last),
            None => Duration::ZERO,
        };
        *guard = Some(now);
        write!(w, "+{}.{:03}s", delta.as_secs(), delta.subsec_millis())
    }
}

fn apply_raop_protocol_config(
    mut builder: RaopServerBuilder,
    cfg: &AppConfig,
) -> Result<RaopServerBuilder, String> {
    if cfg.diagnostics.log_verbosity >= 3 {
        log_protocol_startup(cfg);
    }

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

fn log_protocol_startup(cfg: &AppConfig) {
    info!(
        target: "protocol",
        name = %cfg.name,
        port = cfg.port,
        mode = ?cfg.airplay_mode,
        password = cfg.password.is_some(),
        "protocol startup configuration"
    );

    if cfg.airplay_mode == AirPlayModeConfig::Ap1 {
        let codec_advert = match cfg.ap1_codecs.as_deref() {
            Some(values) => values
                .iter()
                .map(ap1_codec_label)
                .collect::<Vec<_>>()
                .join(","),
            None => "default(shairplay)".to_string(),
        };

        let encryption_advert = match cfg.ap1_encryption.as_deref() {
            Some(values) => values
                .iter()
                .map(ap1_encryption_label)
                .collect::<Vec<_>>()
                .join(","),
            None => "default(shairplay)".to_string(),
        };

        info!(
            target: "protocol",
            txt_cn = %codec_advert,
            txt_et = %encryption_advert,
            txt_tp = "TCP,UDP",
            txt_pw = if cfg.password.is_some() { "true" } else { "false" },
            txt_sr = cfg.raop_output_sample_rate.unwrap_or(44_100),
            txt_ch = cfg.raop_output_max_channels.unwrap_or(2),
            "AP1 mDNS advertisement intent"
        );
    }
}

fn ap1_codec_label(codec: &Ap1CodecConfig) -> &'static str {
    match codec {
        Ap1CodecConfig::Pcm => "pcm",
        Ap1CodecConfig::Alac => "alac",
    }
}

fn ap1_encryption_label(mode: &Ap1EncryptionConfig) -> &'static str {
    match mode {
        Ap1EncryptionConfig::None => "none",
        Ap1EncryptionConfig::Rsa => "rsa",
        Ap1EncryptionConfig::Fairplay => "fairplay",
    }
}
