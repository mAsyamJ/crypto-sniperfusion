#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
echo "SniperFusion harness quickstart (offline; no blockchain sends)"
bash "$SCRIPT_DIR/preflight.sh" "$ROOT"
echo
echo "=== Independent harness unit tests ==="
python3 -m unittest discover -s "$ROOT/harness/tests" -v
echo
echo "=== Synthetic paper evaluation (not profit evidence) ==="
python3 "$SCRIPT_DIR/evaluate_paper.py" "$ROOT/harness/fixtures/example_paper_trades.jsonl"
echo
echo "=== Shell script syntax ==="
bash -n "$SCRIPT_DIR"/*.sh
echo
echo "=== Next engineering agent mission ==="
echo "$ROOT/harness/docs/AGENT_NEXT_GOAL.md"
echo "Read it with: cat harness/docs/AGENT_NEXT_GOAL.md"
echo "No funded wallets or live execution were used."
