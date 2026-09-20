#!/usr/bin/env bash

if [ -z "${BASH_VERSION:-}" ]; then
    if command -v bash >/dev/null 2>&1; then
        exec bash "$0" "$@"
    fi
    echo '{"status":"failed","error":{"stage":"preflight","message":"this script requires bash"}}'
    exit 2
fi

set -Eeuo pipefail

PORT="${PORT:-5000}"
NAME="${NAME:-SSR Debian Smoke}"
BACKEND="${BACKEND:-null}"
LOG_FILE="${LOG_FILE:-/tmp/shairport-sync-rs-smoke.log}"
STARTUP_TIMEOUT_SECONDS="${STARTUP_TIMEOUT_SECONDS:-20}"
MDNS_TIMEOUT_SECONDS="${MDNS_TIMEOUT_SECONDS:-10}"
BUILD_FIRST="${BUILD_FIRST:-1}"
AP1_EXPECT_CN="${AP1_EXPECT_CN:-}"
AP1_EXPECT_ET="${AP1_EXPECT_ET:-}"
AIRPLAY_MODE="${AIRPLAY_MODE:-}"
AP1_CODECS="${AP1_CODECS:-}"
AP1_ENCRYPTION="${AP1_ENCRYPTION:-}"

APP_PID=""
STARTED="false"
PORT_OK="false"
MDNS_OK="false"
MDNS_NAME_OK="false"
MDNS_PORT_OK="false"
AP1_CN_OK="true"
AP1_ET_OK="true"
ERROR_STAGE=""
ERROR_MESSAGE=""
MDNS_MATCH_LINE=""
AP1_ACTUAL_CN=""
AP1_ACTUAL_ET=""

extract_txt_value() {
    local line="$1"
    local key="$2"
    awk -v key="$key" -F'"' '{
        for (i = 2; i <= NF; i += 2) {
            if (index($i, key "=") == 1) {
                sub("^" key "=", "", $i)
                print $i
                exit
            }
        }
    }' <<<"$line"
}

log() {
    echo "$*" >&2
}

require_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        ERROR_STAGE="preflight"
        ERROR_MESSAGE="missing command: $1"
        emit_json "failed"
        exit 1
    fi
}

json_escape() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    s="${s//$'\n'/\\n}"
    s="${s//$'\r'/\\r}"
    s="${s//$'\t'/\\t}"
    printf '%s' "$s"
}

emit_json() {
    local status="$1"
    local ended_at
    ended_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

    printf '{'
    printf '"status":"%s",' "$(json_escape "$status")"
    printf '"checks":{'
    printf '"started":%s,' "$STARTED"
    printf '"port_listening":%s,' "$PORT_OK"
    printf '"mdns_seen":%s,' "$MDNS_OK"
    printf '"mdns_name_match":%s,' "$MDNS_NAME_OK"
    printf '"mdns_port_match":%s,' "$MDNS_PORT_OK"
    printf '"ap1_cn_match":%s,' "$AP1_CN_OK"
    printf '"ap1_et_match":%s' "$AP1_ET_OK"
    printf '},'
    printf '"config":{'
    printf '"port":%s,' "$(json_escape "$PORT")"
    printf '"name":"%s",' "$(json_escape "$NAME")"
    printf '"backend":"%s",' "$(json_escape "$BACKEND")"
    printf '"airplay_mode":"%s",' "$(json_escape "$AIRPLAY_MODE")"
    printf '"ap1_codecs":"%s",' "$(json_escape "$AP1_CODECS")"
    printf '"ap1_encryption":"%s",' "$(json_escape "$AP1_ENCRYPTION")"
    printf '"build_first":%s,' "$( [[ "$BUILD_FIRST" == "1" ]] && echo true || echo false )"
    printf '"ap1_expect_cn":"%s",' "$(json_escape "$AP1_EXPECT_CN")"
    printf '"ap1_expect_et":"%s"' "$(json_escape "$AP1_EXPECT_ET")"
    printf '},'
    printf '"artifacts":{'
    printf '"log_file":"%s",' "$(json_escape "$LOG_FILE")"
    printf '"mdns_match_line":"%s",' "$(json_escape "$MDNS_MATCH_LINE")"
    printf '"ap1_actual_cn":"%s",' "$(json_escape "$AP1_ACTUAL_CN")"
    printf '"ap1_actual_et":"%s"' "$(json_escape "$AP1_ACTUAL_ET")"
    printf '},'
    printf '"error":{'
    printf '"stage":"%s",' "$(json_escape "$ERROR_STAGE")"
    printf '"message":"%s"' "$(json_escape "$ERROR_MESSAGE")"
    printf '},'
    printf '"ended_at":"%s"' "$(json_escape "$ended_at")"
    printf '}\n'
}

cleanup() {
    if [[ -n "${APP_PID}" ]] && kill -0 "${APP_PID}" >/dev/null 2>&1; then
        kill -INT "${APP_PID}" >/dev/null 2>&1 || true
        wait "${APP_PID}" >/dev/null 2>&1 || true
    fi
}

trap cleanup EXIT

require_cmd cargo
require_cmd ss
require_cmd avahi-browse
require_cmd timeout

if [[ ! -f "Cargo.toml" ]]; then
    ERROR_STAGE="preflight"
    ERROR_MESSAGE="run from repository root; Cargo.toml not found"
    emit_json "failed"
    exit 1
fi

if [[ "$BUILD_FIRST" == "1" ]]; then
    log "Building project..."
    if ! cargo build >/dev/null 2>&1; then
        ERROR_STAGE="build"
        ERROR_MESSAGE="cargo build failed"
        emit_json "failed"
        exit 1
    fi
fi

log "Starting shairport-sync-rs on port ${PORT} with backend ${BACKEND}..."
: >"${LOG_FILE}"
run_args=(--backend "${BACKEND}" --name "${NAME}" --port "${PORT}")
if [[ -n "${AIRPLAY_MODE}" ]]; then
    run_args+=(--airplay-mode "${AIRPLAY_MODE}")
fi
if [[ -n "${AP1_CODECS}" ]]; then
    run_args+=(--ap1-codecs "${AP1_CODECS}")
fi
if [[ -n "${AP1_ENCRYPTION}" ]]; then
    run_args+=(--ap1-encryption "${AP1_ENCRYPTION}")
fi
RUST_LOG="${RUST_LOG:-info}" cargo run -- "${run_args[@]}" >"${LOG_FILE}" 2>&1 &
APP_PID=$!
STARTED="true"

for ((i = 0; i < STARTUP_TIMEOUT_SECONDS; i++)); do
    if ! kill -0 "${APP_PID}" >/dev/null 2>&1; then
        ERROR_STAGE="startup"
        ERROR_MESSAGE="server exited before opening port"
        emit_json "failed"
        exit 1
    fi

    if ss -ltn | awk '{print $4}' | grep -Eq "(^|:)${PORT}$"; then
        PORT_OK="true"
        break
    fi

    sleep 1
done

if [[ "$PORT_OK" != "true" ]]; then
    ERROR_STAGE="port-check"
    ERROR_MESSAGE="port ${PORT} did not open within ${STARTUP_TIMEOUT_SECONDS}s"
    emit_json "failed"
    exit 1
fi

log "Browsing for _raop._tcp via Avahi..."
BROWSE_OUTPUT="$(timeout "${MDNS_TIMEOUT_SECONDS}" avahi-browse -prt _raop._tcp 2>/dev/null || true)"
if [[ -z "${BROWSE_OUTPUT}" ]]; then
    ERROR_STAGE="mdns-browse"
    ERROR_MESSAGE="no avahi-browse output in ${MDNS_TIMEOUT_SECONDS}s"
    emit_json "failed"
    exit 1
fi
MDNS_OK="true"

escaped_name="${NAME//\\/\\\\}"
escaped_name="${escaped_name// /\\032}"

if grep -Fq "\\064${escaped_name};" <<<"${BROWSE_OUTPUT}"; then
    MDNS_NAME_OK="true"
fi
if grep -Eq "^=;.*;${PORT};" <<<"${BROWSE_OUTPUT}"; then
    MDNS_PORT_OK="true"
fi

MDNS_MATCH_LINE="$(grep '^=' <<<"${BROWSE_OUTPUT}" | grep -F "\\064${escaped_name};" | grep -E ";${PORT};" | head -n 1 || true)"

if [[ -n "${MDNS_MATCH_LINE}" ]]; then
    MDNS_NAME_OK="true"
    MDNS_PORT_OK="true"
fi

if [[ "$MDNS_NAME_OK" != "true" ]]; then
    ERROR_STAGE="mdns-assert"
    ERROR_MESSAGE="service name '${NAME}' not found in avahi-browse output"
    emit_json "failed"
    exit 1
fi

if [[ "$MDNS_PORT_OK" != "true" ]]; then
    ERROR_STAGE="mdns-assert"
    ERROR_MESSAGE="service port '${PORT}' not found in avahi-browse output"
    emit_json "failed"
    exit 1
fi

AP1_ACTUAL_CN="$(extract_txt_value "${MDNS_MATCH_LINE}" "cn")"
AP1_ACTUAL_ET="$(extract_txt_value "${MDNS_MATCH_LINE}" "et")"

if [[ -n "${AP1_EXPECT_CN}" && "${AP1_ACTUAL_CN}" != "${AP1_EXPECT_CN}" ]]; then
    AP1_CN_OK="false"
    ERROR_STAGE="mdns-ap1-cn"
    ERROR_MESSAGE="AP1 cn mismatch: expected '${AP1_EXPECT_CN}', got '${AP1_ACTUAL_CN:-<missing>}'"
    emit_json "failed"
    exit 1
fi

if [[ -n "${AP1_EXPECT_ET}" && "${AP1_ACTUAL_ET}" != "${AP1_EXPECT_ET}" ]]; then
    AP1_ET_OK="false"
    ERROR_STAGE="mdns-ap1-et"
    ERROR_MESSAGE="AP1 et mismatch: expected '${AP1_EXPECT_ET}', got '${AP1_ACTUAL_ET:-<missing>}'"
    emit_json "failed"
    exit 1
fi

cleanup
APP_PID=""
emit_json "passed"
