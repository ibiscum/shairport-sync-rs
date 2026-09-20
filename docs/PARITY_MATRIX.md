# Shairport Sync -> shairport-sync-rs Parity Matrix

## Objective
Build a Rust-native shairport-sync-rs application that reaches feature parity with the upstream C project by using shairplay-rust as the protocol and stream-processing backend.

## Source Baselines
- Upstream player: /opt/data/shairport-sync
- Backend library: /opt/data/shairplay-rust
- Target app shell: /opt/data/shairport-sync-rs

## Architecture Boundary
Use shairplay-rust for protocol/business logic:
- Discovery and RTSP/RAOP server lifecycle
- AP1 and AP2 session handling
- Pairing and crypto
- RTP ingest, decode, optional resample/mixdown
- Metadata callbacks and event hooks

Implement in shairport-sync-rs:
- CLI and config surface compatible with shairport-sync user workflows
- Audio output backend abstraction (ALSA/PipeWire/Pulse/sndio/stdout/pipe, etc.)
- IPC and integration surfaces (D-Bus, MPRIS, MQTT, metadata pipe/multicast)
- Runtime lifecycle (daemon mode, service integration, logging, diagnostics)

## Parity Matrix (Subsystem Level)

| Subsystem | shairport-sync (C) reference | shairplay-rust capability | shairport-sync-rs work required | Status |
|---|---|---|---|---|
| Main runtime and process lifecycle | shairport.c | RaopServer lifecycle exists; library-first | Executable startup/shutdown and Ctrl+C path are wired; daemon mode/service supervision still pending | IN PROGRESS |
| Discovery (mDNS/Bonjour) | mdns.c + mdns_*.c | Built-in discovery in net/mdns.rs | Basic discovery works via backend crate; runtime interface/identity diagnostics still need parity options | IN PROGRESS |
| RTSP/RAOP protocol handling | rtsp.c, rtp.c, ap2_* modules | Implemented in raop/* and proto/* | Protocol-related builder mapping now includes output sample-rate/channels, AP1 advertisement controls, AP mode/AP2 PIN wiring, and AP2 pairing persistence path wiring; callback/integration parity sweep remains | IN PROGRESS |
| AP1 audio path | rtp.c + decoders | AP1 supported (ALAC/L16, optional encryption modes) | AP1 mode/config wiring is implemented (mode selection + AP1 advertisement controls) and smoke/CI asserts `cn`/`et`; sender interoperability matrix remains | IN PROGRESS |
| AP2 audio path | ap2_buffered_audio_processor.c + related | AP2 supported with pairing, encrypted transport, buffered/realtime audio | AP2 mode defaults, PIN support, and file-backed pairing persistence are wired; smoke/CI now assert `_airplay._tcp` features/flags for AP2 mode; broader AP2 interop validation remains | IN PROGRESS |
| Timing and sync | classic NTP + NQPTP external model | AP1 NTP in crate; AP2 has PTP sink and partial unwired scheduling | Define target parity: acceptable initial sync behavior vs full clock-sync parity milestone | TODO |
| Audio decode/transcode/mixdown | ffmpeg/libsoxr pipeline | AAC/ALAC decode, f32 output, optional resample/mixdown in crate | Output format conversion exists for pipe/stdout paths; broader device negotiation parity remains | IN PROGRESS |
| Audio backend abstraction | audio.c + audio_alsa.c/audio_pw.c/... | Not provided by backend crate (app responsibility) | Rust backend abstraction implemented with null/stdout/pipe and Linux ALSA; more backends pending | IN PROGRESS |
| Metadata extraction and dispatch | metadata/* + metadata options | Metadata forwarding hooks are available | Implement metadata fan-out: pipe, multicast, D-Bus, MPRIS, MQTT payload model | TODO |
| DACP / remote control (Classic) | dacp.c | dacp module exists in crate | Build app-facing remote control bridge and expose via IPC | TODO |
| D-Bus interface | dbus-service.c | Not in crate | Rebuild native D-Bus API and settings controls | TODO |
| MPRIS interface | mpris-service.c | Not in crate | Rebuild MPRIS-like interface and behavior compatibility | TODO |
| MQTT integration | mqtt.c | Not in crate | Implement MQTT publisher/subscriber and command handling policy | TODO |
| Config file compatibility | scripts/shairport-sync.conf + parser in C runtime | Builder API options exist but no shairport-sync.conf parser | TOML config + CLI precedence + validation are implemented; legacy shairport-sync.conf compatibility layer still pending | IN PROGRESS |
| Command-line parity | man page + getopt/popt paths | No dedicated CLI app in backend | Core CLI switches/help/version path implemented; migration aliases/full flag parity still pending | IN PROGRESS |
| Logging and diagnostics | common.c + debug paths | Backend has diagnostics features | Structured tracing and startup/shutdown diagnostics exist; syslog/journal parity still pending | IN PROGRESS |
| Daemon/service operation | libdaemon/systemd helpers | Not in crate | Implement foreground/background policy and systemd-friendly behavior | TODO |
| Build/package outputs | autotools packaging scripts | Rust crate packaging exists only for backend | Add distro packaging, service files, config install layout | TODO |
| Test strategy | tests/ and integration practices in C project | Strong crate test coverage exists | App-level unit/regression tests are in place; Debian smoke scripts and CI workflow added, including AP1 `cn`/`et` and AP2 `_airplay._tcp` features/flags protocol-compat assertions; sender-matrix E2E still pending | IN PROGRESS |

## Functional Priority for Parity
1. Audio receive/playback parity (AP1 and AP2) with robust device output.
2. Configuration and runtime behavior parity for existing shairport-sync users.
3. Metadata and IPC parity (D-Bus, MPRIS, MQTT, metadata outputs).
4. Operational parity (daemonization, service management, packaging, observability).
5. Advanced and edge-path parity (remote control variants, optional backends, uncommon build flags).

## Explicit Non-Goals for Initial Port
- Video/screen mirroring parity from backend crate (shairport-sync upstream is audio-focused).
- Full flag-for-flag compile-time duplication of every historical autotools toggle in milestone 1.

## Risk Register
- AP2 timing parity risk: backend currently documents partial PTP wiring; multi-room sync behavior must be qualified early.
- Audio backend complexity risk: ALSA/PipeWire/Pulse behavior and latency tuning are usually the largest integration effort.
- Compatibility risk: users rely on existing config names/semantics; parser and defaults must be migration-safe.
- IPC scope risk: D-Bus/MPRIS/MQTT behavior differences can break automations even when audio works.

## Exit Criteria for "Parity Achieved"
- Existing shairport-sync users can migrate common deployments by changing binary path only (minimal config edits).
- AP1 and AP2 audio behavior validated with representative iOS/macOS sender matrix.
- Core metadata and control integrations validated in real automations.
- Runtime reliability and service behavior validated under long-running conditions.
