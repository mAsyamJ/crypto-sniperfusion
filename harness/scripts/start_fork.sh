#!/usr/bin/env bash
set -euo pipefail
# Surfpool local simulated network: no upstream sends or funded signing.
# Binary resolution: SNF_SURFPOOL_BIN > harness/bin/surfpool (pinned, see
# harness/docs/surfpool-pin.md) > whatever `surfpool` is on PATH.
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
surfpool_bin="${SNF_SURFPOOL_BIN:-}"
if [[ -z "$surfpool_bin" ]]; then
  if [[ -x "$root/harness/bin/surfpool" ]]; then
    surfpool_bin="$root/harness/bin/surfpool"
  elif command -v surfpool >/dev/null; then
    surfpool_bin="$(command -v surfpool)"
  else
    echo 'Install and verify Surfpool first: harness/docs/surfpool-pin.md' >&2
    exit 2
  fi
fi
: "${SNF_SOURCE_RPC:?Set SNF_SOURCE_RPC to your permitted READ-ONLY upstream Solana RPC endpoint}"
export SNF_FORK_RPC='http://127.0.0.1:18899'
if [[ "$SNF_SOURCE_RPC" == "$SNF_FORK_RPC" ]]; then echo 'Source and fork RPC must differ' >&2; exit 3; fi
if [[ "${LIVE_TRADING_ENABLED:-false}" != 'false' ]]; then echo 'Live trading must be disabled' >&2; exit 3; fi
if [[ -n "${SOLANA_PRIVATE_KEY:-}" || -n "${PRIVATE_KEY:-}" ]]; then echo 'Refusing with private-key environment variables' >&2; exit 3; fi
echo "Starting isolated local Surfpool on 127.0.0.1:18899 via $surfpool_bin (mainnet used only for state reads)"
exec "$surfpool_bin" start --ci --host 127.0.0.1 --port 18899 --ws-port 18900 --rpc-url "$SNF_SOURCE_RPC"
