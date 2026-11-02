#!/usr/bin/env bash
set -euo pipefail
endpoint="${SNF_FORK_RPC:-http://127.0.0.1:18899}"
[[ "$endpoint" == http://127.0.0.1:* || "$endpoint" == http://localhost:* ]] || { echo 'Refusing non-local RPC endpoint' >&2; exit 2; }
curl --fail --silent --show-error --max-time 5 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' "$endpoint" \
  | python3 -c 'import sys,json; d=json.load(sys.stdin); assert "result" in d and d["result"] == "ok",d; print("Local fork health: ok")'
