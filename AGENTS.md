# AGENTS.md — operating rules for this repository

**Read order for every session:** `SOUL.md` (doctrine — why we exist and what winning is) → this file (operations — how we work) → `docs/PROJECT_STATUS.md` (state — what exists today) → `AGENT_MASTER_PROMPT.md` (original build spec, historical where superseded).

**Precedence:** direct operator instructions > §2 safety restrictions in this file > `SOUL.md` doctrine > remaining sections of this file > `docs/AGENT_OPERATIONS.md` > specialist skill playbooks (`.agents/skills/`) > archived milestone prompts.

This file binds every agent/session working here. It is organized by function, not by date; history lives in `docs/PROJECT_STATUS.md` and `docs/evidence/`.

## 1. Mission and the definition of winning

The owner's goal is a **consistently high win rate in paper trading**. In this repository that goal is operationalized — not diluted — as follows (full doctrine: `SOUL.md` §2):

- **Consistent** means positive `incremental_net_edge` vs the S00 NO_TRADE baseline that replicates out-of-sample on `coverage_state=COMPLETE` cohorts, passes gates **G-A through G-G** (`SOUL.md` §7), and survives adverse stress.
- **High win rate** is pursued *through* verified expectancy and bounded tails (CVaR95). A challenger with a high hit-rate but negative or tail-blowing expectancy is **rejected** — a 90% win rate with unbounded tails is a delayed loss.
- Paper trading is the proving ground, not a rehearsal: forward orders, no lookahead, declared exit policies, every fee and failure counted.

Until a challenger passes all gates, **S00 NO_TRADE is the champion** and every other strategy is a hypothesis with a preregistered budget.

## 2. Non-negotiable safety

1. `LIVE_TRADING_ENABLED=false` default; live mode needs 4 independent gates (feature + operator token + validated budget + kill switch). Never weaken.
2. No funded keys, no mainnet broadcast, no paid infra, no push, no force-anything, without explicit owner authorization recorded in the task.
3. Tests use ephemeral/fixture keys only. Secrets never enter logs, errors, commits, fixtures, or LLM prompts.
4. Observer/replay/paper runtimes must remain structurally incapable of broadcasting (capability types, compile-time).
5. No AI job may read/show RPC secrets or wallet key material, change credentials, spend funds, run `--submit`, change repository visibility, or promote a synthetic result to live execution.

## 3. Licensing (ADR-0002)

- chainstack (Apache-2.0): read for behavior, reimplement originally; keep attribution in `docs/UPSTREAM_MANIFEST.md`.
- yuto / m8s / mev-bot: **clean-room only** — facts (IDs, endpoints, thresholds) may inform; code/structures/flow may not be copied or transliterated. Never build or run them.
- Protocol behavior comes from `upstream/pump-docs` + on-chain fixtures, not from any sniper repo.

## 4. Engineering loop (every milestone, in order)

DISCOVER → SPECIFY ACCEPTANCE → WRITE FAILING TEST → IMPLEMENT → `cargo fmt` → `cargo clippy --all-targets --all-features -- -D warnings` → unit/integration tests → SVM/replay tests where applicable → fault-injection where applicable → benchmark where applicable → REVIEW (diff, secrets, licenses) → EVIDENCE (`docs/evidence/{ID}.md`: files, tests, commands, exit codes, measurements, open defects) → UPDATE `docs/PROJECT_STATUS.md` + capability matrix → **local commit** (conventional message, never push).

Failure discipline: a failing gate blocks the milestone; root-cause, fix, re-run. Never mark unexecuted tests passed; never edit expected fixtures to go green without documented protocol-source justification.

## 5. Environment

- Rust toolchain rustc 1.99.1 (rustup); workspace pins in `Cargo.toml` + lockfile; solana family = Generation 2 (ADR-0003). Any version bump = regression event.
- RAM-constrained host: `CARGO_BUILD_JOBS=2` (drop to 1 on swap pressure); watch `free -h` during builds.
- Prefix shell commands with `rtk` (operator proxy) where available.
- Upstream repos under `upstream/` are read-only and gitignored; the chainstack oracle venv lives inside that checkout.

## 6. Where things are

- `SOUL.md` — doctrine: winning, Ten Laws, evidence anchors, strategy frontier, gates.
- `docs/` — engineering docs + `evidence/` receipts + `ADRS/`; status of everything: `docs/PROJECT_STATUS.md`.
- `crates/` — `sniper-core`, `sniper-protocols`, `sniper-runtime`, `sniper-strategy` (engines, oracle, evaluator), `sniper-app` (CLI).
- `harness/` — `agentctl.py` (dispatch), `snf-harness` (doctor/verify/replay/fork/shadow/evaluate/compare/promote), `launch_cohort_capture.py` (read-only G0 collector), `research_cycle.py` / `research_batch.py` (synthetic-only), `research_registry.py` (H8 preregistration), `wallet_alpha_lab.py`, `wallet_watch.py`, `rpc_capability_probe.py`, `cluster_probe.py`.
- `.agents/skills/` — 19 specialist playbooks (documentation, not live capabilities).
- `fixtures/`, `config/`, `.github/workflows/` (CI), `integrations/`, `scripts/`.

## 7. The research loop (operational steps — doctrine in SOUL.md §6)

One bounded cycle per session. Every step leaves a write-once receipt. Never promote from synthetic data.

1. **CAPTURE** — `python3 harness/launch_cohort_capture.py` (read-only mainnet). Accept only `coverage_state=COMPLETE` windows; zero-create windows are recorded as market facts (failure storms), never discarded. Provider `getBlock` gaps are `BLOCKED_RPC_CAPABILITY`, not green runs.
2. **RECONSTRUCT** — Engine B on real reserve/fee snapshots; refuse unobserved reserves and retroactive fee tiers; abstentions are reported, never silently priced.
3. **HYPOTHESIZE** — the trader role (`memecoin-trader-research`) proposes ONE falsifiable, family-tagged mechanism from the frontier (`SOUL.md` §5: S11 filtered entry, S12 filtered graduation, S13 slow mooner, S14 provenance-filtered follow, EX exit engine). The **rejected registry** (`SOUL.md` §5) may not be resubmitted without a materially new mechanism.
4. **PREREGISTER** — `harness/research_registry.py` before any evaluation: cohort definition, features, parameters, exit policy, kill criteria, trial budget; hash-chained; one-use holdout.
5. **PAPER-EXECUTE** — PaperBroker forward-order, no-lookahead, declared exit policy attached (entry-only challengers are refused at the registry).
6. **EVALUATE** — independent PF1 oracle + evaluator vs S00; creator/day-clustered bootstrap; CVaR95; adverse stress (+100% fees, +2 slots latency, 50% no-fill).
7. **CRITIC** — independent critic role produces a separate rejection/acceptance artifact; the holdout opens at most once.
8. **DECIDE** — promote to shadow-candidate / demote to rejected registry; update `docs/PROJECT_STATUS.md`; the receipt is the deliverable.

**Gate order:** a real COMPLETE cohort containing actual launches precedes any S11+ registration. Today G0 mechanics work but **zero successful launches have been captured** (the provider `getBlock` gap); until that is fixed, the registry must refuse new trials on real-data claims.

Synthetic rules: `research_cycle.py` / `research_batch.py` / `agentctl.py research-demo` outputs are permanently `promotable=false`; never feed them to the `promote` command, label them as chain-execution evidence, or use them to claim profitability. Candidate generators and independent critics are separate roles.

## 8. Specialist routing (19 skills; consult `.agents/skills/*/SKILL.md`)

- Repository discipline: `sniperfusion-maintainer`. Mission framing: `sniperfusion-mission-director`.
- Protocol/Rust correctness: `solana-rust-engineer`, `pump-protocol-forensics`.
- G0 unbiased collection: `cohort-coverage-engineer`.
- Strategy hypotheses by family: `memecoin-trader-research` (families S11–S14), `memecoin-lifecycle-research` (S13 survivor signals, death signals), `sniper-microstructure` (S12 migration windows, landing costs), `wallet-alpha-solana-tracker` (S14 leader provenance).
- Exit engineering (EX): `memecoin-lifecycle-research` + `sniper-quant-research`.
- Evaluation and statistics: `sniper-quant-research`, `independent-strategy-critic`, `strategy-experiment-orchestrator`, `h8-research-council` (preregistration, trial budgets, holdout).
- Financial truth: `sniper-pnl-architect` (dispatch for every P&L or strategy-selection report).
- Execution reliability (analysis only, no funded sends): `solana-execution-reliability`.
- Harness operations: `research-harness-conductor`, `harness2-integration` (deterministic channel only; model-backed remains BLOCKED behind the isolated worker).
- Red team: `sniper-security-redteam`. Devnet pilot: `devnet-protocol-pilot`.

Entry point: `python3 harness/agentctl.py doctor`, then `skills`, then `commands` — a read-only dispatch helper, NOT an autonomous agent or key reader.

## 9. Devnet-only program pilot

Read `docs/DEVNET_EXECUTION_PILOT.md` before any test-network operation. There is **NO** authorization for agent-controlled mainnet transactions. `devnet-pump-pilot` is Devnet-genesis-pinned, cannot acquire the production `LiveCapability`, and has a hard buy cap, mint/Token-2022 validation, simulation, a private local faucet wallet, and at most one buy plus one sell. Never point it at a funded CLI signer; never call `--submit` automatically; a real Devnet signature is claimed only after the operator executes and reconciles it. `harness/cluster_probe.py` is strictly read-only.

## 10. Wallet Alpha (leader-follow) work

Read `docs/WALLET_ALPHA_UPSTREAM_AUDIT.md`, `docs/WALLET_ALPHA_GOAL_PLAN.md`, and `harness/docs/WALLET_ALPHA_AGENT_MISSION.md` first. Use `.agents/skills/wallet-alpha-solana-tracker/SKILL.md`. Preserve the Rust PF1 `oracle.rs` as the sole portfolio accounting truth; the offline Python lab is a research contract test, not a second ledger. **Provenance before belief** (`SOUL.md` Law 4): leader P&L is filtered for hand-offs, wash loops, and dev linkage before any follow hypothesis; verified-real trades are *candidate* signals only. Current record: W01 naive follow was executed and **REJECTED** (all 6 delay arms net-negative vs S00) — do not resubmit naive follow; S14 provenance-filtered follow is the open successor.

## 11. P&L discipline (profit-first)

The owner priority is **net realizable P&L after all trading/operating costs and risk controls**. Read `docs/PROFIT_FIRST_GOAL_PLAN.md` and `harness/docs/PROFIT_FIRST_AGENT_MISSION.md`; invoke `.agents/skills/sniper-pnl-architect/SKILL.md` for financial reporting or strategy selection. Reuse the H7/PF1 independent oracle and evaluator — do not rewrite or double-count. Separate `SYNTHETIC | FORK_MECHANICS | SHADOW_ESTIMATE | LIVE_CONFIRMED` receipt classes; ensure realistic executable exit marks; isolate external deposits/withdrawals and SOL/USD beta; fail closed on missing real fees/reserves. Fee truth is on-chain-observed (curve 95+30 bps; PumpSwap tiered 125→30 bps) — never hardcode or backfill.

## 12. Current facts snapshot (2026-11-05; verify against `docs/PROJECT_STATUS.md` at HEAD)

- Verified and merged: protocol decode/builders (M0–M3c incl. real-ELF fork execution), paper broker (H2), P&L V2 + fee provenance (PF0/PF1), strategy SDK + evaluator + 12 negative controls (H7), H8 registry machinery, Wallet Alpha stack (WA0→WA-full), Harness2 deterministic integration, INFRA 1–3 read-only RPC integration.
- **Verdict everywhere: `EDGE_NOT_DEMONSTRATED` — S00 NO_TRADE dominates** all tested policies on synthetic cohorts and real signals.
- **G0: mechanics COMPLETE, zero successful launches captured** (three COMPLETE windows landed in an 87% pump-tx failure storm). The provider `getBlock` gap is the single blocker for the entire S11+ frontier.
- H3 ≥7-day prospective shadow: NOT_STARTED (the harness refuses to fabricate it). Model-backed Harness2: BLOCKED. Next work package: `docs/NEXT_GOAL_H8_G0.md` PR-1→PR-4.
- Legacy `agent_mission.py plan` and older H5–H8 documents carry stale implementation-state strings; treat `docs/evidence/*` and actual code as present truth.

## 13. Documentation change discipline

- `SOUL.md` is doctrine: amend deliberately (owner-visible), never casually; evidence anchors and rejected registry are append-only.
- Evidence files and receipts are immutable once written; corrections happen in new files.
- This file is reorganized by function; new operational rules land as sections, dated facts land in `docs/PROJECT_STATUS.md`, strategy content lands in `SOUL.md` §5. Do not let this file regress into an append-only log.