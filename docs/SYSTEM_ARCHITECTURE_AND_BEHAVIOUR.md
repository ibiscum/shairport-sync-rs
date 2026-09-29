# shairport-sync-rs System Architecture and Behaviour

## Purpose
This document describes the current runtime architecture and behaviour of shairport-sync-rs, with emphasis on diagnostics and log behaviour.

## Scope and Boundaries
shairport-sync-rs is the application shell and operational layer.
shairplay-rust provides core AirPlay protocol and stream logic.

Implemented primarily in this repository:
- Process lifecycle and startup/shutdown orchestration
- Configuration loading, precedence, and validation
- Audio backend routing and output path
- Activity monitor and diagnostics integration
- Operator-facing logging setup and controls

Provided by shairplay-rust backend:
- RAOP server lifecycle internals
- AP1/AP2 protocol handling
- Session callbacks for audio/control/metadata events

## Module Architecture
- src/main.rs
  - CLI entrypoint
  - Effective config load
  - tracing initialization
  - RAOP server builder wiring
  - runtime run loop and shutdown snapshot
- src/config/mod.rs
  - CLI model
  - TOML file model (flat + sectioned)
  - environment model (SSR_* and SSR_DIAGNOSTICS_*)
  - precedence merge and validation
- src/audio/mod.rs
  - AudioHandler implementation
  - backend session construction (null/stdout/pipe/alsa)
  - volume handling and callback-level protocol logging hooks
- src/observability/mod.rs
  - ActivityMonitor counters and snapshots
  - periodic logging task
  - JSONL snapshot writer
  - playback-state indicator (streaming_now)
- src/runtime/mod.rs
  - Ctrl+C signal handling and orderly stop

## Runtime Behaviour
### Startup
1. Parse CLI arguments.
2. Load and merge config using precedence:
   defaults < file < environment < CLI.
3. Initialize tracing based on diagnostics/log settings.
4. Build ActivityMonitor and optional periodic snapshot task.
5. Build RAOP server using config mapping.
6. Start server and enter runtime loop.

### Shutdown
1. On Ctrl+C, stop server.
2. Emit final activity snapshot (reason="shutdown").
3. Abort periodic snapshot task.

## Audio and Session Behaviour
### Session lifecycle
- on audio_init:
  - session started counter increments
  - backend session is created
- on audio_flush:
  - flush counters increment
  - session is not ended by flush
- on session object drop:
  - active session counter decrements

This avoids false drops of active_sessions during normal flushes/transitions.

### Playback signal
- streaming_now is computed from recent audio callbacks.
- It is true when at least one callback has been observed within a 5 second window.
- It is included in activity logs and JSONL snapshots.

## Observability and Snapshot Metrics
Activity snapshots include, among others:
- started_sessions
- active_sessions
- streaming_now
- connected_clients
- audio_callbacks and audio_samples_in
- audio_flushes
- backend_samples_out, backend_write_errors, backend_recoveries
- ALSA recovery/underrun/buffer/latency counters
- sender action counters (connect/disconnect/reconnect/volume/metadata/flush)
- gapless_transitions

## Log Behaviour
## Default verbosity mapping
The diagnostics log_verbosity value maps to default tracing filters as follows:

- 0 -> info,shairplay=info
- 1 -> debug,shairplay=debug
- 2 -> trace,shairplay=trace
- 3 -> trace,shairplay=trace

Notes:
- Level 1 is less verbose than levels 2 and 3.
- Levels 2 and 3 share the same default filter.

## Level 3 protocol diagnostics
Additional protocol-focused logs are explicitly gated at log_verbosity >= 3.
These logs are intended for interoperability troubleshooting and mirror upstream high-verbosity workflows.

At level 3, shairport-sync-rs emits additional target=protocol events for:
- startup protocol summary
- AP1 mDNS advertisement intent summary
- parameter/control exchange callbacks:
  - set volume
  - metadata update
  - client connected
  - client disconnected

## log_filter override behaviour
If log_filter (or SSR_LOG_FILTER) is set, it overrides the default filter derived from log_verbosity.

Implications:
- log_verbosity still controls level-3-gated protocol branches.
- log_filter can still suppress those emitted events by level or target.

## Diagnostics controls interacting with log output
- log_format: text or json
- log_show_file_and_line: toggles source file/line fields
- log_show_time_since_startup: uptime-style timestamps
- log_show_time_since_last_message: delta-style timestamps

If both time_since_startup and time_since_last_message are enabled, delta-style timestamps are used.

## Current known behaviour and limitations
- Some diagnostics keys are parsed and validated for compatibility but not yet wired to protocol behaviour:
  - disable_resend_requests
  - drop_this_fraction_of_audio_packets
  - retain_cover_art
  - get_plist_metadata
- Syslog/journal parity behaviours are not yet fully aligned with upstream.

## Operator Guidance
- Use log_verbosity=1 for routine troubleshooting.
- Use log_verbosity=2 for deeper internal tracing.
- Use log_verbosity=3 when investigating protocol compatibility issues, mDNS advertisement shape, or callback-level control exchange.
- Prefer explicit log_filter for precise target scoping in long-running sessions.

## Related Documents
- docs/PARITY_MATRIX.md
- docs/PORTING_ROADMAP.md
- README.md
