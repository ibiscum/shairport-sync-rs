# shairport-sync-rs

shairport-sync-rs is a Rust-native porting project for shairport-sync.

Its purpose is to provide a production-grade AirPlay audio receiver application in Rust, while reusing the protocol and stream business logic from shairplay-rust.

## Purpose

This repository is an application shell around shairplay-rust. It is intended to deliver:

- Runtime and daemon behavior shall be compatible with existing shairport-sync deployments.
- Config and CLI compatibility for migration from existing systems.
- Audio backend integrations for Linux and BSD style deployments.
- Metadata and control integrations used in home automation and desktop environments.

In short:

- shairplay-rust provides protocol and media core logic.
- shairport-sync-rs provides the executable, integration surfaces, and operational compatibility.

## M0 Scaffold Quick Start

Build and run the initial M0 slice:

```bash
cargo run -- --name "Shairport Sync RS" --port 5000 --backend null
```

Optional config-file startup:

```bash
cargo run -- --config ./shairport-sync-rs.toml
```

Supported M0 config keys (TOML):

- `name` (string)
- `port` (integer)
- `backend` (`"null"`, `"stdout"`, `"pipe"`, and on Linux `"alsa"`)
- `output_format` (`"f32le"`, `"s16le"`, or `"s24le"`; currently applied to `stdout` and `pipe`, default `"f32le"`)
- `alsa_device` (string, used by Linux ALSA backend; default `"default"`)
- `alsa_period_frames` (integer, Linux ALSA backend period size in frames; default `1024`)
- `alsa_buffer_frames` (integer, Linux ALSA backend buffer size in frames; default `4096`; must be >= `alsa_period_frames`)
- `pipe_path` (string, used by `pipe` backend; default `"/tmp/shairport-sync-rs.pcm"`)
- `password` (string)
- `max_clients` (integer)
- `raop_output_sample_rate` (integer, optional; resamples decoded AirPlay audio before backend delivery)
- `raop_output_max_channels` (integer, optional; downmixes decoded AirPlay audio to this maximum channel count)
- `ap1_codecs` (array, optional; advertised AP1 codec list, values: `"pcm"`, `"alac"`)
- `ap1_encryption` (array, optional; advertised AP1 encryption list, values: `"none"`, `"rsa"`, `"fairplay"`)
- `airplay_mode` (`"ap1"` or `"ap2"`; default `"ap2"`)
- `ap2_pin` (string, optional; requires `airplay_mode = "ap2"`)
- `ap2_pairing_store_path` (string, optional; requires `airplay_mode = "ap2"`; persists AP2 pairings/identity)
- `activity_interval_secs` (integer; periodic activity snapshot interval in seconds, default `30`)
- `activity_snapshot_path` (string, optional; JSONL file for machine-readable activity snapshots)
- `log_format` (`"text"` or `"json"`; default `"text"`)
- `log_filter` (string, optional; tracing filter directives, e.g. `"info,shairplay=debug"`)

Linux ALSA CLI example:

```bash
cargo run -- --backend alsa --alsa-device default
```

Pipe backend CLI example:

```bash
cargo run -- --backend pipe --pipe-path /tmp/shairport-sync-rs.pcm
```

Pipe backend with 16-bit output:

```bash
cargo run -- --backend pipe --pipe-path /tmp/shairport-sync-rs.pcm --output-format s16le
```

RAOP protocol tuning example (AP1 advertisement + output shaping):

```bash
cargo run -- \
	--backend null \
	--airplay-mode ap1 \
	--raop-output-sample-rate 48000 \
	--raop-output-max-channels 2 \
	--ap1-codecs pcm,alac \
	--ap1-encryption none,rsa
```

AP2 pairing PIN example:

```bash
cargo run -- \
	--backend null \
	--airplay-mode ap2 \
	--ap2-pin 12345678
```

AP2 pairing persistence example:

```bash
cargo run -- \
	--backend null \
	--airplay-mode ap2 \
	--ap2-pairing-store-path /var/lib/shairport-sync-rs/ap2-pairings.toml
```

AP1 + ALSA example (encryption `none,rsa,fairplay`):

```bash
cargo run -- \
	--name "Shairport Sync RS AP1" \
	--backend alsa \
	--alsa-device default \
	--airplay-mode ap1 \
	--ap1-codecs pcm,alac \
	--ap1-encryption none,rsa,fairplay
```

Integration test ALSA

```bash
cargo run -- --backend pipe --pipe-path /tmp/ssr-ap1.pcm --airplay-mode ap1 --ap1-codecs pcm,alac --ap1-encryption none,rsa,fairplay


cargo run -- --name "Shairport Sync RS AP1" --port 5000 --backend alsa --alsa-device default --airplay-mode ap1 --ap1-codecs pcm,alac --ap1-encryption none --alsa-period-frames 1024 --alsa-buffer-frames 4096 --log-format json --log-filter info,shairplay=debug
```

Example:

```toml
name = "Shairport Sync RS"
port = 5000
backend = "null"
max_clients = 10
```

Sectioned TOML layout (recommended):

```toml
[general]
name = "Shairport Sync RS"
port = 5000
max_clients = 10

[audio]
backend = "alsa"
output_format = "f32le"
airplay_mode = "ap2"

[audio.alsa]
device = "default"
period_frames = 1024
buffer_frames = 4096

[audio.pipewire]
# Forward-compatible section for upcoming PipeWire backend support.
# Keep backend = "alsa" until runtime PipeWire backend support lands.
# application_name = "Shairport Sync RS"
# output_rate = 48000

[observability]
activity_interval_secs = 30
log_format = "text"
```

Linux ALSA example:

```toml
name = "Shairport Sync RS"
port = 5000
backend = "alsa"
alsa_device = "default"
max_clients = 10
```

Pipe backend example:

```toml
name = "Shairport Sync RS"
port = 5000
backend = "pipe"
pipe_path = "/tmp/shairport-sync-rs.pcm"
output_format = "f32le"
max_clients = 10
```

## Manual Run and Debian Smoke Tests

Manual run (no installation):

```bash
cargo run -- --backend null --name "SSR Debian Smoke" --port 5000
```

Smoke test scripts (run from repository root):

```bash
bash scripts/debian-smoke-test.sh
```

JSON variant for CI parsing:

```bash
bash scripts/debian-smoke-test-json.sh
```

The JSON script prints one JSON object to stdout with `status` set to `passed` or `failed` and includes check-level details and log artifact paths.

Optional overrides:

```bash
PORT=5001 NAME="SSR CI Smoke" BACKEND=null ./scripts/debian-smoke-test.sh
PORT=5001 NAME="SSR CI Smoke" BACKEND=null ./scripts/debian-smoke-test-json.sh
```

Optional AP1 advertisement assertions (for protocol-compat checks):

```bash
AP1_EXPECT_CN="0,1" AP1_EXPECT_ET="0" ./scripts/debian-smoke-test.sh
AP1_EXPECT_CN="0,1" AP1_EXPECT_ET="0" ./scripts/debian-smoke-test-json.sh
```

When set, `AP1_EXPECT_CN` and `AP1_EXPECT_ET` are matched against the resolved `_raop._tcp` TXT record (`cn=` and `et=`) for the exact service instance matching both configured name and port.

To force AP1 mode during smoke tests, set launch overrides as well:

```bash
AIRPLAY_MODE=ap1 AP1_CODECS="pcm,alac" AP1_ENCRYPTION="none" \
AP1_EXPECT_CN="0,1" AP1_EXPECT_ET="0" ./scripts/debian-smoke-test.sh
```

Optional AP1 hold-window stability check (server must remain alive during hold):

```bash
HOLD_SECONDS=30 AIRPLAY_MODE=ap1 AP1_CODECS="pcm,alac" AP1_ENCRYPTION="none" \
AP1_EXPECT_CN="0,1" AP1_EXPECT_ET="0" ./scripts/debian-smoke-test-json.sh
```

AP1 long-play soak harness (iterative AP1 startup/discovery/hold/shutdown checks):

```bash
ITERATIONS=10 HOLD_SECONDS=30 BUILD_FIRST=1 bash scripts/ap1-soak.sh
```

AP1/ALSA silence quick verdict from activity snapshots:

```bash
bash scripts/ap1-alsa-diagnose.sh /tmp/ssr-activity.jsonl
```

M0 evidence tests:

```bash
# Integration: AP1-mode runtime startup and clean SIGINT shutdown.
cargo test --test m0_runtime_integration

# Regression: discovery + AP1 advertisement checks via smoke JSON.
cargo test --test m0_smoke_regression -- --ignored

# Regression: AP1 soak harness single-iteration check.
cargo test --test ap1_soak_harness_regression -- --ignored

# Regression: AP1 reconnect/gapless diagnostics counters.
cargo test observability::tests::reconnect_and_gapless_regression_counters_progress_as_expected
```

## Observability and Activity Monitor

The runtime now emits periodic activity snapshots covering the currently implemented
playback path and backends (null/stdout/pipe/alsa):

- session lifecycle counters
- connected client count
- audio callback/sample counters
- backend write/error/recovery counters
- ALSA underrun, recovery-attempt/failure, buffer-depth, and latency counters
- metadata update counters
- sender action counters (connect/disconnect/reconnect, volume, metadata, flush)
- gapless transition candidate counters (session rollover while active)

Activity monitor controls (CLI/TOML):

- `activity_interval_secs` (default `30`): snapshot interval
- `activity_snapshot_path` (optional): JSONL output path for activity snapshots

Logging controls (CLI/TOML):

- `log_format`: `text` (default) or `json`
- `log_filter`: tracing filter directives (default derived from `diagnostics.log_verbosity`, e.g. `info,shairplay=info`)

SSR environment variables (`defaults < TOML file < SSR_* env < CLI`):

- `SSR_NAME`: string (receiver display name)
- `SSR_PORT`: integer (`u16`, e.g. `5000`)
- `SSR_BACKEND`: `null` | `stdout` | `pipe` | `alsa` (Linux)
- `SSR_OUTPUT_FORMAT`: `f32le` | `s16le` | `s24le`
- `SSR_ALSA_DEVICE`: string (Linux ALSA device name)
- `SSR_ALSA_PERIOD_FRAMES`: integer (`u32`)
- `SSR_ALSA_BUFFER_FRAMES`: integer (`u32`)
- `SSR_PIPE_PATH`: string (pipe backend output path)
- `SSR_PASSWORD`: string
- `SSR_MAX_CLIENTS`: integer (`usize`)
- `SSR_RAOP_OUTPUT_SAMPLE_RATE`: integer (`u32`)
- `SSR_RAOP_OUTPUT_MAX_CHANNELS`: integer (`u8`)
- `SSR_AP1_CODECS`: comma-separated list of `pcm`, `alac`
- `SSR_AP1_ENCRYPTION`: comma-separated list of `none`, `rsa`, `fairplay`
- `SSR_AIRPLAY_MODE`: `ap1` | `ap2`
- `SSR_AP2_PIN`: string
- `SSR_AP2_PAIRING_STORE_PATH`: string
- `SSR_ACTIVITY_INTERVAL_SECS`: integer (`u64`)
- `SSR_ACTIVITY_SNAPSHOT_PATH`: string (JSONL output path)
- `SSR_LOG_FORMAT`: `text` | `json`
- `SSR_LOG_FILTER`: tracing filter directives, e.g. `info,shairplay=debug`

Diagnostics env variables (parsed in the same precedence chain):

| Variable | Type / Allowed Values | Notes |
| --- | --- | --- |
| `SSR_DIAGNOSTICS_DISABLE_RESEND_REQUESTS` | bool: `yes/no`, `true/false`, `1/0` | Parsed; currently logs a compatibility warning (not yet wired to protocol behavior). |
| `SSR_DIAGNOSTICS_STATISTICS` | bool: `yes/no`, `true/false`, `1/0` | Enables/disables periodic activity snapshot logging task. |
| `SSR_DIAGNOSTICS_LOG_VERBOSITY` | integer `u8` (expected `0..3`) | Maps to the default log filter when `log_filter` / `SSR_LOG_FILTER` is not set. |
| `SSR_DIAGNOSTICS_LOG_SHOW_FILE_AND_LINE` | bool: `yes/no`, `true/false`, `1/0` | Toggles file/line fields in logger output. |
| `SSR_DIAGNOSTICS_LOG_SHOW_TIME_SINCE_STARTUP` | bool: `yes/no`, `true/false`, `1/0` | Uses uptime-style timestamp in log formatter. |
| `SSR_DIAGNOSTICS_LOG_SHOW_TIME_SINCE_LAST_MESSAGE` | bool: `yes/no`, `true/false`, `1/0` | Uses delta-since-last-message timestamp formatter. |
| `SSR_DIAGNOSTICS_DROP_THIS_FRACTION_OF_AUDIO_PACKETS` | float `0.0..1.0` | Parsed/validated; currently logs a compatibility warning (simulation not yet wired). |
| `SSR_DIAGNOSTICS_RETAIN_COVER_ART` | bool: `yes/no`, `true/false`, `1/0` | Parsed; currently logs a compatibility warning (retention behavior not yet wired). |
| `SSR_DIAGNOSTICS_GET_PLIST_METADATA` | bool: `yes/no`, `true/false`, `1/0` | Parsed; currently logs a compatibility warning (plist stream not yet wired). |

When `diagnostics.log_verbosity` / `SSR_DIAGNOSTICS_LOG_VERBOSITY` is `3`,
`shairport-sync-rs` also emits extra protocol diagnostics intended to mirror
upstream high-verbosity troubleshooting workflows:

- startup protocol summary (target: `protocol`)
- AP1 mDNS advertisement intent summary (`cn`, `et`, `tp`, `pw`, `sr`, `ch`)
- parameter/control exchange events (`set volume`, metadata updates, client connect/disconnect)

These extra diagnostics are designed for interoperability debugging and can be noisy.

Diagnostics examples:

```bash
# JSON logs with high diagnostic verbosity and delta timestamps.
SSR_LOG_FORMAT=json \
SSR_DIAGNOSTICS_LOG_VERBOSITY=3 \
SSR_DIAGNOSTICS_LOG_SHOW_TIME_SINCE_LAST_MESSAGE=yes \
cargo run -- --backend null

# Text logs tuned for Docker tailing with periodic stats snapshots.
SSR_LOG_FORMAT=text \
SSR_DIAGNOSTICS_LOG_VERBOSITY=3 \
SSR_DIAGNOSTICS_STATISTICS=yes \
SSR_DIAGNOSTICS_LOG_SHOW_FILE_AND_LINE=yes \
SSR_DIAGNOSTICS_LOG_SHOW_TIME_SINCE_STARTUP=yes \
cargo run -- \
	--name "Shairport Sync RS AP1" \
	--backend alsa \
	--alsa-device default \
	--airplay-mode ap1 \
	--ap1-codecs pcm,alac \
	--ap1-encryption none,rsa
```

Examples:

```bash
cargo run -- --backend null --log-format json
cargo run -- --backend alsa --activity-interval-secs 10 --activity-snapshot-path /tmp/ssr-activity.jsonl
SSR_LOG_FORMAT=json SSR_LOG_FILTER=info,shairplay=debug cargo run -- --backend null
SSR_BACKEND=pipe SSR_PIPE_PATH=/tmp/ssr-env.pcm SSR_OUTPUT_FORMAT=s24le cargo run
```

### Troubleshooting

1. Running bash scripts via `sh`

Symptom: errors like `[[: not found` or `Syntax error: Bad for loop variable`.

Cause: Debian `sh` is typically `dash`, which does not support Bash-only syntax.

Fix: run scripts with `bash` or execute them directly after `chmod +x`:

```bash
bash scripts/debian-smoke-test.sh
bash scripts/debian-smoke-test-json.sh
./scripts/debian-smoke-test.sh
./scripts/debian-smoke-test-json.sh
```

2. mDNS port mismatch interpretation in Avahi output

Symptom: the smoke script reports missing expected port even though the service name appears.

Cause: `_raop._tcp` entries from multiple devices or previous local runs may coexist; the name alone does not prove the expected instance/port.

Fix: rely on resolved (`=`) lines from parseable output and verify both service name and port together:

```bash
avahi-browse -prt _raop._tcp
```

## License and Notices

Conservative licensing policy for this repository:

1. Code in this repository is licensed under LGPL-3.0-or-later by default.
2. Any code copied or adapted from shairport-sync keeps its original permissive license notice as stated in the source file headers.
3. If a file carries a specific license header, that file header controls for that file.
4. Distributions that combine code under multiple licenses must preserve all required notices and comply with all applicable license obligations.

Upstream license context used for this policy:

- shairplay-rust: LGPL-3.0 (see its LICENSE file).
- shairport-sync: source files include permissive license headers; the upstream COPYING file directs readers to individual source files for license terms.

Practical compliance guidance for contributors:

- Keep original copyright and permission headers on imported/adapted files.
- Add clear attribution in commit messages when porting logic.
- Do not remove upstream notices.

Canonical policy files in this repository:

- LICENSE
- NOTICE

Per-file headers remain authoritative for file-specific exceptions and upstream-preserved notices.
