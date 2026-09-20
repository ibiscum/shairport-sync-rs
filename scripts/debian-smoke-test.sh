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
AP1_EXPECT_CN="${AP1_EXPECT_CN:-}"
AP1_EXPECT_ET="${AP1_EXPECT_ET:-}"
AIRPLAY_MODE="${AIRPLAY_MODE:-}"
AP1_CODECS="${AP1_CODECS:-}"
AP1_ENCRYPTION="${AP1_ENCRYPTION:-}"
AP2_PIN="${AP2_PIN:-}"
AP2_PAIRING_STORE_PATH="${AP2_PAIRING_STORE_PATH:-}"
AP2_EXPECT_FEATURES="${AP2_EXPECT_FEATURES:-}"
AP2_EXPECT_FLAGS="${AP2_EXPECT_FLAGS:-}"

APP_PID=""
MDNS_MATCH_LINE=""
AIRPLAY_MATCH_LINE=""

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
if [[ -n "${AP2_PIN}" ]]; then
    run_args+=(--ap2-pin "${AP2_PIN}")
fi
if [[ -n "${AP2_PAIRING_STORE_PATH}" ]]; then
    run_args+=(--ap2-pairing-store-path "${AP2_PAIRING_STORE_PATH}")
fi
RUST_LOG="${RUST_LOG:-info}" cargo run -- "${run_args[@]}" >"${LOG_FILE}" 2>&1 &
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

MDNS_MATCH_LINE="$(grep '^=' <<<"${BROWSE_OUTPUT}" | grep -F "\\064${escaped_name};" | grep -E ";${PORT};" | head -n 1 || true)"

if [[ -z "${MDNS_MATCH_LINE}" ]]; then
    echo "Did not find resolved Avahi entry matching name '${NAME}' and port '${PORT}'." >&2
    exit 1
fi

echo "mDNS check passed: found '${NAME}' on port ${PORT}."

if [[ -n "${AP1_EXPECT_CN}" ]]; then
    actual_cn="$(extract_txt_value "${MDNS_MATCH_LINE}" "cn")"
    if [[ "${actual_cn}" != "${AP1_EXPECT_CN}" ]]; then
        echo "AP1 cn mismatch: expected '${AP1_EXPECT_CN}', got '${actual_cn:-<missing>}'" >&2
        exit 1
    fi
    echo "AP1 cn check passed: ${actual_cn}"
fi

if [[ -n "${AP1_EXPECT_ET}" ]]; then
    actual_et="$(extract_txt_value "${MDNS_MATCH_LINE}" "et")"
    if [[ "${actual_et}" != "${AP1_EXPECT_ET}" ]]; then
        echo "AP1 et mismatch: expected '${AP1_EXPECT_ET}', got '${actual_et:-<missing>}'" >&2
        exit 1
    fi
    echo "AP1 et check passed: ${actual_et}"
fi

if [[ "${AIRPLAY_MODE}" == "ap2" || -n "${AP2_EXPECT_FEATURES}" || -n "${AP2_EXPECT_FLAGS}" ]]; then
    echo "Browsing for _airplay._tcp service via Avahi..."
    AIRPLAY_BROWSE_OUTPUT="$(timeout "${MDNS_TIMEOUT_SECONDS}" avahi-browse -prt _airplay._tcp 2>/dev/null || true)"
    if [[ -z "${AIRPLAY_BROWSE_OUTPUT}" ]]; then
        echo "No _airplay._tcp output captured in ${MDNS_TIMEOUT_SECONDS}s." >&2
        exit 1
    fi

    airplay_escaped_name="${NAME//\\/\\\\}"
    airplay_escaped_name="${airplay_escaped_name// /\\032}"
    AIRPLAY_MATCH_LINE="$(grep '^=' <<<"${AIRPLAY_BROWSE_OUTPUT}" | grep -F ";${airplay_escaped_name};" | grep -E ";${PORT};" | head -n 1 || true)"

    if [[ -z "${AIRPLAY_MATCH_LINE}" ]]; then
        echo "Did not find resolved _airplay._tcp entry matching name '${NAME}' and port '${PORT}'." >&2
        exit 1
    fi

    if [[ -n "${AP2_EXPECT_FEATURES}" ]]; then
        actual_features="$(extract_txt_value "${AIRPLAY_MATCH_LINE}" "features")"
        if [[ "${actual_features}" != "${AP2_EXPECT_FEATURES}" ]]; then
            echo "AP2 features mismatch: expected '${AP2_EXPECT_FEATURES}', got '${actual_features:-<missing>}'" >&2
            exit 1
        fi
        echo "AP2 features check passed: ${actual_features}"
    fi

    if [[ -n "${AP2_EXPECT_FLAGS}" ]]; then
        actual_flags="$(extract_txt_value "${AIRPLAY_MATCH_LINE}" "flags")"
        if [[ "${actual_flags}" != "${AP2_EXPECT_FLAGS}" ]]; then
            echo "AP2 flags mismatch: expected '${AP2_EXPECT_FLAGS}', got '${actual_flags:-<missing>}'" >&2
            exit 1
        fi
        echo "AP2 flags check passed: ${actual_flags}"
    fi
fi

echo "Stopping server cleanly..."

kill -INT "${APP_PID}" >/dev/null 2>&1 || true
wait "${APP_PID}" >/dev/null 2>&1 || true
APP_PID=""

echo "Smoke test passed."
