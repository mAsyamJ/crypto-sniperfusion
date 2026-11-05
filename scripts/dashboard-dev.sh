#!/usr/bin/env bash
# SniperFusion Control Tower one-command launcher (DASH-03).
# Starts the read-only Rust API (loopback 127.0.0.1:7433) with a fresh
# projection rebuild, then the React dev server (127.0.0.1:5173) which
# proxies /api to it. Ctrl-C stops both.
set -euo pipefail
cd "$(dirname "$0")/.."

API_ADDR=127.0.0.1:7433
DB=harness/runs/dashboard-cache/projection.sqlite3
ROOTS="harness/runs/wallet-alpha:harness/runs/launch-cohort:harness/runs:harness/runs/harness2"

echo "[dashboard-dev] building API (cargo)…"
CARGO_BUILD_JOBS=2 cargo build -p sniper-dashboard-api

cleanup() {
  [[ -n "${API_PID:-}" ]] && kill "$API_PID" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

echo "[dashboard-dev] starting API on http://${API_ADDR} (rebuild projection)…"
./target/debug/sniper-dashboard-api --db "$DB" --roots "$ROOTS" --addr "$API_ADDR" --rebuild &
API_PID=$!

for i in $(seq 1 50); do
  if curl -sf "http://${API_ADDR}/api/v1/health" >/dev/null 2>&1; then break; fi
  sleep 0.2
done
echo "[dashboard-dev] API ready: http://${API_ADDR}/api/v1/health"

echo "[dashboard-dev] starting frontend dev server on http://127.0.0.1:5173 …"
cd apps/dashboard
export PATH="$HOME/.nvm/versions/node/$(ls "$HOME/.nvm/versions/node" | tail -1)/bin:$PATH"
exec npx vite --host 127.0.0.1 --port 5173
