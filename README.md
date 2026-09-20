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
