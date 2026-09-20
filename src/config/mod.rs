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
            pipe_path: Some("/tmp/shairport-sync-rs-test.pcm".to_string()),
            password: None,
            max_clients: 1,
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

        let err = cfg.validate().expect_err("pipe backend should require non-empty pipe_path");
        assert!(err.contains("pipe_path"));
    }

    #[test]
    fn validate_rejects_empty_name() {
        let mut cfg = valid_base();
        cfg.name = "".to_string();

        let err = cfg.validate().expect_err("empty name should fail validation");
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
password = "file-pass"
max_clients = 2
"#;
        fs::write(&path, toml).expect("failed to write temp config");

        let cli = Cli {
            config: Some(path.clone()),
            name: Some("From CLI".to_string()),
            port: Some(7000),
            backend: None,
            output_format: Some(OutputSampleFormat::S16Le),
            alsa_device: None,
            pipe_path: Some("/tmp/from-cli.pcm".to_string()),
            password: Some("cli-pass".to_string()),
            max_clients: Some(4),
        };

        let loaded = AppConfig::load(&cli).expect("config load should succeed");

        assert_eq!(loaded.name, "From CLI");
        assert_eq!(loaded.port, 7000);
        assert!(matches!(loaded.backend, AudioBackend::Pipe));
        assert_eq!(loaded.output_format, OutputSampleFormat::S16Le);
        assert_eq!(loaded.pipe_path.as_deref(), Some("/tmp/from-cli.pcm"));
        assert_eq!(loaded.password.as_deref(), Some("cli-pass"));
        assert_eq!(loaded.max_clients, 4);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_rejects_missing_config_file() {
        let cli = Cli {
            config: Some(PathBuf::from("/definitely/not/present/shairport-sync-rs.toml")),
            name: None,
            port: None,
            backend: None,
            output_format: None,
            alsa_device: None,
            pipe_path: None,
            password: None,
            max_clients: None,
        };

        let err = AppConfig::load(&cli).expect_err("missing config file should fail");
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
            pipe_path: None,
            password: None,
            max_clients: None,
        };

        let err = AppConfig::load(&cli).expect_err("invalid config should fail");
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
            let loaded =
                AppConfig::load(&cli).unwrap_or_else(|e| panic!("case '{}' failed: {e}", case.name));

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
            let err = AppConfig::load(&cli)
                .expect_err("invalid case should fail config validation");
            assert!(
                err.contains(case.expected_error_substring),
                "unexpected error for case '{}': {err}",
                case.name
            );
        }
    }
}
