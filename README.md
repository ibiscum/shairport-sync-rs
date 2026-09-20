# shairport-sync-rs

shairport-sync-rs is a Rust-native porting project for shairport-sync.

Its purpose is to provide a production-grade AirPlay audio receiver application in Rust, while reusing the protocol and stream business logic from shairplay-rust.

## Purpose

This repository is the application shell around shairplay-rust. It is intended to deliver:

- Runtime and daemon behavior compatible with existing shairport-sync deployments.
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

Example:

```toml
name = "Shairport Sync RS"
port = 5000
backend = "null"
max_clients = 10
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
sh scripts/debian-smoke-test.sh
```

JSON variant for CI parsing:

```bash
sh scripts/debian-smoke-test-json.sh
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

AP2 protocol-compat assertion example:

```bash
AIRPLAY_MODE=ap2 AP2_PIN="12345678" AP2_PAIRING_STORE_PATH="/tmp/ap2-pairings.toml" \
AP2_EXPECT_FEATURES="0x405D4A00,0x14340" AP2_EXPECT_FLAGS="0x204" \
./scripts/debian-smoke-test-json.sh
```

When AP2 checks are enabled, the scripts verify a matching resolved `_airplay._tcp` TXT record for the same service name and port, then validate `features=` and `flags=` values.

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
