#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:-$HOME/project/crypto-bot}"
cd "$ROOT"
test -f Cargo.toml && test -f AGENTS.md && test -d crates || { echo 'FAIL: not the SniperFusion root' >&2; exit 2; }
printf 'HEAD: '; git rev-parse HEAD
printf 'REMOTE: '; git remote get-url origin || true
if [[ -n "$(git status --porcelain)" ]]; then echo 'WARN: working tree not clean (not modified by this script)' >&2; fi
if [[ "${LIVE_TRADING_ENABLED:-false}" != 'false' ]]; then echo 'FAIL: live trading must be disabled' >&2; exit 3; fi
if [[ -n "${SOLANA_PRIVATE_KEY:-}" || -n "${PRIVATE_KEY:-}" ]]; then echo 'FAIL: private-key env vars present (values not logged)' >&2; exit 3; fi
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
printf '\n=== Workspace offline checks ===\n'
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
printf '\nPASS: offline baseline checks. NOT a proof of fork or live execution.\n'
