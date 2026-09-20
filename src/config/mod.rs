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

    #[arg(long, help = "Resample AirPlay output to this rate before backend delivery")]
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
    pub raop_output_sample_rate: Option<u32>,
    pub raop_output_max_channels: Option<u8>,
    pub ap1_codecs: Option<Vec<Ap1CodecConfig>>,
    pub ap1_encryption: Option<Vec<Ap1EncryptionConfig>>,
    pub airplay_mode: AirPlayModeConfig,
    pub ap2_pin: Option<String>,
    pub ap2_pairing_store_path: Option<String>,
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
    raop_output_sample_rate: Option<u32>,
    raop_output_max_channels: Option<u8>,
    ap1_codecs: Option<Vec<Ap1CodecConfig>>,
    ap1_encryption: Option<Vec<Ap1EncryptionConfig>>,
    airplay_mode: Option<AirPlayModeConfig>,
    ap2_pin: Option<String>,
    ap2_pairing_store_path: Option<String>,
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
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: AirPlayModeConfig::Ap2,
            ap2_pin: None,
            ap2_pairing_store_path: None,
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
        if let Some(v) = file.raop_output_sample_rate {
            self.raop_output_sample_rate = Some(v);
        }
        if let Some(v) = file.raop_output_max_channels {
            self.raop_output_max_channels = Some(v);
        }
        if let Some(v) = file.ap1_codecs {
            self.ap1_codecs = Some(v);
        }
        if let Some(v) = file.ap1_encryption {
            self.ap1_encryption = Some(v);
        }
        if let Some(v) = file.airplay_mode {
            self.airplay_mode = v;
        }
        if let Some(v) = file.ap2_pin {
            self.ap2_pin = Some(v);
        }
        if let Some(v) = file.ap2_pairing_store_path {
            self.ap2_pairing_store_path = Some(v);
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
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: AirPlayModeConfig::Ap2,
            ap2_pin: None,
            ap2_pairing_store_path: None,
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

        let err = cfg
            .validate()
            .expect_err("ap2_pin should require ap2 mode");
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
raop_output_sample_rate = 44100
raop_output_max_channels = 2
ap1_codecs = ["pcm", "alac"]
ap1_encryption = ["none", "rsa"]
airplay_mode = "ap1"
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
            raop_output_sample_rate: Some(48000),
            raop_output_max_channels: Some(1),
            ap1_codecs: Some(vec![Ap1CodecConfig::Alac]),
            ap1_encryption: Some(vec![Ap1EncryptionConfig::Rsa]),
            airplay_mode: Some(AirPlayModeConfig::Ap1),
            ap2_pin: None,
            ap2_pairing_store_path: None,
        };

        let loaded = AppConfig::load(&cli).expect("config load should succeed");

        assert_eq!(loaded.name, "From CLI");
        assert_eq!(loaded.port, 7000);
        assert!(matches!(loaded.backend, AudioBackend::Pipe));
        assert_eq!(loaded.output_format, OutputSampleFormat::S16Le);
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
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: None,
            ap2_pin: None,
            ap2_pairing_store_path: None,
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
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: None,
            ap2_pin: None,
            ap2_pairing_store_path: None,
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
}
