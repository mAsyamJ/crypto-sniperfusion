#!/usr/bin/env bash
set -euo pipefail
# Surfpool local simulated network: no upstream sends or funded signing.
if ! command -v surfpool >/dev/null; then
 echo 'Install and verify Surfpool first: https://github.com/solana-foundation/surfpool' >&2
 exit 2
fi
: "${SNF_SOURCE_RPC:?Set SNF_SOURCE_RPC to your permitted READ-ONLY upstream Solana RPC endpoint}"
export SNF_FORK_RPC='http://127.0.0.1:18899'
if [[ "$SNF_SOURCE_RPC" == "$SNF_FORK_RPC" ]]; then echo 'Source and fork RPC must differ' >&2; exit 3; fi
if [[ "${LIVE_TRADING_ENABLED:-false}" != 'false' ]]; then echo 'Live trading must be disabled' >&2; exit 3; fi
if [[ -n "${SOLANA_PRIVATE_KEY:-}" || -n "${PRIVATE_KEY:-}" ]]; then echo 'Refusing with private-key environment variables' >&2; exit 3; fi
echo 'Starting isolated local Surfpool on 127.0.0.1:18899 (mainnet used only for state reads)'
exec surfpool start --ci --host 127.0.0.1 --port 18899 --ws-port 18900 --rpc-url "$SNF_SOURCE_RPC"
