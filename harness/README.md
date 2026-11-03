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


## Research loop V0 (2026-11-03)

The [operational runbook](docs/RESEARCH_LOOP_RUNBOOK.md), [deep research review](docs/DEEP_RESEARCH_IMPLEMENTATION_REVIEW.md) and eight `../.agents/skills/*/SKILL.md` skills provide a safe multi-agent research workflow.

To run **synthetic offline** S00 no-trade + 10 proposed entry policies:

```bash
python3 -m unittest discover -s harness/tests -v
python3 harness/research_batch.py validate --events harness/fixtures/research/synthetic_lifecycle.jsonl
python3 harness/research_batch.py batch --events harness/fixtures/research/synthetic_lifecycle.jsonl --out harness/runs/local-demo.json
python3 harness/research_batch.py diagnose --report harness/runs/local-demo.json
```

The script is not a calibrated Solana paper broker, historical market replay, execution-quality simulator, or source of profitable strategy evidence. It cannot sign or broadcast and refuses `LIVE_TRADING_ENABLED`. Existing H4 `snf-harness` controls remain the authority for future evidence gates; its `shadow` still honestly returns NOT_STARTED.

## H5 candidate: experimental Devnet pilot and iteration controller

Read `docs/DEVNET_EXECUTION_PILOT.md` and `harness/docs/RESEARCH_CYCLE_V1.md` from repository root.

Run `python3 harness/research_cycle.py --events harness/fixtures/research/synthetic_lifecycle.jsonl --out harness/runs/three-rounds.json --cycles 3` to test a bounded synthetic strategy iteration. All receipts are non-promotable and do not establish trading profit.

Run `python3 harness/cluster_probe.py --cluster devnet` for a read-only network/program check. The Rust `devnet-pump-pilot` binary offers optional, narrowly bounded Devnet program mechanics tests against a provided eligible Devnet mint. No Mainnet transactions or autonomous sniping are enabled. Keep test signer files private and outside git.

## H5–H8 agent mission coordinator

Read `harness/docs/MASTER_AGENT_MISSION_H5_H8.md` for the complete agent prompt and `harness/docs/H5_H8_GOAL_PLAN.md` for the milestone gates. From repo root run `python3 harness/agent_mission.py plan`, then `python3 harness/agent_mission.py run --events harness/fixtures/research/synthetic_lifecycle.jsonl --cycles 3 --out harness/runs/mission-001.json`. The controller uses synthetic data and never signs or broadcasts. It produces immutable, non-promotable evidence receipts.
