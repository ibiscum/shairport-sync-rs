use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "shairport-sync-rs",
    version,
    about = "Rust-native AirPlay audio receiver"
)]
pub struct Cli {
    #[arg(long, value_name = "FILE", help = "Path to TOML config file")]
    pub config: Option<PathBuf>,

    #[arg(long, help = "AirPlay display name")]
    pub name: Option<String>,

    #[arg(long, help = "RTSP listen port")]
    pub port: Option<u16>,

    #[arg(long, value_enum, help = "Audio sink backend")]
    pub backend: Option<AudioBackend>,

    #[arg(
        long,
        value_enum,
        help = "Output sample format for pipe/stdout backends"
    )]
    pub output_format: Option<OutputSampleFormat>,

    #[arg(
        long,
        help = "ALSA playback device name (Linux only), e.g. default or hw:0,0"
    )]
    pub alsa_device: Option<String>,

    #[arg(
        long,
        help = "ALSA period size in frames (Linux only, backend=alsa)"
    )]
    pub alsa_period_frames: Option<u32>,

    #[arg(
        long,
        help = "ALSA buffer size in frames (Linux only, backend=alsa)"
    )]
    pub alsa_buffer_frames: Option<u32>,

    #[arg(long, help = "Output file/path for pipe backend (raw PCM stream)")]
    pub pipe_path: Option<String>,

    #[arg(long, help = "HTTP Digest password")]
    pub password: Option<String>,

    #[arg(long, help = "Maximum concurrent clients")]
    pub max_clients: Option<usize>,

    #[arg(
        long,
        help = "Resample AirPlay output to this rate before backend delivery"
    )]
    pub raop_output_sample_rate: Option<u32>,

    #[arg(long, help = "Downmix AirPlay output to this maximum channel count")]
    pub raop_output_max_channels: Option<u8>,

    #[arg(
        long,
        value_enum,
        value_delimiter = ',',
        help = "Advertised AP1 codecs (comma-separated), e.g. pcm,alac"
    )]
    pub ap1_codecs: Option<Vec<Ap1CodecConfig>>,

    #[arg(
        long,
        value_enum,
        value_delimiter = ',',
        help = "Advertised AP1 encryption modes (comma-separated), e.g. none,rsa,fairplay"
    )]
    pub ap1_encryption: Option<Vec<Ap1EncryptionConfig>>,

    #[arg(long, value_enum, help = "AirPlay protocol mode to advertise")]
    pub airplay_mode: Option<AirPlayModeConfig>,

    #[arg(long, help = "Require AP2 HomeKit pairing with this one-time PIN")]
    pub ap2_pin: Option<String>,

    #[arg(long, help = "Path to AP2 pairing persistence file")]
    pub ap2_pairing_store_path: Option<String>,

    #[arg(
        long,
        value_name = "FILE",
        help = "Path to RAOP RSA private key PEM file (overrides SHAIRPLAY_RSA_KEY_PATH)"
    )]
    pub rsa_key_path: Option<String>,

    #[arg(long, help = "Activity snapshot interval in seconds")]
    pub activity_interval_secs: Option<u64>,

    #[arg(long, help = "Path to JSONL activity snapshot output file")]
    pub activity_snapshot_path: Option<String>,

    #[arg(long, value_enum, help = "Runtime log output format")]
    pub log_format: Option<LogFormat>,

    #[arg(
        long,
        help = "Tracing filter directives, e.g. 'info,shairplay=debug'"
    )]
    pub log_filter: Option<String>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioBackend {
    Null,
    Stdout,
    Pipe,
    #[cfg(target_os = "linux")]
    Alsa,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Deserialize)]
pub enum OutputSampleFormat {
    #[value(name = "f32le")]
    #[serde(rename = "f32le")]
    F32Le,
    #[value(name = "s16le")]
    #[serde(rename = "s16le")]
    S16Le,
    #[value(name = "s24le")]
    #[serde(rename = "s24le")]
    S24Le,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ap1CodecConfig {
    Pcm,
    Alac,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ap1EncryptionConfig {
    None,
    Rsa,
    Fairplay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Deserialize)]
pub enum AirPlayModeConfig {
    #[value(name = "ap1")]
    #[serde(rename = "ap1")]
    Ap1,
    #[value(name = "ap2")]
    #[serde(rename = "ap2")]
    Ap2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogFormat {
    Text,
    Json,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub name: String,
    pub port: u16,
    pub backend: AudioBackend,
    pub output_format: OutputSampleFormat,
    pub alsa_device: Option<String>,
    pub alsa_period_frames: Option<u32>,
    pub alsa_buffer_frames: Option<u32>,
    pub pipe_path: Option<String>,
    pub password: Option<String>,
    pub max_clients: usize,
    pub raop_output_sample_rate: Option<u32>,
    pub raop_output_max_channels: Option<u8>,
    pub ap1_codecs: Option<Vec<Ap1CodecConfig>>,
    pub ap1_encryption: Option<Vec<Ap1EncryptionConfig>>,
    pub airplay_mode: AirPlayModeConfig,
    pub ap2_pin: Option<String>,
    pub ap2_pairing_store_path: Option<String>,
    pub rsa_key_path: Option<String>,
    pub activity_interval_secs: u64,
    pub activity_snapshot_path: Option<String>,
    pub log_format: LogFormat,
    pub log_filter: Option<String>,
    pub diagnostics: DiagnosticsConfig,
}

#[derive(Debug, Clone)]
pub struct DiagnosticsConfig {
    pub disable_resend_requests: bool,
    pub statistics: bool,
    pub log_verbosity: u8,
    pub log_show_file_and_line: bool,
    pub log_show_time_since_startup: bool,
    pub log_show_time_since_last_message: bool,
    pub drop_this_fraction_of_audio_packets: f64,
    pub retain_cover_art: bool,
    pub get_plist_metadata: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct FileConfig {
    name: Option<String>,
    port: Option<u16>,
    backend: Option<AudioBackend>,
    output_format: Option<OutputSampleFormat>,
    alsa_device: Option<String>,
    alsa_period_frames: Option<u32>,
    alsa_buffer_frames: Option<u32>,
    pipe_path: Option<String>,
    password: Option<String>,
    max_clients: Option<usize>,
    raop_output_sample_rate: Option<u32>,
    raop_output_max_channels: Option<u8>,
    ap1_codecs: Option<Vec<Ap1CodecConfig>>,
    ap1_encryption: Option<Vec<Ap1EncryptionConfig>>,
    airplay_mode: Option<AirPlayModeConfig>,
    ap2_pin: Option<String>,
    ap2_pairing_store_path: Option<String>,
    rsa_key_path: Option<String>,
    activity_interval_secs: Option<u64>,
    activity_snapshot_path: Option<String>,
    log_format: Option<LogFormat>,
    log_filter: Option<String>,

    general: Option<GeneralSection>,
    audio: Option<AudioSection>,
    ap1: Option<Ap1Section>,
    ap2: Option<Ap2Section>,
    observability: Option<ObservabilitySection>,
    diagnostics: Option<DiagnosticsSection>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct GeneralSection {
    name: Option<String>,
    port: Option<u16>,
    password: Option<String>,
    max_clients: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct AudioSection {
    backend: Option<AudioBackend>,
    output_format: Option<OutputSampleFormat>,
    alsa_device: Option<String>,
    alsa_period_frames: Option<u32>,
    alsa_buffer_frames: Option<u32>,
    pipe_path: Option<String>,
    raop_output_sample_rate: Option<u32>,
    raop_output_max_channels: Option<u8>,
    airplay_mode: Option<AirPlayModeConfig>,
    rsa_key_path: Option<String>,
    alsa: Option<AudioAlsaSection>,
    pipewire: Option<AudioPipewireSection>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct AudioAlsaSection {
    device: Option<String>,
    period_frames: Option<u32>,
    buffer_frames: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct AudioPipewireSection {
    application_name: Option<String>,
    node_name: Option<String>,
    sink_target: Option<String>,
    output_rate: Option<u32>,
    output_format: Option<OutputSampleFormat>,
    output_channels: Option<u8>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct Ap1Section {
    codecs: Option<Vec<Ap1CodecConfig>>,
    encryption: Option<Vec<Ap1EncryptionConfig>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct Ap2Section {
    pin: Option<String>,
    pairing_store_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ObservabilitySection {
    activity_interval_secs: Option<u64>,
    activity_snapshot_path: Option<String>,
    log_format: Option<LogFormat>,
    log_filter: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct DiagnosticsSection {
    disable_resend_requests: Option<TomlBool>,
    statistics: Option<TomlBool>,
    log_verbosity: Option<u8>,
    log_show_file_and_line: Option<TomlBool>,
    log_show_time_since_startup: Option<TomlBool>,
    log_show_time_since_last_message: Option<TomlBool>,
    drop_this_fraction_of_audio_packets: Option<f64>,
    retain_cover_art: Option<TomlBool>,
    get_plist_metadata: Option<TomlBool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum TomlBool {
    Bool(bool),
    Text(String),
}

impl TomlBool {
    fn parse(self, key: &str) -> Result<bool, String> {
        match self {
            Self::Bool(v) => Ok(v),
            Self::Text(v) => {
                if v.eq_ignore_ascii_case("yes")
                    || v.eq_ignore_ascii_case("true")
                    || v == "1"
                {
                    return Ok(true);
                }
                if v.eq_ignore_ascii_case("no")
                    || v.eq_ignore_ascii_case("false")
                    || v == "0"
                {
                    return Ok(false);
                }
                Err(format!(
                    "{key} must be yes/no or true/false when set; got '{v}'"
                ))
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
struct EnvConfig {
    name: Option<String>,
    port: Option<u16>,
    backend: Option<AudioBackend>,
    output_format: Option<OutputSampleFormat>,
    alsa_device: Option<String>,
    alsa_period_frames: Option<u32>,
    alsa_buffer_frames: Option<u32>,
    pipe_path: Option<String>,
    password: Option<String>,
    max_clients: Option<usize>,
    raop_output_sample_rate: Option<u32>,
    raop_output_max_channels: Option<u8>,
    ap1_codecs: Option<Vec<Ap1CodecConfig>>,
    ap1_encryption: Option<Vec<Ap1EncryptionConfig>>,
    airplay_mode: Option<AirPlayModeConfig>,
    ap2_pin: Option<String>,
    ap2_pairing_store_path: Option<String>,
    rsa_key_path: Option<String>,
    activity_interval_secs: Option<u64>,
    activity_snapshot_path: Option<String>,
    log_format: Option<LogFormat>,
    log_filter: Option<String>,
    diagnostics_disable_resend_requests: Option<bool>,
    diagnostics_statistics: Option<bool>,
    diagnostics_log_verbosity: Option<u8>,
    diagnostics_log_show_file_and_line: Option<bool>,
    diagnostics_log_show_time_since_startup: Option<bool>,
    diagnostics_log_show_time_since_last_message: Option<bool>,
    diagnostics_drop_this_fraction_of_audio_packets: Option<f64>,
    diagnostics_retain_cover_art: Option<bool>,
    diagnostics_get_plist_metadata: Option<bool>,
}

impl EnvConfig {
    fn from_env() -> Result<Self, String> {
        Self::from_pairs(std::env::vars())
    }

    fn from_pairs<I, K, V>(pairs: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let map: HashMap<String, String> = pairs
            .into_iter()
            .map(|(k, v)| (k.as_ref().to_string(), v.as_ref().to_string()))
            .collect();

        let mut cfg = Self::default();
        cfg.name = map.get("SSR_NAME").cloned();
        cfg.port = parse_env_number::<u16>(&map, "SSR_PORT")?;
        cfg.backend = parse_env_backend(&map)?;
        cfg.output_format = parse_env_output_format(&map)?;
        cfg.alsa_device = map.get("SSR_ALSA_DEVICE").cloned();
        cfg.alsa_period_frames = parse_env_number::<u32>(&map, "SSR_ALSA_PERIOD_FRAMES")?;
        cfg.alsa_buffer_frames = parse_env_number::<u32>(&map, "SSR_ALSA_BUFFER_FRAMES")?;
        cfg.pipe_path = map.get("SSR_PIPE_PATH").cloned();
        cfg.password = map.get("SSR_PASSWORD").cloned();
        cfg.max_clients = parse_env_number::<usize>(&map, "SSR_MAX_CLIENTS")?;
        cfg.raop_output_sample_rate = parse_env_number::<u32>(&map, "SSR_RAOP_OUTPUT_SAMPLE_RATE")?;
        cfg.raop_output_max_channels = parse_env_number::<u8>(&map, "SSR_RAOP_OUTPUT_MAX_CHANNELS")?;
        cfg.ap1_codecs = parse_env_ap1_codecs(&map)?;
        cfg.ap1_encryption = parse_env_ap1_encryption(&map)?;
        cfg.airplay_mode = parse_env_airplay_mode(&map)?;
        cfg.ap2_pin = map.get("SSR_AP2_PIN").cloned();
        cfg.ap2_pairing_store_path = map.get("SSR_AP2_PAIRING_STORE_PATH").cloned();
        cfg.rsa_key_path = map.get("SSR_RSA_KEY_PATH").cloned();
        cfg.activity_interval_secs = parse_env_number::<u64>(&map, "SSR_ACTIVITY_INTERVAL_SECS")?;
        cfg.activity_snapshot_path = map.get("SSR_ACTIVITY_SNAPSHOT_PATH").cloned();
        cfg.log_format = parse_env_log_format(&map)?;
        cfg.log_filter = map.get("SSR_LOG_FILTER").cloned();
        cfg.diagnostics_disable_resend_requests =
            parse_env_bool(&map, "SSR_DIAGNOSTICS_DISABLE_RESEND_REQUESTS")?;
        cfg.diagnostics_statistics = parse_env_bool(&map, "SSR_DIAGNOSTICS_STATISTICS")?;
        cfg.diagnostics_log_verbosity =
            parse_env_number::<u8>(&map, "SSR_DIAGNOSTICS_LOG_VERBOSITY")?;
        cfg.diagnostics_log_show_file_and_line =
            parse_env_bool(&map, "SSR_DIAGNOSTICS_LOG_SHOW_FILE_AND_LINE")?;
        cfg.diagnostics_log_show_time_since_startup =
            parse_env_bool(&map, "SSR_DIAGNOSTICS_LOG_SHOW_TIME_SINCE_STARTUP")?;
        cfg.diagnostics_log_show_time_since_last_message = parse_env_bool(
            &map,
            "SSR_DIAGNOSTICS_LOG_SHOW_TIME_SINCE_LAST_MESSAGE",
        )?;
        cfg.diagnostics_drop_this_fraction_of_audio_packets = parse_env_number::<f64>(
            &map,
            "SSR_DIAGNOSTICS_DROP_THIS_FRACTION_OF_AUDIO_PACKETS",
        )?;
        cfg.diagnostics_retain_cover_art =
            parse_env_bool(&map, "SSR_DIAGNOSTICS_RETAIN_COVER_ART")?;
        cfg.diagnostics_get_plist_metadata =
            parse_env_bool(&map, "SSR_DIAGNOSTICS_GET_PLIST_METADATA")?;
        Ok(cfg)
    }
}

fn parse_env_bool(map: &HashMap<String, String>, key: &str) -> Result<Option<bool>, String> {
    let Some(raw) = map.get(key) else {
        return Ok(None);
    };

    if raw.eq_ignore_ascii_case("yes")
        || raw.eq_ignore_ascii_case("true")
        || raw == "1"
    {
        return Ok(Some(true));
    }
    if raw.eq_ignore_ascii_case("no")
        || raw.eq_ignore_ascii_case("false")
        || raw == "0"
    {
        return Ok(Some(false));
    }

    Err(format!(
        "failed to parse environment variable {key}: expected yes/no, true/false, or 1/0, got '{raw}'"
    ))
}

fn parse_env_number<T>(map: &HashMap<String, String>, key: &str) -> Result<Option<T>, String>
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    let Some(raw) = map.get(key) else {
        return Ok(None);
    };

    raw.parse::<T>()
        .map(Some)
        .map_err(|e| format!("failed to parse environment variable {key}: {e}"))
}

fn parse_env_backend(map: &HashMap<String, String>) -> Result<Option<AudioBackend>, String> {
    let Some(raw) = map.get("SSR_BACKEND") else {
        return Ok(None);
    };

    if raw.eq_ignore_ascii_case("null") {
        return Ok(Some(AudioBackend::Null));
    }
    if raw.eq_ignore_ascii_case("stdout") {
        return Ok(Some(AudioBackend::Stdout));
    }
    if raw.eq_ignore_ascii_case("pipe") {
        return Ok(Some(AudioBackend::Pipe));
    }
    #[cfg(target_os = "linux")]
    if raw.eq_ignore_ascii_case("alsa") {
        return Ok(Some(AudioBackend::Alsa));
    }

    Err(format!(
        "failed to parse environment variable SSR_BACKEND: unsupported value '{raw}'"
    ))
}

fn parse_env_output_format(
    map: &HashMap<String, String>,
) -> Result<Option<OutputSampleFormat>, String> {
    let Some(raw) = map.get("SSR_OUTPUT_FORMAT") else {
        return Ok(None);
    };

    if raw.eq_ignore_ascii_case("f32le") {
        return Ok(Some(OutputSampleFormat::F32Le));
    }
    if raw.eq_ignore_ascii_case("s16le") {
        return Ok(Some(OutputSampleFormat::S16Le));
    }
    if raw.eq_ignore_ascii_case("s24le") {
        return Ok(Some(OutputSampleFormat::S24Le));
    }

    Err(format!(
        "failed to parse environment variable SSR_OUTPUT_FORMAT: unsupported value '{raw}'"
    ))
}

fn parse_env_ap1_codecs(
    map: &HashMap<String, String>,
) -> Result<Option<Vec<Ap1CodecConfig>>, String> {
    let Some(raw) = map.get("SSR_AP1_CODECS") else {
        return Ok(None);
    };

    let mut parsed = Vec::new();
    for token in raw.split(',') {
        let item = token.trim();
        if item.is_empty() {
            continue;
        }
        if item.eq_ignore_ascii_case("pcm") {
            parsed.push(Ap1CodecConfig::Pcm);
            continue;
        }
        if item.eq_ignore_ascii_case("alac") {
            parsed.push(Ap1CodecConfig::Alac);
            continue;
        }
        return Err(format!(
            "failed to parse environment variable SSR_AP1_CODECS: unsupported value '{item}'"
        ));
    }

    Ok(Some(parsed))
}

fn parse_env_ap1_encryption(
    map: &HashMap<String, String>,
) -> Result<Option<Vec<Ap1EncryptionConfig>>, String> {
    let Some(raw) = map.get("SSR_AP1_ENCRYPTION") else {
        return Ok(None);
    };

    let mut parsed = Vec::new();
    for token in raw.split(',') {
        let item = token.trim();
        if item.is_empty() {
            continue;
        }
        if item.eq_ignore_ascii_case("none") {
            parsed.push(Ap1EncryptionConfig::None);
            continue;
        }
        if item.eq_ignore_ascii_case("rsa") {
            parsed.push(Ap1EncryptionConfig::Rsa);
            continue;
        }
        if item.eq_ignore_ascii_case("fairplay") {
            parsed.push(Ap1EncryptionConfig::Fairplay);
            continue;
        }
        return Err(format!(
            "failed to parse environment variable SSR_AP1_ENCRYPTION: unsupported value '{item}'"
        ));
    }

    Ok(Some(parsed))
}

fn parse_env_airplay_mode(
    map: &HashMap<String, String>,
) -> Result<Option<AirPlayModeConfig>, String> {
    let Some(raw) = map.get("SSR_AIRPLAY_MODE") else {
        return Ok(None);
    };

    if raw.eq_ignore_ascii_case("ap1") {
        return Ok(Some(AirPlayModeConfig::Ap1));
    }
    if raw.eq_ignore_ascii_case("ap2") {
        return Ok(Some(AirPlayModeConfig::Ap2));
    }

    Err(format!(
        "failed to parse environment variable SSR_AIRPLAY_MODE: unsupported value '{raw}'"
    ))
}

fn parse_env_log_format(map: &HashMap<String, String>) -> Result<Option<LogFormat>, String> {
    let Some(raw) = map.get("SSR_LOG_FORMAT") else {
        return Ok(None);
    };

    if raw.eq_ignore_ascii_case("text") {
        return Ok(Some(LogFormat::Text));
    }
    if raw.eq_ignore_ascii_case("json") {
        return Ok(Some(LogFormat::Json));
    }

    Err(format!(
        "failed to parse environment variable SSR_LOG_FORMAT: unsupported value '{raw}'"
    ))
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "Shairport Sync RS".to_string(),
            port: 5000,
            backend: AudioBackend::Null,
            output_format: OutputSampleFormat::F32Le,
            alsa_device: Some("default".to_string()),
            alsa_period_frames: Some(1024),
            alsa_buffer_frames: Some(4096),
            pipe_path: Some("/tmp/shairport-sync-rs.pcm".to_string()),
            password: None,
            max_clients: 10,
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: AirPlayModeConfig::Ap2,
            ap2_pin: None,
            ap2_pairing_store_path: None,
            rsa_key_path: None,
            activity_interval_secs: 30,
            activity_snapshot_path: None,
            log_format: LogFormat::Text,
            log_filter: None,
            diagnostics: DiagnosticsConfig {
                disable_resend_requests: false,
                statistics: false,
                log_verbosity: 0,
                log_show_file_and_line: true,
                log_show_time_since_startup: false,
                log_show_time_since_last_message: true,
                drop_this_fraction_of_audio_packets: 0.0,
                retain_cover_art: false,
                get_plist_metadata: false,
            },
        }
    }
}

impl AppConfig {
    pub fn load(cli: &Cli) -> Result<Self, String> {
        let env_cfg = EnvConfig::from_env()?;
        Self::load_with_env(cli, env_cfg)
    }

    fn load_with_env(cli: &Cli, env_cfg: EnvConfig) -> Result<Self, String> {
        let mut cfg = AppConfig::default();

        if let Some(path) = &cli.config {
            let content = fs::read_to_string(path)
                .map_err(|e| format!("failed to read config file {}: {e}", path.display()))?;
            let file_cfg: FileConfig = toml::from_str(&content)
                .map_err(|e| format!("failed to parse config file {}: {e}", path.display()))?;
            cfg.apply_file(file_cfg)?;
        }

        cfg.apply_env(env_cfg);
        cfg.apply_cli(cli);
        cfg.validate()?;
        Ok(cfg)
    }

    fn apply_file(&mut self, file: FileConfig) -> Result<(), String> {
        let FileConfig {
            name,
            port,
            backend,
            output_format,
            alsa_device,
            alsa_period_frames,
            alsa_buffer_frames,
            pipe_path,
            password,
            max_clients,
            raop_output_sample_rate,
            raop_output_max_channels,
            ap1_codecs,
            ap1_encryption,
            airplay_mode,
            ap2_pin,
            ap2_pairing_store_path,
            rsa_key_path,
            activity_interval_secs,
            activity_snapshot_path,
            log_format,
            log_filter,
            general,
            audio,
            ap1,
            ap2,
            observability,
            diagnostics,
        } = file;

        if let Some(v) = name {
            self.name = v;
        }
        if let Some(v) = port {
            self.port = v;
        }
        if let Some(v) = backend {
            self.backend = v;
        }
        if let Some(v) = output_format {
            self.output_format = v;
        }
        if let Some(v) = alsa_device {
            self.alsa_device = Some(v);
        }
        if let Some(v) = alsa_period_frames {
            self.alsa_period_frames = Some(v);
        }
        if let Some(v) = alsa_buffer_frames {
            self.alsa_buffer_frames = Some(v);
        }
        if let Some(v) = pipe_path {
            self.pipe_path = Some(v);
        }
        if let Some(v) = password {
            self.password = Some(v);
        }
        if let Some(v) = max_clients {
            self.max_clients = v;
        }
        if let Some(v) = raop_output_sample_rate {
            self.raop_output_sample_rate = Some(v);
        }
        if let Some(v) = raop_output_max_channels {
            self.raop_output_max_channels = Some(v);
        }
        if let Some(v) = ap1_codecs {
            self.ap1_codecs = Some(v);
        }
        if let Some(v) = ap1_encryption {
            self.ap1_encryption = Some(v);
        }
        if let Some(v) = airplay_mode {
            self.airplay_mode = v;
        }
        if let Some(v) = ap2_pin {
            self.ap2_pin = Some(v);
        }
        if let Some(v) = ap2_pairing_store_path {
            self.ap2_pairing_store_path = Some(v);
        }
        if let Some(v) = rsa_key_path {
            self.rsa_key_path = Some(v);
        }
        if let Some(v) = activity_interval_secs {
            self.activity_interval_secs = v;
        }
        if let Some(v) = activity_snapshot_path {
            self.activity_snapshot_path = Some(v);
        }
        if let Some(v) = log_format {
            self.log_format = v;
        }
        if let Some(v) = log_filter {
            self.log_filter = Some(v);
        }

        if let Some(section) = general {
            if let Some(v) = section.name {
                self.name = v;
            }
            if let Some(v) = section.port {
                self.port = v;
            }
            if let Some(v) = section.password {
                self.password = Some(v);
            }
            if let Some(v) = section.max_clients {
                self.max_clients = v;
            }
        }

        if let Some(section) = audio {
            let AudioSection {
                backend,
                output_format,
                alsa_device,
                alsa_period_frames,
                alsa_buffer_frames,
                pipe_path,
                raop_output_sample_rate,
                raop_output_max_channels,
                airplay_mode,
                rsa_key_path,
                alsa,
                pipewire,
            } = section;

            if let Some(v) = backend {
                self.backend = v;
            }
            if let Some(v) = output_format {
                self.output_format = v;
            }
            if let Some(v) = alsa_device {
                self.alsa_device = Some(v);
            }
            if let Some(v) = alsa_period_frames {
                self.alsa_period_frames = Some(v);
            }
            if let Some(v) = alsa_buffer_frames {
                self.alsa_buffer_frames = Some(v);
            }
            if let Some(v) = pipe_path {
                self.pipe_path = Some(v);
            }
            if let Some(v) = raop_output_sample_rate {
                self.raop_output_sample_rate = Some(v);
            }
            if let Some(v) = raop_output_max_channels {
                self.raop_output_max_channels = Some(v);
            }
            if let Some(v) = airplay_mode {
                self.airplay_mode = v;
            }
            if let Some(v) = rsa_key_path {
                self.rsa_key_path = Some(v);
            }

            if let Some(alsa_section) = alsa {
                if let Some(v) = alsa_section.device {
                    self.alsa_device = Some(v);
                }
                if let Some(v) = alsa_section.period_frames {
                    self.alsa_period_frames = Some(v);
                }
                if let Some(v) = alsa_section.buffer_frames {
                    self.alsa_buffer_frames = Some(v);
                }
            }

            if let Some(pipewire_section) = pipewire {
                if let Some(v) = pipewire_section.output_format {
                    self.output_format = v;
                }
                // Keep these accepted for forward-compatible config shape while
                // PipeWire backend support is not yet wired in runtime.
                let _ = pipewire_section.application_name;
                let _ = pipewire_section.node_name;
                let _ = pipewire_section.sink_target;
                let _ = pipewire_section.output_rate;
                let _ = pipewire_section.output_channels;
            }
        }

        if let Some(section) = ap1 {
            if let Some(v) = section.codecs {
                self.ap1_codecs = Some(v);
            }
            if let Some(v) = section.encryption {
                self.ap1_encryption = Some(v);
            }
        }

        if let Some(section) = ap2 {
            if let Some(v) = section.pin {
                self.ap2_pin = Some(v);
            }
            if let Some(v) = section.pairing_store_path {
                self.ap2_pairing_store_path = Some(v);
            }
        }

        if let Some(section) = observability {
            if let Some(v) = section.activity_interval_secs {
                self.activity_interval_secs = v;
            }
            if let Some(v) = section.activity_snapshot_path {
                self.activity_snapshot_path = Some(v);
            }
            if let Some(v) = section.log_format {
                self.log_format = v;
            }
            if let Some(v) = section.log_filter {
                self.log_filter = Some(v);
            }
        }

        if let Some(section) = diagnostics {
            if let Some(v) = section.disable_resend_requests {
                self.diagnostics.disable_resend_requests =
                    v.parse("diagnostics.disable_resend_requests")?;
            }
            if let Some(v) = section.statistics {
                self.diagnostics.statistics = v.parse("diagnostics.statistics")?;
            }
            if let Some(v) = section.log_verbosity {
                self.diagnostics.log_verbosity = v;
            }
            if let Some(v) = section.log_show_file_and_line {
                self.diagnostics.log_show_file_and_line =
                    v.parse("diagnostics.log_show_file_and_line")?;
            }
            if let Some(v) = section.log_show_time_since_startup {
                self.diagnostics.log_show_time_since_startup =
                    v.parse("diagnostics.log_show_time_since_startup")?;
            }
            if let Some(v) = section.log_show_time_since_last_message {
                self.diagnostics.log_show_time_since_last_message =
                    v.parse("diagnostics.log_show_time_since_last_message")?;
            }
            if let Some(v) = section.drop_this_fraction_of_audio_packets {
                self.diagnostics.drop_this_fraction_of_audio_packets = v;
            }
            if let Some(v) = section.retain_cover_art {
                self.diagnostics.retain_cover_art = v.parse("diagnostics.retain_cover_art")?;
            }
            if let Some(v) = section.get_plist_metadata {
                self.diagnostics.get_plist_metadata = v.parse("diagnostics.get_plist_metadata")?;
            }
        }

        Ok(())
    }

    fn apply_cli(&mut self, cli: &Cli) {
        if let Some(v) = &cli.name {
            self.name = v.clone();
        }
        if let Some(v) = cli.port {
            self.port = v;
        }
        if let Some(v) = cli.backend {
            self.backend = v;
        }
        if let Some(v) = cli.output_format {
            self.output_format = v;
        }
        if let Some(v) = &cli.alsa_device {
            self.alsa_device = Some(v.clone());
        }
        if let Some(v) = cli.alsa_period_frames {
            self.alsa_period_frames = Some(v);
        }
        if let Some(v) = cli.alsa_buffer_frames {
            self.alsa_buffer_frames = Some(v);
        }
        if let Some(v) = &cli.pipe_path {
            self.pipe_path = Some(v.clone());
        }
        if let Some(v) = &cli.password {
            self.password = Some(v.clone());
        }
        if let Some(v) = cli.max_clients {
            self.max_clients = v;
        }
        if let Some(v) = cli.raop_output_sample_rate {
            self.raop_output_sample_rate = Some(v);
        }
        if let Some(v) = cli.raop_output_max_channels {
            self.raop_output_max_channels = Some(v);
        }
        if let Some(v) = &cli.ap1_codecs {
            self.ap1_codecs = Some(v.clone());
        }
        if let Some(v) = &cli.ap1_encryption {
            self.ap1_encryption = Some(v.clone());
        }
        if let Some(v) = cli.airplay_mode {
            self.airplay_mode = v;
        }
        if let Some(v) = &cli.ap2_pin {
            self.ap2_pin = Some(v.clone());
        }
        if let Some(v) = &cli.ap2_pairing_store_path {
            self.ap2_pairing_store_path = Some(v.clone());
        }
        if let Some(v) = &cli.rsa_key_path {
            self.rsa_key_path = Some(v.clone());
        }
        if let Some(v) = cli.activity_interval_secs {
            self.activity_interval_secs = v;
        }
        if let Some(v) = &cli.activity_snapshot_path {
            self.activity_snapshot_path = Some(v.clone());
        }
        if let Some(v) = cli.log_format {
            self.log_format = v;
        }
        if let Some(v) = &cli.log_filter {
            self.log_filter = Some(v.clone());
        }
    }

    fn apply_env(&mut self, env: EnvConfig) {
        if let Some(v) = env.name {
            self.name = v;
        }
        if let Some(v) = env.port {
            self.port = v;
        }
        if let Some(v) = env.backend {
            self.backend = v;
        }
        if let Some(v) = env.output_format {
            self.output_format = v;
        }
        if let Some(v) = env.alsa_device {
            self.alsa_device = Some(v);
        }
        if let Some(v) = env.alsa_period_frames {
            self.alsa_period_frames = Some(v);
        }
        if let Some(v) = env.alsa_buffer_frames {
            self.alsa_buffer_frames = Some(v);
        }
        if let Some(v) = env.pipe_path {
            self.pipe_path = Some(v);
        }
        if let Some(v) = env.password {
            self.password = Some(v);
        }
        if let Some(v) = env.max_clients {
            self.max_clients = v;
        }
        if let Some(v) = env.raop_output_sample_rate {
            self.raop_output_sample_rate = Some(v);
        }
        if let Some(v) = env.raop_output_max_channels {
            self.raop_output_max_channels = Some(v);
        }
        if let Some(v) = env.ap1_codecs {
            self.ap1_codecs = Some(v);
        }
        if let Some(v) = env.ap1_encryption {
            self.ap1_encryption = Some(v);
        }
        if let Some(v) = env.airplay_mode {
            self.airplay_mode = v;
        }
        if let Some(v) = env.ap2_pin {
            self.ap2_pin = Some(v);
        }
        if let Some(v) = env.ap2_pairing_store_path {
            self.ap2_pairing_store_path = Some(v);
        }
        if let Some(v) = env.rsa_key_path {
            self.rsa_key_path = Some(v);
        }
        if let Some(v) = env.activity_interval_secs {
            self.activity_interval_secs = v;
        }
        if let Some(v) = env.activity_snapshot_path {
            self.activity_snapshot_path = Some(v);
        }
        if let Some(v) = env.log_format {
            self.log_format = v;
        }
        if let Some(v) = env.log_filter {
            self.log_filter = Some(v);
        }
        if let Some(v) = env.diagnostics_disable_resend_requests {
            self.diagnostics.disable_resend_requests = v;
        }
        if let Some(v) = env.diagnostics_statistics {
            self.diagnostics.statistics = v;
        }
        if let Some(v) = env.diagnostics_log_verbosity {
            self.diagnostics.log_verbosity = v;
        }
        if let Some(v) = env.diagnostics_log_show_file_and_line {
            self.diagnostics.log_show_file_and_line = v;
        }
        if let Some(v) = env.diagnostics_log_show_time_since_startup {
            self.diagnostics.log_show_time_since_startup = v;
        }
        if let Some(v) = env.diagnostics_log_show_time_since_last_message {
            self.diagnostics.log_show_time_since_last_message = v;
        }
        if let Some(v) = env.diagnostics_drop_this_fraction_of_audio_packets {
            self.diagnostics.drop_this_fraction_of_audio_packets = v;
        }
        if let Some(v) = env.diagnostics_retain_cover_art {
            self.diagnostics.retain_cover_art = v;
        }
        if let Some(v) = env.diagnostics_get_plist_metadata {
            self.diagnostics.get_plist_metadata = v;
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("name must not be empty".to_string());
        }
        if self.port == 0 {
            return Err("port must be greater than 0".to_string());
        }
        if self.max_clients == 0 {
            return Err("max_clients must be greater than 0".to_string());
        }
        if self.activity_interval_secs == 0 {
            return Err("activity_interval_secs must be greater than 0".to_string());
        }
        if matches!(self.raop_output_sample_rate, Some(0)) {
            return Err("raop_output_sample_rate must be greater than 0".to_string());
        }
        if matches!(self.raop_output_max_channels, Some(0)) {
            return Err("raop_output_max_channels must be greater than 0".to_string());
        }
        if let Some(v) = &self.ap1_codecs {
            if v.is_empty() {
                return Err("ap1_codecs must not be empty when set".to_string());
            }
        }
        if let Some(v) = &self.ap1_encryption {
            if v.is_empty() {
                return Err("ap1_encryption must not be empty when set".to_string());
            }
        }
        if self.airplay_mode == AirPlayModeConfig::Ap2
            && (self.ap1_codecs.is_some() || self.ap1_encryption.is_some())
        {
            return Err("ap1_codecs/ap1_encryption require airplay_mode=ap1".to_string());
        }
        if let Some(pin) = self.ap2_pin.as_deref() {
            if pin.trim().is_empty() {
                return Err("ap2_pin must not be empty when set".to_string());
            }
            if self.airplay_mode != AirPlayModeConfig::Ap2 {
                return Err("ap2_pin requires airplay_mode=ap2".to_string());
            }
        }
        if let Some(path) = self.ap2_pairing_store_path.as_deref() {
            if path.trim().is_empty() {
                return Err("ap2_pairing_store_path must not be empty when set".to_string());
            }
            if self.airplay_mode != AirPlayModeConfig::Ap2 {
                return Err("ap2_pairing_store_path requires airplay_mode=ap2".to_string());
            }
        }
        if let Some(path) = self.rsa_key_path.as_deref()
            && path.trim().is_empty()
        {
            return Err("rsa_key_path must not be empty when set".to_string());
        }
        #[cfg(target_os = "linux")]
        {
            if matches!(self.alsa_period_frames, Some(0)) {
                return Err("alsa_period_frames must be greater than 0 when set".to_string());
            }

            if matches!(self.alsa_buffer_frames, Some(0)) {
                return Err("alsa_buffer_frames must be greater than 0 when set".to_string());
            }

            if let (Some(period), Some(buffer)) = (self.alsa_period_frames, self.alsa_buffer_frames)
                && buffer < period
            {
                return Err(
                    "alsa_buffer_frames must be greater than or equal to alsa_period_frames"
                        .to_string(),
                );
            }

            if matches!(self.backend, AudioBackend::Alsa)
                && self
                    .alsa_device
                    .as_deref()
                    .is_none_or(|v| v.trim().is_empty())
            {
                return Err("alsa_device must not be empty when backend is alsa".to_string());
            }

            if matches!(self.backend, AudioBackend::Alsa)
                && self.output_format != OutputSampleFormat::F32Le
            {
                return Err("backend alsa currently supports only output_format=f32le".to_string());
            }
        }
        if matches!(self.backend, AudioBackend::Pipe)
            && self
                .pipe_path
                .as_deref()
                .is_none_or(|v| v.trim().is_empty())
        {
            return Err("pipe_path must not be empty when backend is pipe".to_string());
        }
        if self
            .activity_snapshot_path
            .as_deref()
            .is_some_and(|v| v.trim().is_empty())
        {
            return Err("activity_snapshot_path must not be empty when set".to_string());
        }
        if self
            .log_filter
            .as_deref()
            .is_some_and(|v| v.trim().is_empty())
        {
            return Err("log_filter must not be empty when set".to_string());
        }
        if self.diagnostics.log_verbosity > 3 {
            return Err("diagnostics.log_verbosity must be between 0 and 3".to_string());
        }
        if !(0.0..=1.0).contains(&self.diagnostics.drop_this_fraction_of_audio_packets) {
            return Err(
                "diagnostics.drop_this_fraction_of_audio_packets must be between 0.0 and 1.0"
                    .to_string(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn valid_base() -> AppConfig {
        AppConfig {
            name: "test".to_string(),
            port: 5000,
            backend: AudioBackend::Null,
            output_format: OutputSampleFormat::F32Le,
            alsa_device: Some("default".to_string()),
            alsa_period_frames: Some(1024),
            alsa_buffer_frames: Some(4096),
            pipe_path: Some("/tmp/shairport-sync-rs-test.pcm".to_string()),
            password: None,
            max_clients: 1,
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: AirPlayModeConfig::Ap2,
            ap2_pin: None,
            ap2_pairing_store_path: None,
            rsa_key_path: None,
            activity_interval_secs: 30,
            activity_snapshot_path: None,
            log_format: LogFormat::Text,
            log_filter: None,
            diagnostics: DiagnosticsConfig {
                disable_resend_requests: false,
                statistics: false,
                log_verbosity: 0,
                log_show_file_and_line: true,
                log_show_time_since_startup: false,
                log_show_time_since_last_message: true,
                drop_this_fraction_of_audio_packets: 0.0,
                retain_cover_art: false,
                get_plist_metadata: false,
            },
        }
    }

    fn unique_temp_file(prefix: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock moved backwards")
            .as_nanos();
        let pid = std::process::id();
        std::env::temp_dir().join(format!("shairport-sync-rs-{prefix}-{pid}-{ts}.toml"))
    }

    fn parse_cli(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("CLI parsing should succeed")
    }

    #[test]
    fn validate_accepts_pipe_with_s24le() {
        let mut cfg = valid_base();
        cfg.backend = AudioBackend::Pipe;
        cfg.output_format = OutputSampleFormat::S24Le;

        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn validate_rejects_pipe_with_empty_pipe_path() {
        let mut cfg = valid_base();
        cfg.backend = AudioBackend::Pipe;
        cfg.pipe_path = Some("   ".to_string());

        let err = cfg
            .validate()
            .expect_err("pipe backend should require non-empty pipe_path");
        assert!(err.contains("pipe_path"));
    }

    #[test]
    fn validate_rejects_empty_name() {
        let mut cfg = valid_base();
        cfg.name = "".to_string();

        let err = cfg
            .validate()
            .expect_err("empty name should fail validation");
        assert!(err.contains("name"));
    }

    #[test]
    fn validate_rejects_zero_max_clients() {
        let mut cfg = valid_base();
        cfg.max_clients = 0;

        let err = cfg
            .validate()
            .expect_err("max_clients=0 should fail validation");
        assert!(err.contains("max_clients"));
    }

    #[test]
    fn validate_rejects_zero_raop_output_sample_rate() {
        let mut cfg = valid_base();
        cfg.raop_output_sample_rate = Some(0);

        let err = cfg
            .validate()
            .expect_err("raop_output_sample_rate=0 should fail validation");
        assert!(err.contains("raop_output_sample_rate"));
    }

    #[test]
    fn validate_rejects_zero_raop_output_max_channels() {
        let mut cfg = valid_base();
        cfg.raop_output_max_channels = Some(0);

        let err = cfg
            .validate()
            .expect_err("raop_output_max_channels=0 should fail validation");
        assert!(err.contains("raop_output_max_channels"));
    }

    #[test]
    fn validate_rejects_invalid_diagnostics_drop_fraction() {
        let mut cfg = valid_base();
        cfg.diagnostics.drop_this_fraction_of_audio_packets = 1.2;

        let err = cfg
            .validate()
            .expect_err("drop_this_fraction_of_audio_packets > 1.0 should fail validation");
        assert!(err.contains("drop_this_fraction_of_audio_packets"));
    }

    #[test]
    fn validate_rejects_ap1_advertisement_in_ap2_mode() {
        let mut cfg = valid_base();
        cfg.airplay_mode = AirPlayModeConfig::Ap2;
        cfg.ap1_codecs = Some(vec![Ap1CodecConfig::Pcm]);

        let err = cfg
            .validate()
            .expect_err("ap1 advertisement should require ap1 mode");
        assert!(err.contains("airplay_mode=ap1"));
    }

    #[test]
    fn validate_rejects_empty_ap2_pin() {
        let mut cfg = valid_base();
        cfg.ap2_pin = Some("   ".to_string());

        let err = cfg
            .validate()
            .expect_err("blank ap2_pin should fail validation");
        assert!(err.contains("ap2_pin"));
    }

    #[test]
    fn validate_rejects_ap2_pin_with_ap1_mode() {
        let mut cfg = valid_base();
        cfg.airplay_mode = AirPlayModeConfig::Ap1;
        cfg.ap2_pin = Some("12345678".to_string());

        let err = cfg.validate().expect_err("ap2_pin should require ap2 mode");
        assert!(err.contains("airplay_mode=ap2"));
    }

    #[test]
    fn validate_rejects_empty_ap2_pairing_store_path() {
        let mut cfg = valid_base();
        cfg.ap2_pairing_store_path = Some("   ".to_string());

        let err = cfg
            .validate()
            .expect_err("blank ap2_pairing_store_path should fail validation");
        assert!(err.contains("ap2_pairing_store_path"));
    }

    #[test]
    fn validate_rejects_ap2_pairing_store_path_with_ap1_mode() {
        let mut cfg = valid_base();
        cfg.airplay_mode = AirPlayModeConfig::Ap1;
        cfg.ap2_pairing_store_path = Some("/tmp/ap2-pairings.json".to_string());

        let err = cfg
            .validate()
            .expect_err("ap2_pairing_store_path should require ap2 mode");
        assert!(err.contains("airplay_mode=ap2"));
    }

    #[test]
    fn validate_rejects_empty_rsa_key_path() {
        let mut cfg = valid_base();
        cfg.rsa_key_path = Some("   ".to_string());

        let err = cfg
            .validate()
            .expect_err("blank rsa_key_path should fail validation");
        assert!(err.contains("rsa_key_path"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn validate_rejects_alsa_with_non_f32le_output() {
        let mut cfg = valid_base();
        cfg.backend = AudioBackend::Alsa;
        cfg.output_format = OutputSampleFormat::S16Le;

        let err = cfg
            .validate()
            .expect_err("alsa backend should reject non-f32le output format");
        assert!(err.contains("output_format"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn validate_rejects_zero_alsa_period_frames() {
        let mut cfg = valid_base();
        cfg.backend = AudioBackend::Alsa;
        cfg.alsa_period_frames = Some(0);

        let err = cfg
            .validate()
            .expect_err("alsa_period_frames=0 should fail validation");
        assert!(err.contains("alsa_period_frames"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn validate_rejects_zero_alsa_buffer_frames() {
        let mut cfg = valid_base();
        cfg.backend = AudioBackend::Alsa;
        cfg.alsa_buffer_frames = Some(0);

        let err = cfg
            .validate()
            .expect_err("alsa_buffer_frames=0 should fail validation");
        assert!(err.contains("alsa_buffer_frames"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn validate_rejects_alsa_buffer_smaller_than_period() {
        let mut cfg = valid_base();
        cfg.backend = AudioBackend::Alsa;
        cfg.alsa_period_frames = Some(2048);
        cfg.alsa_buffer_frames = Some(1024);

        let err = cfg
            .validate()
            .expect_err("alsa_buffer_frames < alsa_period_frames should fail validation");
        assert!(err.contains("alsa_buffer_frames"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn validate_rejects_alsa_with_empty_device() {
        let mut cfg = valid_base();
        cfg.backend = AudioBackend::Alsa;
        cfg.alsa_device = Some("   ".to_string());

        let err = cfg
            .validate()
            .expect_err("alsa backend should require non-empty alsa_device");
        assert!(err.contains("alsa_device"));
    }

    #[test]
    fn load_merges_file_and_cli_with_cli_precedence() {
        let path = unique_temp_file("config-load-precedence");
        let toml = r#"
name = "From File"
port = 6000
backend = "pipe"
output_format = "s24le"
pipe_path = "/tmp/from-file.pcm"
alsa_period_frames = 1536
alsa_buffer_frames = 6144
password = "file-pass"
max_clients = 2
raop_output_sample_rate = 44100
raop_output_max_channels = 2
ap1_codecs = ["pcm", "alac"]
ap1_encryption = ["none", "rsa"]
airplay_mode = "ap1"
rsa_key_path = "/tmp/from-file-airport.key"
"#;
        fs::write(&path, toml).expect("failed to write temp config");

        let cli = Cli {
            config: Some(path.clone()),
            name: Some("From CLI".to_string()),
            port: Some(7000),
            backend: None,
            output_format: Some(OutputSampleFormat::S16Le),
            alsa_device: None,
            alsa_period_frames: Some(960),
            alsa_buffer_frames: Some(3840),
            pipe_path: Some("/tmp/from-cli.pcm".to_string()),
            password: Some("cli-pass".to_string()),
            max_clients: Some(4),
            raop_output_sample_rate: Some(48000),
            raop_output_max_channels: Some(1),
            ap1_codecs: Some(vec![Ap1CodecConfig::Alac]),
            ap1_encryption: Some(vec![Ap1EncryptionConfig::Rsa]),
            airplay_mode: Some(AirPlayModeConfig::Ap1),
            ap2_pin: None,
            ap2_pairing_store_path: None,
            rsa_key_path: Some("/tmp/from-cli-airport.key".to_string()),
            activity_interval_secs: Some(12),
            activity_snapshot_path: Some("/tmp/activity-cli.jsonl".to_string()),
            log_format: Some(LogFormat::Json),
            log_filter: None,
        };

        let loaded = AppConfig::load_with_env(&cli, EnvConfig::default())
            .expect("config load should succeed");

        assert_eq!(loaded.name, "From CLI");
        assert_eq!(loaded.port, 7000);
        assert!(matches!(loaded.backend, AudioBackend::Pipe));
        assert_eq!(loaded.output_format, OutputSampleFormat::S16Le);
        assert_eq!(loaded.alsa_period_frames, Some(960));
        assert_eq!(loaded.alsa_buffer_frames, Some(3840));
        assert_eq!(loaded.pipe_path.as_deref(), Some("/tmp/from-cli.pcm"));
        assert_eq!(loaded.password.as_deref(), Some("cli-pass"));
        assert_eq!(loaded.max_clients, 4);
        assert_eq!(loaded.raop_output_sample_rate, Some(48000));
        assert_eq!(loaded.raop_output_max_channels, Some(1));
        assert_eq!(loaded.ap1_codecs, Some(vec![Ap1CodecConfig::Alac]));
        assert_eq!(loaded.ap1_encryption, Some(vec![Ap1EncryptionConfig::Rsa]));
        assert_eq!(loaded.airplay_mode, AirPlayModeConfig::Ap1);
        assert!(loaded.ap2_pin.is_none());
        assert!(loaded.ap2_pairing_store_path.is_none());
        assert_eq!(loaded.rsa_key_path.as_deref(), Some("/tmp/from-cli-airport.key"));
        assert_eq!(loaded.activity_interval_secs, 12);
        assert_eq!(
            loaded.activity_snapshot_path.as_deref(),
            Some("/tmp/activity-cli.jsonl")
        );
        assert_eq!(loaded.log_format, LogFormat::Json);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_rejects_missing_config_file() {
        let cli = Cli {
            config: Some(PathBuf::from(
                "/definitely/not/present/shairport-sync-rs.toml",
            )),
            name: None,
            port: None,
            backend: None,
            output_format: None,
            alsa_device: None,
            alsa_period_frames: None,
            alsa_buffer_frames: None,
            pipe_path: None,
            password: None,
            max_clients: None,
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: None,
            ap2_pin: None,
            ap2_pairing_store_path: None,
            rsa_key_path: None,
            activity_interval_secs: None,
            activity_snapshot_path: None,
            log_format: None,
            log_filter: None,
        };

        let err = AppConfig::load_with_env(&cli, EnvConfig::default())
            .expect_err("missing config file should fail");
        assert!(err.contains("failed to read config file"));
    }

    #[test]
    fn load_rejects_invalid_toml() {
        let path = unique_temp_file("config-load-invalid");
        fs::write(&path, "name = [").expect("failed to write invalid temp config");

        let cli = Cli {
            config: Some(path.clone()),
            name: None,
            port: None,
            backend: None,
            output_format: None,
            alsa_device: None,
            alsa_period_frames: None,
            alsa_buffer_frames: None,
            pipe_path: None,
            password: None,
            max_clients: None,
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: None,
            ap2_pin: None,
            ap2_pairing_store_path: None,
            rsa_key_path: None,
            activity_interval_secs: None,
            activity_snapshot_path: None,
            log_format: None,
            log_filter: None,
        };

        let err = AppConfig::load_with_env(&cli, EnvConfig::default())
            .expect_err("invalid config should fail");
        assert!(err.contains("failed to parse config file"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn cli_parse_backend_selection_valid_cases_table_driven() {
        struct ValidCase {
            name: &'static str,
            args: Vec<&'static str>,
            expected_backend: AudioBackend,
            expected_format: OutputSampleFormat,
            expected_pipe_path: Option<&'static str>,
        }

        let cases = vec![
            ValidCase {
                name: "pipe with s24le and explicit path",
                args: vec![
                    "shairport-sync-rs",
                    "--backend",
                    "pipe",
                    "--output-format",
                    "s24le",
                    "--pipe-path",
                    "/tmp/cli-pipe.pcm",
                ],
                expected_backend: AudioBackend::Pipe,
                expected_format: OutputSampleFormat::S24Le,
                expected_pipe_path: Some("/tmp/cli-pipe.pcm"),
            },
            ValidCase {
                name: "stdout with s16le",
                args: vec![
                    "shairport-sync-rs",
                    "--backend",
                    "stdout",
                    "--output-format",
                    "s16le",
                ],
                expected_backend: AudioBackend::Stdout,
                expected_format: OutputSampleFormat::S16Le,
                expected_pipe_path: None,
            },
        ];

        for case in cases {
            let cli = parse_cli(&case.args);
            let loaded = AppConfig::load(&cli)
                .unwrap_or_else(|e| panic!("case '{}' failed: {e}", case.name));

            assert!(
                std::mem::discriminant(&loaded.backend)
                    == std::mem::discriminant(&case.expected_backend),
                "unexpected backend for case '{}'",
                case.name
            );
            assert_eq!(
                loaded.output_format, case.expected_format,
                "unexpected output format for case '{}'",
                case.name
            );

            if let Some(expected_path) = case.expected_pipe_path {
                assert_eq!(
                    loaded.pipe_path.as_deref(),
                    Some(expected_path),
                    "unexpected pipe path for case '{}'",
                    case.name
                );
            }
        }
    }

    #[test]
    fn cli_parse_backend_selection_invalid_cases_table_driven() {
        struct InvalidCase {
            name: &'static str,
            args: Vec<&'static str>,
            expected_error_substring: &'static str,
        }

        let mut cases = vec![InvalidCase {
            name: "pipe with blank path",
            args: vec![
                "shairport-sync-rs",
                "--backend",
                "pipe",
                "--pipe-path",
                "   ",
            ],
            expected_error_substring: "pipe_path",
        }];

        #[cfg(target_os = "linux")]
        cases.push(InvalidCase {
            name: "alsa with non-f32le output",
            args: vec![
                "shairport-sync-rs",
                "--backend",
                "alsa",
                "--output-format",
                "s16le",
            ],
            expected_error_substring: "output_format",
        });

        for case in cases {
            let cli = parse_cli(&case.args);
            let err =
                AppConfig::load(&cli).expect_err("invalid case should fail config validation");
            assert!(
                err.contains(case.expected_error_substring),
                "unexpected error for case '{}': {err}",
                case.name
            );
        }
    }

    #[test]
    fn cli_parse_ap1_and_raop_protocol_options() {
        let cli = parse_cli(&[
            "shairport-sync-rs",
            "--airplay-mode",
            "ap1",
            "--raop-output-sample-rate",
            "48000",
            "--raop-output-max-channels",
            "1",
            "--ap1-codecs",
            "pcm,alac",
            "--ap1-encryption",
            "none,rsa",
        ]);

        let loaded = AppConfig::load(&cli).expect("config load should succeed");
        assert_eq!(loaded.raop_output_sample_rate, Some(48_000));
        assert_eq!(loaded.raop_output_max_channels, Some(1));
        assert_eq!(
            loaded.ap1_codecs,
            Some(vec![Ap1CodecConfig::Pcm, Ap1CodecConfig::Alac])
        );
        assert_eq!(
            loaded.ap1_encryption,
            Some(vec![Ap1EncryptionConfig::None, Ap1EncryptionConfig::Rsa])
        );
        assert_eq!(loaded.airplay_mode, AirPlayModeConfig::Ap1);
    }

    #[test]
    fn cli_parse_ap2_mode_with_pin() {
        let cli = parse_cli(&[
            "shairport-sync-rs",
            "--airplay-mode",
            "ap2",
            "--ap2-pin",
            "12345678",
            "--ap2-pairing-store-path",
            "/tmp/ap2-pairings.json",
        ]);

        let loaded = AppConfig::load(&cli).expect("config load should succeed");
        assert_eq!(loaded.airplay_mode, AirPlayModeConfig::Ap2);
        assert_eq!(loaded.ap2_pin.as_deref(), Some("12345678"));
        assert_eq!(
            loaded.ap2_pairing_store_path.as_deref(),
            Some("/tmp/ap2-pairings.json")
        );
    }

    #[test]
    fn m0_minimal_config_surface_loads_name_port_mode_and_output_selection() {
        let cli = parse_cli(&[
            "shairport-sync-rs",
            "--name",
            "M0 Unit",
            "--port",
            "7001",
            "--airplay-mode",
            "ap1",
            "--backend",
            "stdout",
        ]);

        let loaded = AppConfig::load(&cli).expect("M0 minimal config should load");
        assert_eq!(loaded.name, "M0 Unit");
        assert_eq!(loaded.port, 7001);
        assert_eq!(loaded.airplay_mode, AirPlayModeConfig::Ap1);
        assert!(matches!(loaded.backend, AudioBackend::Stdout));
    }

    #[test]
    fn load_supports_sectioned_toml_layout() {
        let path = unique_temp_file("config-load-sectioned-layout");
        let toml = r#"
[general]
name = "Sectioned Config"
port = 6600
max_clients = 3

[audio]
backend = "pipe"
output_format = "s24le"
pipe_path = "/tmp/sectioned.pcm"
airplay_mode = "ap1"
rsa_key_path = "/tmp/sectioned-airport.key"
raop_output_sample_rate = 48000
raop_output_max_channels = 2

[audio.alsa]
device = "default"
period_frames = 2048
buffer_frames = 8192

[ap1]
codecs = ["pcm", "alac"]
encryption = ["none", "rsa"]

[observability]
activity_interval_secs = 9
activity_snapshot_path = "/tmp/sectioned-activity.jsonl"
log_format = "json"
"#;
        fs::write(&path, toml).expect("failed to write sectioned temp config");

        let cli = Cli {
            config: Some(path.clone()),
            name: None,
            port: None,
            backend: None,
            output_format: None,
            alsa_device: None,
            alsa_period_frames: None,
            alsa_buffer_frames: None,
            pipe_path: None,
            password: None,
            max_clients: None,
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: None,
            ap2_pin: None,
            ap2_pairing_store_path: None,
            rsa_key_path: None,
            activity_interval_secs: None,
            activity_snapshot_path: None,
            log_format: None,
            log_filter: None,
        };

        let loaded = AppConfig::load_with_env(&cli, EnvConfig::default())
            .expect("sectioned config should load");

        assert_eq!(loaded.name, "Sectioned Config");
        assert_eq!(loaded.port, 6600);
        assert_eq!(loaded.max_clients, 3);
        assert!(matches!(loaded.backend, AudioBackend::Pipe));
        assert_eq!(loaded.output_format, OutputSampleFormat::S24Le);
        assert_eq!(loaded.pipe_path.as_deref(), Some("/tmp/sectioned.pcm"));
        assert_eq!(loaded.alsa_device.as_deref(), Some("default"));
        assert_eq!(loaded.alsa_period_frames, Some(2048));
        assert_eq!(loaded.alsa_buffer_frames, Some(8192));
        assert_eq!(loaded.airplay_mode, AirPlayModeConfig::Ap1);
        assert_eq!(loaded.rsa_key_path.as_deref(), Some("/tmp/sectioned-airport.key"));
        assert_eq!(loaded.raop_output_sample_rate, Some(48_000));
        assert_eq!(loaded.raop_output_max_channels, Some(2));
        assert_eq!(
            loaded.ap1_codecs,
            Some(vec![Ap1CodecConfig::Pcm, Ap1CodecConfig::Alac])
        );
        assert_eq!(
            loaded.ap1_encryption,
            Some(vec![Ap1EncryptionConfig::None, Ap1EncryptionConfig::Rsa])
        );
        assert_eq!(loaded.activity_interval_secs, 9);
        assert_eq!(
            loaded.activity_snapshot_path.as_deref(),
            Some("/tmp/sectioned-activity.jsonl")
        );
        assert_eq!(loaded.log_format, LogFormat::Json);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_supports_diagnostics_section() {
        let path = unique_temp_file("config-load-diagnostics-section");
        let toml = r#"
[diagnostics]
statistics = "yes"
log_verbosity = 2
log_show_file_and_line = "no"
log_show_time_since_startup = "yes"
log_show_time_since_last_message = "no"
disable_resend_requests = "yes"
drop_this_fraction_of_audio_packets = 0.25
retain_cover_art = "yes"
get_plist_metadata = "yes"
"#;
        fs::write(&path, toml).expect("failed to write diagnostics temp config");

        let cli = Cli {
            config: Some(path.clone()),
            name: None,
            port: None,
            backend: None,
            output_format: None,
            alsa_device: None,
            alsa_period_frames: None,
            alsa_buffer_frames: None,
            pipe_path: None,
            password: None,
            max_clients: None,
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: None,
            ap2_pin: None,
            ap2_pairing_store_path: None,
            rsa_key_path: None,
            activity_interval_secs: None,
            activity_snapshot_path: None,
            log_format: None,
            log_filter: None,
        };

        let loaded = AppConfig::load_with_env(&cli, EnvConfig::default())
            .expect("diagnostics section should load");

        assert!(loaded.diagnostics.statistics);
        assert_eq!(loaded.diagnostics.log_verbosity, 2);
        assert!(!loaded.diagnostics.log_show_file_and_line);
        assert!(loaded.diagnostics.log_show_time_since_startup);
        assert!(!loaded.diagnostics.log_show_time_since_last_message);
        assert!(loaded.diagnostics.disable_resend_requests);
        assert!((loaded.diagnostics.drop_this_fraction_of_audio_packets - 0.25).abs() < 1e-9);
        assert!(loaded.diagnostics.retain_cover_art);
        assert!(loaded.diagnostics.get_plist_metadata);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_supports_diagnostics_environment_overrides() {
        let path = unique_temp_file("config-load-diagnostics-env-overrides");
        let toml = r#"
[diagnostics]
statistics = "no"
log_verbosity = 1
log_show_file_and_line = "yes"
log_show_time_since_startup = "no"
log_show_time_since_last_message = "yes"
disable_resend_requests = "no"
drop_this_fraction_of_audio_packets = 0.1
retain_cover_art = "no"
get_plist_metadata = "no"
"#;
        fs::write(&path, toml).expect("failed to write diagnostics temp config");

        let cli = Cli {
            config: Some(path.clone()),
            name: None,
            port: None,
            backend: None,
            output_format: None,
            alsa_device: None,
            alsa_period_frames: None,
            alsa_buffer_frames: None,
            pipe_path: None,
            password: None,
            max_clients: None,
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: None,
            ap2_pin: None,
            ap2_pairing_store_path: None,
            rsa_key_path: None,
            activity_interval_secs: None,
            activity_snapshot_path: None,
            log_format: None,
            log_filter: None,
        };

        let env_cfg = EnvConfig::from_pairs(vec![
            ("SSR_DIAGNOSTICS_STATISTICS", "yes"),
            ("SSR_DIAGNOSTICS_LOG_VERBOSITY", "3"),
            ("SSR_DIAGNOSTICS_LOG_SHOW_FILE_AND_LINE", "no"),
            ("SSR_DIAGNOSTICS_LOG_SHOW_TIME_SINCE_STARTUP", "yes"),
            ("SSR_DIAGNOSTICS_LOG_SHOW_TIME_SINCE_LAST_MESSAGE", "no"),
            ("SSR_DIAGNOSTICS_DISABLE_RESEND_REQUESTS", "yes"),
            (
                "SSR_DIAGNOSTICS_DROP_THIS_FRACTION_OF_AUDIO_PACKETS",
                "0.25",
            ),
            ("SSR_DIAGNOSTICS_RETAIN_COVER_ART", "yes"),
            ("SSR_DIAGNOSTICS_GET_PLIST_METADATA", "yes"),
        ])
        .expect("env parse should succeed");

        let loaded = AppConfig::load_with_env(&cli, env_cfg)
            .expect("diagnostics env overrides should load");

        assert!(loaded.diagnostics.statistics);
        assert_eq!(loaded.diagnostics.log_verbosity, 3);
        assert!(!loaded.diagnostics.log_show_file_and_line);
        assert!(loaded.diagnostics.log_show_time_since_startup);
        assert!(!loaded.diagnostics.log_show_time_since_last_message);
        assert!(loaded.diagnostics.disable_resend_requests);
        assert!((loaded.diagnostics.drop_this_fraction_of_audio_packets - 0.25).abs() < 1e-9);
        assert!(loaded.diagnostics.retain_cover_art);
        assert!(loaded.diagnostics.get_plist_metadata);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_rejects_invalid_diagnostics_environment_value() {
        let err = EnvConfig::from_pairs(vec![
            ("SSR_DIAGNOSTICS_LOG_SHOW_FILE_AND_LINE", "maybe"),
        ])
        .expect_err("invalid diagnostics env bool should fail parsing");
        assert!(err.contains("SSR_DIAGNOSTICS_LOG_SHOW_FILE_AND_LINE"));

        let err = EnvConfig::from_pairs(vec![(
            "SSR_DIAGNOSTICS_DROP_THIS_FRACTION_OF_AUDIO_PACKETS",
            "not-a-number",
        )])
        .expect_err("invalid diagnostics env fraction should fail parsing");
        assert!(err.contains("SSR_DIAGNOSTICS_DROP_THIS_FRACTION_OF_AUDIO_PACKETS"));
    }

    #[test]
    fn load_merges_file_env_and_cli_with_defined_precedence() {
        let path = unique_temp_file("config-load-file-env-cli-precedence");
        let toml = r#"
name = "From File"
port = 6000
backend = "null"
output_format = "f32le"
rsa_key_path = "/tmp/from-file-airport.key"
activity_interval_secs = 10
log_format = "text"
log_filter = "info,shairplay=info"
"#;
        fs::write(&path, toml).expect("failed to write temp config");

        let cli = Cli {
            config: Some(path.clone()),
            name: Some("From CLI".to_string()),
            port: None,
            backend: Some(AudioBackend::Stdout),
            output_format: None,
            alsa_device: None,
            alsa_period_frames: None,
            alsa_buffer_frames: None,
            pipe_path: None,
            password: None,
            max_clients: None,
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: None,
            ap2_pin: None,
            ap2_pairing_store_path: None,
            rsa_key_path: Some("/tmp/from-cli-airport.key".to_string()),
            activity_interval_secs: Some(14),
            activity_snapshot_path: None,
            log_format: Some(LogFormat::Text),
            log_filter: Some("trace,shairplay=trace".to_string()),
        };

        let env_cfg = EnvConfig::from_pairs(vec![
            ("SSR_PORT", "7000"),
            ("SSR_OUTPUT_FORMAT", "s24le"),
            ("SSR_RSA_KEY_PATH", "/tmp/from-env-airport.key"),
            ("SSR_ACTIVITY_INTERVAL_SECS", "12"),
            ("SSR_LOG_FORMAT", "json"),
            ("SSR_LOG_FILTER", "debug,shairplay=debug"),
        ])
        .expect("env parse should succeed");

        let loaded = AppConfig::load_with_env(&cli, env_cfg).expect("config load should succeed");

        // CLI beats env and file.
        assert_eq!(loaded.name, "From CLI");
        assert!(matches!(loaded.backend, AudioBackend::Stdout));
        assert_eq!(loaded.activity_interval_secs, 14);
        assert_eq!(loaded.log_format, LogFormat::Text);
        assert_eq!(
            loaded.log_filter.as_deref(),
            Some("trace,shairplay=trace")
        );
        assert_eq!(loaded.rsa_key_path.as_deref(), Some("/tmp/from-cli-airport.key"));

        // Env beats file.
        assert_eq!(loaded.port, 7000);
        assert_eq!(loaded.output_format, OutputSampleFormat::S24Le);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_uses_env_when_cli_and_file_do_not_override() {
        let cli = Cli {
            config: None,
            name: None,
            port: None,
            backend: None,
            output_format: None,
            alsa_device: None,
            alsa_period_frames: None,
            alsa_buffer_frames: None,
            pipe_path: None,
            password: None,
            max_clients: None,
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: None,
            ap2_pin: None,
            ap2_pairing_store_path: None,
            rsa_key_path: None,
            activity_interval_secs: None,
            activity_snapshot_path: None,
            log_format: None,
            log_filter: None,
        };

        let env_cfg = EnvConfig::from_pairs(vec![
            ("SSR_NAME", "From Env"),
            ("SSR_BACKEND", "pipe"),
            ("SSR_PIPE_PATH", "/tmp/from-env.pcm"),
            ("SSR_RSA_KEY_PATH", "/tmp/from-env-airport.key"),
            ("SSR_ACTIVITY_SNAPSHOT_PATH", "/tmp/activity-env.jsonl"),
            ("SSR_LOG_FILTER", "debug,shairplay=debug"),
        ])
        .expect("env parse should succeed");

        let loaded =
            AppConfig::load_with_env(&cli, env_cfg).expect("config load should succeed");

        assert_eq!(loaded.name, "From Env");
        assert!(matches!(loaded.backend, AudioBackend::Pipe));
        assert_eq!(loaded.pipe_path.as_deref(), Some("/tmp/from-env.pcm"));
        assert_eq!(
            loaded.activity_snapshot_path.as_deref(),
            Some("/tmp/activity-env.jsonl")
        );
        assert_eq!(
            loaded.log_filter.as_deref(),
            Some("debug,shairplay=debug")
        );
        assert_eq!(loaded.rsa_key_path.as_deref(), Some("/tmp/from-env-airport.key"));
    }

    #[test]
    fn load_rejects_empty_log_filter_from_env() {
        let cli = Cli {
            config: None,
            name: None,
            port: None,
            backend: None,
            output_format: None,
            alsa_device: None,
            alsa_period_frames: None,
            alsa_buffer_frames: None,
            pipe_path: None,
            password: None,
            max_clients: None,
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: None,
            ap2_pin: None,
            ap2_pairing_store_path: None,
            rsa_key_path: None,
            activity_interval_secs: None,
            activity_snapshot_path: None,
            log_format: None,
            log_filter: None,
        };

        let env_cfg = EnvConfig::from_pairs(vec![("SSR_LOG_FILTER", "   ")])
            .expect("env parse should succeed");
        let err = AppConfig::load_with_env(&cli, env_cfg)
            .expect_err("blank log filter should fail validation");
        assert!(err.contains("log_filter"));
    }

    #[test]
    fn load_rejects_invalid_environment_value() {
        let cli = Cli {
            config: None,
            name: None,
            port: None,
            backend: None,
            output_format: None,
            alsa_device: None,
            alsa_period_frames: None,
            alsa_buffer_frames: None,
            pipe_path: None,
            password: None,
            max_clients: None,
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: None,
            ap2_pin: None,
            ap2_pairing_store_path: None,
            rsa_key_path: None,
            activity_interval_secs: None,
            activity_snapshot_path: None,
            log_format: None,
            log_filter: None,
        };

        let env_err = EnvConfig::from_pairs(vec![("SSR_PORT", "abc")])
            .expect_err("invalid env value should fail parsing");
        assert!(env_err.contains("SSR_PORT"));

        let env_cfg = EnvConfig::from_pairs(vec![("SSR_BACKEND", "pipe")])
            .expect("env parse should succeed");
        let loaded =
            AppConfig::load_with_env(&cli, env_cfg).expect("valid env config should load");
        assert!(matches!(loaded.backend, AudioBackend::Pipe));
    }
}
