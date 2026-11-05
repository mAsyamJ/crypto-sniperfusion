# AGENTS.md — engineering rules for this repository

Read `AGENT_MASTER_PROMPT.md` (authoritative spec) and the approved amendments. This file binds every agent/session working here.

## Non-negotiable safety

1. `LIVE_TRADING_ENABLED=false` default; live mode needs 4 independent gates (feature + operator token + validated budget + kill switch). Never weaken.
2. No funded keys, no mainnet broadcast, no paid infra, no push, no force-anything, without explicit owner authorization recorded in the task.
3. Tests use ephemeral/fixture keys only. Secrets never enter logs, errors, commits, or fixtures.
4. Observer/replay/paper runtimes must remain structurally incapable of broadcasting (capability types, compile-time).

## Licensing (ADR-0002)

- chainstack (Apache-2.0): read for behavior, reimplement originally; keep attribution in `docs/UPSTREAM_MANIFEST.md`.
- yuto / m8s / mev-bot: **clean-room only** — facts (IDs, endpoints, thresholds) may inform; code/structures/flow may not be copied or transliterated. Never build or run them.
- Protocol behavior comes from `upstream/pump-docs` + on-chain fixtures, not from any sniper repo.

## Engineering loop (every milestone, in order)

DISCOVER → SPECIFY ACCEPTANCE → WRITE FAILING TEST → IMPLEMENT → `cargo fmt` → `cargo clippy --all-targets --all-features -- -D warnings` → unit/integration tests → SVM/replay tests where applicable → fault-injection where applicable → benchmark where applicable → REVIEW (diff, secrets, licenses) → EVIDENCE (`docs/evidence/M{n}.md`: files, tests, commands, exit codes, measurements, open defects) → UPDATE `docs/PROJECT_STATUS.md` + capability matrix → **local commit** (conventional message, never push).

Failure discipline: a failing gate blocks the milestone; root-cause, fix, re-run. Never mark unexecuted tests passed; never edit expected fixtures to go green without documented protocol-source justification.

## Environment

- Rust toolchain rustc 1.99.1 (rustup); workspace pins in `Cargo.toml` + lockfile; solana family = Generation 2 (ADR-0003). Any version bump = regression event.
- RAM-constrained host: `CARGO_BUILD_JOBS=2` (drop to 1 on swap pressure); watch `free -h` during builds.
- Prefix shell commands with `rtk` (operator proxy) where available.
- Upstream repos under `upstream/` are read-only and gitignored; the chainstack oracle venv lives inside that checkout.

## Where things are

`docs/` (all engineering docs + `evidence/`), `crates/` (workspace from M1), `fixtures/`, `config/`, `.github/workflows/` (CI from M1). Status of everything: `docs/PROJECT_STATUS.md`.

## Current milestone protocol

M0 → M9 per amended sequence (`AGENT_MASTER_PROMPT.md` §8 as amended). A milestone advances only when its gate in `docs/TEST_PLAN.md` passes for real. First vertical slice: complete Pump.fun/PumpSwap event → screening → simulated entry → position → simulated exit lifecycle, offline, fixture-reproducible.


## Research-specific agent skills and evidence routing (2026-11-03)

Use repository-local Agent Skills in `.agents/skills/*/SKILL.md` based on the task: `pump-protocol-forensics`, `memecoin-lifecycle-research`, `sniper-microstructure`, `sniper-quant-research`, `sniper-security-redteam`, `solana-execution-reliability`, `research-harness-conductor`, and `independent-strategy-critic`. These instructions are **playbooks**, not live capabilities.

New research flow: `harness/docs/RESEARCH_LOOP_RUNBOOK.md`. V0 `harness/research_batch.py` is a strictly offline **synthetic surrogate** and its outputs are permanently `promotable=false`. Do not feed them to the H4 `promote` command, label them as chain-execution evidence or use them to claim profitability. Only verified real-program/fork and real shadow sources may contribute to their respective gates. Candidate generators and independent critics must be separate roles. No AI process can access funded keys, submit transactions or change live strategy configurations.


## Devnet-only program pilot and bounded research loops (PR #6, 2026-11-04)

- Read \`docs/DEVNET_EXECUTION_PILOT.md\` before any test-network operations. There is **NO** support or authorization for agent-controlled mainnet transactions.
- Dedicated \`devnet-pump-pilot\` is a separate **Devnet-genesis-pinned** CLI that cannot acquire the production \`LiveCapability\`; it has a hard buy cap, mint and Token-2022 validation, simulation, private local faucet wallet, durable intent journal and at most one buy plus one sell. Never point to the default or funded Solana CLI signer, never share keys with AI.
- \`harness/cluster_probe.py\` provides strictly read-only program executability and genesis checks for Devnet/Testnet; Pump is documented on Devnet, not Testnet.
- \`harness/research_cycle.py\` uses **synthetic** inputs only, finite trial counts, preregistered parameter changes, adverse execution scenarios and a single sealed toy check. All receipts \`promotable=false\`. It is NOT a production backtester, automated LLM agent or a credible alpha claim.
- Skills: \`.agents/skills/devnet-protocol-pilot/SKILL.md\` and \`.agents/skills/strategy-experiment-orchestrator/SKILL.md\`.
- No automated signer connections, live snipes, paid RPC service activation or unbounded self-improvement loops. Real Devnet trade signature can only be claimed after operator executes and reconciles it.

## H5–H8 Master Mission (2026-11-04)

For new coding agents: read `harness/docs/MASTER_AGENT_MISSION_H5_H8.md`, `harness/docs/H5_H8_GOAL_PLAN.md` and `.agents/skills/sniperfusion-mission-director/SKILL.md` before beginning any research/Devnet work. The local `python3 harness/agent_mission.py plan` and `python3 harness/agent_mission.py run --events harness/fixtures/research/synthetic_lifecycle.jsonl --cycles 3 --out harness/runs/mission-001.json` are deliberately **offline, synthetic, and not promotable**. Do not confuse an unverified gate with a PASS. Never give the offline agent runner a funded key or a Devnet pilot signer, and never call `devnet-pump-pilot --submit` automatically. A separate operator explicitly approves at most one bounded Devnet protocol test, not mainnet sniping.


## Current operating entrypoint (H7 merged; reviewed 2026-11-04)

**Precedence:** direct operator instructions > safety restrictions in this file > `docs/AGENT_OPERATIONS.md` > specialist skill playbooks > archived milestone prompts. Read `docs/PROJECT_STATUS.md` and evidence at the current `git rev-parse HEAD` before treating a statement as implemented.

Start with `python3 harness/agentctl.py doctor`, `python3 harness/agentctl.py skills`, and `python3 harness/agentctl.py commands`. This is a *read-only, offline* dispatch tool, NOT an autonomous agent, broadcaster, key reader or substitute for `harness/snf-harness`.

Specialist routing: `sniperfusion-maintainer` owns repository discipline; `solana-rust-engineer` owns Rust/IDL/SVM correctness; `memecoin-trader-research` owns realistic trade hypotheses; `cohort-coverage-engineer` owns unbiased G0 collection; `h8-research-council` owns preregistration, trial budgets and independent criticism. Existing 11 skills remain active and are consulted as appropriate.

**Current facts:** H6/H7 source is merged; G0 COMPLETE-coverage mainnet cohort NOT established; H3 >=7-day shadow NOT started; Engine B lacks verified real snapshots and fee provenance; synthetic S00 NO_TRADE beat S01-S10. The legacy `harness/agent_mission.py plan` and older H5-H8 documents have stale implementation-state strings; treat `docs/evidence/H6.md`, `docs/evidence/H7.md` and actual code as the present evidence, not old planning tables.

No AI job may read/show RPC secrets or wallet key material, change the operator's credential, spend funds, run `--submit`, change public/private visibility, or promote a synthetic result to live execution. Repository-local skills are documentation and do not grant external tool permissions. Use official Foundation Solana development skills only after reviewing and pinning the external source; never execute external install scripts as a side effect of reading skill instructions.

The next bounded work package is in `docs/NEXT_GOAL_H8_G0.md`. Preserve existing code and do not declare a gate green without a dated, independently reproducible receipt.

## Profit-first H8 amendment (2026-11-04)

The owner priority is **net realizable P&L after all trading/operating costs and risk controls**. Read `docs/PROFIT_FIRST_GOAL_PLAN.md` and `harness/docs/PROFIT_FIRST_AGENT_MISSION.md`; invoke `.agents/skills/sniper-pnl-architect/SKILL.md` for financial reporting or strategy selection. Reuse existing H7 independent oracle (`sniper-strategy/src/oracle.rs`) and evaluator; do not rewrite or double-count. Separate synthetic/fork/shadow/confirmed trading receipts, ensure realistic executable exit marks, isolate external deposits/withdrawals and SOL/USD beta, compare matched S00 NO_TRADE and delayed-entry strategies, and fail closed on missing real fees/reserves. H8 must seek evidence for positive net edge, not promise profits. The P&L V2 requirements are not implemented merely because this plan exists.

## Wallet Alpha integration (2026-11-04)

Read `docs/WALLET_ALPHA_UPSTREAM_AUDIT.md`, `docs/WALLET_ALPHA_GOAL_PLAN.md`, and `harness/docs/WALLET_ALPHA_AGENT_MISSION.md` before wallet-copy, social hype or strategy alpha work. Use `.agents/skills/wallet-alpha-solana-tracker/SKILL.md`. Preserve Rust PF1 `oracle.rs` as the sole portfolio accounting truth; the offline Python wallet-alpha lab is a research contract test, not a second financial ledger. Treat successful leader trades as *candidate* signals, not guaranteed follower gains, and do not reveal/rotate existing RPC credentials.
