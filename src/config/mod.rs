use clap::Parser;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[command(name = "shairport-sync-rs", version, about = "Rust-native AirPlay audio receiver")]
pub struct Cli {
    #[arg(long, value_name = "FILE", help = "Path to TOML config file")]
    pub config: Option<PathBuf>,

    #[arg(long, help = "AirPlay display name")]
    pub name: Option<String>,

    #[arg(long, help = "RTSP listen port")]
    pub port: Option<u16>,

    #[arg(long, value_enum, help = "Audio sink backend")]
    pub backend: Option<AudioBackend>,

    #[arg(long, value_enum, help = "Output sample format for pipe/stdout backends")]
    pub output_format: Option<OutputSampleFormat>,

    #[arg(long, help = "ALSA playback device name (Linux only), e.g. default or hw:0,0")]
    pub alsa_device: Option<String>,

    #[arg(long, help = "Output file/path for pipe backend (raw f32le stream)")]
    pub pipe_path: Option<String>,

    #[arg(long, help = "HTTP Digest password")]
    pub password: Option<String>,

    #[arg(long, help = "Maximum concurrent clients")]
    pub max_clients: Option<usize>,
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

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub name: String,
    pub port: u16,
    pub backend: AudioBackend,
    pub output_format: OutputSampleFormat,
    pub alsa_device: Option<String>,
    pub pipe_path: Option<String>,
    pub password: Option<String>,
    pub max_clients: usize,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct FileConfig {
    name: Option<String>,
    port: Option<u16>,
    backend: Option<AudioBackend>,
    output_format: Option<OutputSampleFormat>,
    alsa_device: Option<String>,
    pipe_path: Option<String>,
    password: Option<String>,
    max_clients: Option<usize>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "Shairport Sync RS".to_string(),
            port: 5000,
            backend: AudioBackend::Null,
            output_format: OutputSampleFormat::F32Le,
            alsa_device: Some("default".to_string()),
            pipe_path: Some("/tmp/shairport-sync-rs.pcm".to_string()),
            password: None,
            max_clients: 10,
        }
    }
}

impl AppConfig {
    pub fn load(cli: &Cli) -> Result<Self, String> {
        let mut cfg = AppConfig::default();

        if let Some(path) = &cli.config {
            let content = fs::read_to_string(path)
                .map_err(|e| format!("failed to read config file {}: {e}", path.display()))?;
            let file_cfg: FileConfig = toml::from_str(&content)
                .map_err(|e| format!("failed to parse config file {}: {e}", path.display()))?;
            cfg.apply_file(file_cfg);
        }

        cfg.apply_cli(cli);
        cfg.validate()?;
        Ok(cfg)
    }

    fn apply_file(&mut self, file: FileConfig) {
        if let Some(v) = file.name {
            self.name = v;
        }
        if let Some(v) = file.port {
            self.port = v;
        }
        if let Some(v) = file.backend {
            self.backend = v;
        }
        if let Some(v) = file.output_format {
            self.output_format = v;
        }
        if let Some(v) = file.alsa_device {
            self.alsa_device = Some(v);
        }
        if let Some(v) = file.pipe_path {
            self.pipe_path = Some(v);
        }
        if let Some(v) = file.password {
            self.password = Some(v);
        }
        if let Some(v) = file.max_clients {
            self.max_clients = v;
        }
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
        if let Some(v) = &cli.pipe_path {
            self.pipe_path = Some(v.clone());
        }
        if let Some(v) = &cli.password {
            self.password = Some(v.clone());
        }
        if let Some(v) = cli.max_clients {
            self.max_clients = v;
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
        #[cfg(target_os = "linux")]
        {
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
                return Err(
                    "backend alsa currently supports only output_format=f32le".to_string(),
                );
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
        Ok(())
    }
}
