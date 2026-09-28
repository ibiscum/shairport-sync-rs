#!/usr/bin/env bash

if [ -z "${BASH_VERSION:-}" ]; then
    if command -v bash >/dev/null 2>&1; then
        exec bash "$0" "$@"
    fi
    echo "This script requires bash." >&2
    exit 2
fi

set -Eeuo pipefail

ITERATIONS="${ITERATIONS:-10}"
HOLD_SECONDS="${HOLD_SECONDS:-30}"
BASE_PORT="${BASE_PORT:-5600}"
NAME_PREFIX="${NAME_PREFIX:-SSR AP1 Soak}"
BACKEND="${BACKEND:-null}"
BUILD_FIRST="${BUILD_FIRST:-1}"
STARTUP_TIMEOUT_SECONDS="${STARTUP_TIMEOUT_SECONDS:-20}"
MDNS_TIMEOUT_SECONDS="${MDNS_TIMEOUT_SECONDS:-10}"
ACTIVITY_INTERVAL_SECS="${ACTIVITY_INTERVAL_SECS:-5}"
ARTIFACT_DIR="${ARTIFACT_DIR:-/tmp/shairport-sync-rs-ap1-soak}"

mkdir -p "${ARTIFACT_DIR}"

require_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Missing required command: $1" >&2
        exit 1
    fi
}

require_cmd bash
require_cmd python3

if [[ ! -f "Cargo.toml" ]]; then
    echo "Run this script from the repository root (Cargo.toml not found)." >&2
    exit 1
fi

passed=0
failed=0

for ((i = 1; i <= ITERATIONS; i++)); do
    port=$((BASE_PORT + i))
    name="${NAME_PREFIX} ${i}"
    run_dir="${ARTIFACT_DIR}/run-${i}"
    mkdir -p "${run_dir}"

    log_file="${run_dir}/server.log"
    snapshot_file="${run_dir}/activity.jsonl"

    echo "[soak] iteration ${i}/${ITERATIONS}: port=${port}, hold=${HOLD_SECONDS}s"

    set +e
    json_output="$(
        PORT="${port}" \
        NAME="${name}" \
        BACKEND="${BACKEND}" \
        BUILD_FIRST="${BUILD_FIRST}" \
        STARTUP_TIMEOUT_SECONDS="${STARTUP_TIMEOUT_SECONDS}" \
        MDNS_TIMEOUT_SECONDS="${MDNS_TIMEOUT_SECONDS}" \
        HOLD_SECONDS="${HOLD_SECONDS}" \
        AIRPLAY_MODE="ap1" \
        AP1_CODECS="pcm,alac" \
        AP1_ENCRYPTION="none" \
        AP1_EXPECT_CN="0,1" \
        AP1_EXPECT_ET="0" \
        ACTIVITY_INTERVAL_SECS="${ACTIVITY_INTERVAL_SECS}" \
        ACTIVITY_SNAPSHOT_PATH="${snapshot_file}" \
        LOG_FILE="${log_file}" \
        bash scripts/debian-smoke-test-json.sh
    )"
    rc=$?
    set -e

    printf '%s\n' "${json_output}" >"${run_dir}/result.json"

    status="$(python3 -c 'import json,sys
try:
    data=json.loads(sys.stdin.read())
    print(data.get("status","failed"))
except Exception:
    print("failed")' <<<"${json_output}")"

    if [[ ${rc} -eq 0 && "${status}" == "passed" ]]; then
        passed=$((passed + 1))
        echo "[soak] iteration ${i}: passed"
    else
        failed=$((failed + 1))
        stage="$(python3 -c 'import json,sys
try:
    data=json.loads(sys.stdin.read())
    print(data.get("error",{}).get("stage","unknown"))
except Exception:
    print("invalid-json")' <<<"${json_output}")"
        message="$(python3 -c 'import json,sys
try:
    data=json.loads(sys.stdin.read())
    print(data.get("error",{}).get("message","unknown"))
except Exception:
    print("invalid-json")' <<<"${json_output}")"
        echo "[soak] iteration ${i}: failed at stage=${stage} message=${message}"
    fi

    BUILD_FIRST=0

done

summary_file="${ARTIFACT_DIR}/summary.json"
python3 - <<'PY' "${summary_file}" "${ITERATIONS}" "${passed}" "${failed}" "${HOLD_SECONDS}" "${ARTIFACT_DIR}"
import json
import sys

summary = {
    "iterations": int(sys.argv[2]),
    "passed": int(sys.argv[3]),
    "failed": int(sys.argv[4]),
    "hold_seconds": int(sys.argv[5]),
    "artifact_dir": sys.argv[6],
}

with open(sys.argv[1], "w", encoding="utf-8") as f:
    json.dump(summary, f, indent=2)

print(json.dumps(summary))
PY

if [[ ${failed} -ne 0 ]]; then
    echo "[soak] failures detected. See ${summary_file} and per-run result.json files." >&2
    exit 1
fi

echo "[soak] all iterations passed. Summary: ${summary_file}"
