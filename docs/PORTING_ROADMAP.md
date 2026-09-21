# shairport-sync-rs Porting Roadmap

## Goal
Deliver a Rust-native replacement for shairport-sync that uses shairplay-rust for protocol/business logic while matching operational expectations of production shairport-sync deployments.

## Guiding Principles
- Reuse, do not reimplement: keep protocol handling inside shairplay-rust.
- Preserve operator workflows: config semantics and service behavior should feel familiar.
- Ship in slices: produce usable milestones early, then expand toward full parity.
- Treat AP2 timing behavior as a first-class validation axis.

## Target Runtime Shape
- Binary crate: shairport-sync-rs (daemon/service executable)
- Internal crates/modules:
  - config: file parsing + CLI + effective config model
  - runtime: lifecycle, signal handling, task supervision
  - audio: output trait + backend implementations
  - integrations: metadata pipe/multicast, D-Bus, MPRIS, MQTT, DACP bridge
  - observability: logging, metrics hooks, diagnostics toggles
- Backend dependency: shairplay-rust as core RAOP/AP stack

## Milestones

### M0 - Bootstrap and Skeleton
Deliverables:
- Cargo workspace and executable layout.
- Config model with minimal options (name, port, mode, output selection).
- RaopServer startup/shutdown wired to runtime.
- Health logging and clean Ctrl+C handling.

Acceptance:
- Receiver appears in AirPlay discovery.
- Basic AP1 playback to a dummy or stdout backend works end-to-end.

Status: FULFILLED

Evidence:
- Runtime startup/shutdown and Ctrl+C flow are implemented (`src/main.rs`, `src/runtime/mod.rs`).
- Discovery is covered by Debian smoke tests and CI mDNS assertions (`scripts/debian-smoke-test*.sh`, `.github/workflows/debian-smoke.yml`).
- Basic AP1 path to dummy/stdout backends is covered by app-level tests (`src/audio/mod.rs`) and AP1 advertisement/protocol checks in smoke/CI.

### M1 - Audio Core Parity
Deliverables:
- Audio backend abstraction and at least ALSA + Pipe/Stdout implementations.
- Format negotiation and conversion from backend f32 stream to backend device requirements.
- Latency and buffer controls exposed in config.

Acceptance:
- Stable long-play sessions on AP1.
- Gapless transition and reconnect behavior validated for common sender actions.

### M1.1 - PipeWire Pipeline and Backend Behavior Compliance
Deliverables:
- Native PipeWire audio backend with a production-ready playback pipeline (stream creation, format setup, buffering, start/stop, recovery).
- Backend capability mapping layer so backend-specific constraints are translated into effective runtime behavior consistent with upstream shairport-sync.
- Conformance test suite for all implemented backends (ALSA, PipeWire, Pipe, Stdout, Null) covering lifecycle, underrun/recovery, reconnect, and shutdown semantics.
- Operator-facing compatibility notes that document any unavoidable differences from upstream behavior.

Acceptance:
- PipeWire backend passes long-play and reconnect smoke/regression scenarios equivalent to ALSA validation depth.
- Every implemented backend satisfies the same backend conformance contract and matches upstream shairport-sync behavior for start, write, pause/resume (where supported), stop, and error handling.
- Port/listener release and service advertisement teardown behavior remain deterministic during shutdown across backend switches.
- Any remaining backend-specific deltas vs upstream are explicitly documented and justified.

### M2 - AP2 Operational Parity
Deliverables:
- AP2 mode controls (pairing store, PIN, mode selection).
- Runtime compatibility options mapped from shairport-sync expectations.
- Explicit diagnostics around AP2 session establishment and timing.

Acceptance:
- AP2 playback (stereo + multichannel downmix paths) validated on iOS/macOS versions in support matrix.
- Session handoff and reconnect behavior consistent with user expectations.

### M3 - Metadata and IPC Parity
Deliverables:
- Metadata fan-out: pipe and multicast outputs.
- D-Bus native interface.
- MPRIS-like interface.
- MQTT integration (publish metadata and handle allowed control commands).

Acceptance:
- Existing automation use-cases can be migrated with documented compatibility notes.

### M4 - Service, Packaging, and Compatibility
Deliverables:
- Systemd service units and install layout.
- Config compatibility layer for common shairport-sync.conf fields.
- Migration guide and known-differences matrix.

Acceptance:
- Typical distro/service deployment runs unattended with restart safety.
- Migration dry-run from existing config succeeds for common scenarios.

### M5 - Hardening and Full Parity Sweep
Deliverables:
- End-to-end regression suite with sender matrix.
- Soak tests for long-running stability.
- Performance and memory profiling, startup/connect latency tuning.

Acceptance:
- Defined parity checklist from PARITY_MATRIX.md signed off.

## Work Breakdown by Team Stream

### Stream A: Runtime and Config
- Implement config parser compatible with key shairport-sync.conf fields.
- Define precedence model: defaults < file < environment < CLI.
- Add validation with clear operator-facing errors.

### Stream B: Audio Backends
- Build backend trait with pluggable implementations.
- Implement ALSA first, then PipeWire/Pulse as needed.
- Add backend conformance tests (start, write, pause, stop, recover).

### Stream C: Integrations
- Metadata schema normalization.
- D-Bus/MPRIS API design based on current behavior expectations.
- MQTT topic and command contract.

### Stream D: QA and Interop
- Sender coverage matrix: iOS, macOS, HomePod/Apple TV interactions where feasible.
- AP1/AP2 scenario scripts (connect, seek, pause, route switch, reconnect).
- Golden-config migration tests.

## Suggested Initial Directory Layout
- src/main.rs
- src/config/
- src/runtime/
- src/audio/
- src/integrations/
- src/observability/
- tests/e2e/
- tests/compat/
- docs/

## Compatibility Notes to Decide Early
- Which legacy config keys must be exact-compat vs alias-only.
- Minimum supported output backends for first production release.
- Default AP mode policy when both AP1 and AP2 are available.
- Policy for features present in backend but outside shairport-sync scope (video/HLS).

## High-Risk Items Requiring Early Prototypes
- AP2 timing behavior under real-world network jitter.
- ALSA/PipeWire latency and drift behavior under load.
- D-Bus/MPRIS command semantics expected by existing clients.

## Definition of Done for First Production Release
- AP1 and AP2 audio are production-stable for primary target platforms.
- Config migration guide published and validated against real sample configs.
- Core integrations (metadata output + at least one control interface) are reliable.
- Service packaging and operational runbook are complete.
