# SniperFusion — Engineering Audit and Simulation Harness

This directory contains the **2026-11-03 audit, next-goal agent prompt, and safe, offline-first evaluation scripts**. It is an add-on to the existing Rust project, not a completed live trading system.

Audited baseline: commit 2d51a32b4a480618483e2018d9f234d0906594e0. The project has advanced since this audit if main has moved; agents must inspect the current SHA before applying patches.

## Quick start (from repository root)

```bash
cd ~/project/crypto-bot
git pull --ff-only origin main
bash harness/scripts/preflight.sh .
python3 -m unittest discover -s harness/tests -v
python3 harness/scripts/evaluate_paper.py harness/fixtures/example_paper_trades.jsonl
cat harness/docs/AGENT_NEXT_GOAL.md
```

To try a **local Surfpool fork**, first install Surfpool from its official instructions (review the installer and verify the version). Configure a read-only upstream RPC *only in your shell environment*, never in git. Then run:

```bash
export SNF_SOURCE_RPC="YOUR_READ_ONLY_SOLANA_RPC"
bash harness/scripts/start_fork.sh
# In a second terminal, at repository root:
bash harness/scripts/fork_health.sh
```

This starts and checks a local fork. **It does not perform or validate a Pump.fun buy/sell.** That is the M3c engineering goal.

## Contents

- docs/ENGINEERING_AUDIT.md — findings against the audited GitHub head
- docs/HARNESS_WORKFLOW.md — continuous improvement harness design and gates
- docs/AGENT_NEXT_GOAL.md — ready-to-paste coding agent mission H0–H4
- scripts/preflight.sh — non-mutating Rust fmt/clippy/test checks
- scripts/start_fork.sh and scripts/fork_health.sh — local-only Surfpool startup and health
- scripts/evaluate_paper.py — stdlib-only evaluation of completed synthetic paper round trips
- tests/ — independent evaluator tests
- fixtures/ — SYNTHETIC records, not live-trading performance

## Restrictions

No live mainnet broadcasting, wallet secrets, paid infra, strategy promotion into live trading, or unstated Git pushes. Observer, replay and paper modes must remain incapable of broadcasting. Official protocol documentation and current program state supersede old snippets. Surfpool's fork is not historical block-order or MEV simulation.

## Project integration

Do not overwrite root docs/, scripts/ or fixtures/. The agent should implement production-grade harness functionality in Rust and wire it to the app, while retaining this self-contained handoff as reviewable evidence and a reference interface.

Source: https://github.com/mAsyamJ/crypto-sniperfusion
Surfpool CLI: https://solana.com/docs/tools/surfpool/toolchain/cli
