#!/usr/bin/env bash

if [ -z "${BASH_VERSION:-}" ]; then
    if command -v bash >/dev/null 2>&1; then
        exec bash "$0" "$@"
    fi
    echo "This script requires bash." >&2
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

APP_PID=""

require_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Missing required command: $1" >&2
        exit 1
    fi
}

cleanup() {
    local exit_code=$?

    if [[ -n "${APP_PID}" ]] && kill -0 "${APP_PID}" >/dev/null 2>&1; then
        kill -INT "${APP_PID}" >/dev/null 2>&1 || true
        wait "${APP_PID}" >/dev/null 2>&1 || true
    fi

    if [[ ${exit_code} -ne 0 ]]; then
        echo
        echo "Smoke test failed. Last server log lines:"
        if [[ -f "${LOG_FILE}" ]]; then
            tail -n 50 "${LOG_FILE}" || true
        else
            echo "No log file at ${LOG_FILE}"
        fi
    fi

    exit "${exit_code}"
}

trap cleanup EXIT

require_cmd cargo
require_cmd ss
require_cmd avahi-browse
require_cmd timeout

if [[ ! -f "Cargo.toml" ]]; then
    echo "Run this script from the repository root (Cargo.toml not found)." >&2
    exit 1
fi

if [[ "${BUILD_FIRST}" == "1" ]]; then
    echo "Building project..."
    cargo build >/dev/null
fi

echo "Starting shairport-sync-rs on port ${PORT} with backend ${BACKEND}..."
: >"${LOG_FILE}"
RUST_LOG="${RUST_LOG:-info}" cargo run -- --backend "${BACKEND}" --name "${NAME}" --port "${PORT}" >"${LOG_FILE}" 2>&1 &
APP_PID=$!

port_ok=0
for ((i = 0; i < STARTUP_TIMEOUT_SECONDS; i++)); do
    if ! kill -0 "${APP_PID}" >/dev/null 2>&1; then
        echo "Server process exited during startup." >&2
        exit 1
    fi

    if ss -ltn | awk '{print $4}' | grep -Eq "(^|:)${PORT}$"; then
        port_ok=1
        break
    fi

    sleep 1
done

if [[ ${port_ok} -ne 1 ]]; then
    echo "Server did not open TCP port ${PORT} within ${STARTUP_TIMEOUT_SECONDS}s." >&2
    exit 1
fi

echo "Port check passed: TCP ${PORT} is listening."

echo "Browsing for _raop._tcp service via Avahi..."
BROWSE_OUTPUT="$(timeout "${MDNS_TIMEOUT_SECONDS}" avahi-browse -prt _raop._tcp 2>/dev/null || true)"

if [[ -z "${BROWSE_OUTPUT}" ]]; then
    echo "No mDNS browse output captured in ${MDNS_TIMEOUT_SECONDS}s." >&2
    exit 1
fi

escaped_name="${NAME//\\/\\\\}"
escaped_name="${escaped_name// /\\032}"

if ! grep -Fq "\\064${escaped_name};" <<<"${BROWSE_OUTPUT}"; then
    echo "Did not find service name '${NAME}' in Avahi output." >&2
    exit 1
fi

if ! grep -Eq "^=;.*;${PORT};" <<<"${BROWSE_OUTPUT}"; then
    echo "Did not find service port '${PORT}' in Avahi output." >&2
    exit 1
fi

echo "mDNS check passed: found '${NAME}' on port ${PORT}."
echo "Stopping server cleanly..."

kill -INT "${APP_PID}" >/dev/null 2>&1 || true
wait "${APP_PID}" >/dev/null 2>&1 || true
APP_PID=""

echo "Smoke test passed."
