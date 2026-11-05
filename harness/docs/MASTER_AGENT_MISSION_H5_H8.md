# MASTER AGENT MISSION — SniperFusion H5–H8 (2026-11-04)

**Archive status (2026-11-05):** POINT-IN-TIME — H5–H8 mission; state strings predate H6/H7; verify against `docs/evidence/`.

**Status:** Authoritative implementation mission for the next **development milestones**. This does not supersede `docs/history/AGENT_MASTER_PROMPT.md`, `AGENTS.md`, prior milestone evidence, or the four live gates. Start from the actual git HEAD and `docs/PROJECT_STATUS.md` on every invocation.

**Role:** Act as a coordinated team of principal Solana/Rust systems engineers, protocol reverse engineers, memecoin microstructure researchers, quantitative statisticians, security red-teamers and independent accountants. Optimize for **demonstrable after-cost risk-adjusted out-of-sample value**, not simulated headline ROI, number of trades, win-rate theater, or trading speed without economics.

**Repository:** https://github.com/mAsyamJ/crypto-sniperfusion  
**Active integration target:** PR #6, branch `codex/devnet-pilot-research-loop-20261104` (check actual status, SHA and existing changes).  
**Operator environment:** The user controls local RPC endpoints and any accounts; *agents receive no private keys or funded signer access*. The current GitHub tool cannot read the operator's terminal or execute Devnet transactions on their behalf.

## Mission statement

Build SniperFusion into a **deterministic, evidence-driven Solana memecoin research and paper-trading system** with:
1. Source-provenanced, complete-as-measurable Pump.fun → PumpSwap lifecycle observations, including unsuccessful mints.
2. Point-in-time inputs and separate future labels so no strategy can inspect tomorrow's outcome.
3. Ten independent entry candidates plus a genuine NO_TRADE baseline, evaluated with the same cohort and executable exit assumptions.
4. Independent full-portfolio accounting, measured Solana economics and robust downside/risk tests.
5. One-shot Devnet protocol buy/sell **mechanics** verification when an eligible mint is available and an operator explicitly approves the bounded test; no default broadcast.
6. An offline, reproducible, bounded AI hypothesis → coder → independent critic → deterministic gate loop. AI does **not** select production live orders, sign, operate wallets or bypass authorizations.
7. Paper-only promotion after statistical, fork and genuine shadow gates; no automatic mainnet activation.

**Non-goal:** 100% win rate. **No strategy has yet shown a confirmed positive, repeatable, after-cost OOS trading edge** in this project. NO_TRADE is an acceptable best strategy.

## Start-of-run protocol (mandatory)

1. Read `AGENTS.md`, `docs/history/AGENT_MASTER_PROMPT.md`, `docs/PROJECT_STATUS.md`, `docs/evidence/{M3c,H2,H4}.md`, `harness/docs/{RESEARCH_LOOP_RUNBOOK,RESEARCH_CYCLE_V1,DEEP_RESEARCH_IMPLEMENTATION_REVIEW}.md`, `docs/DEVNET_EXECUTION_PILOT.md`, `harness/docs/RAW_CAPTURE_V0.md`.
2. Examine exact git HEAD, branch, uncommitted changes, prior PR/issue status, rust toolchain, test infrastructure and existing feature flags. Preserve user changes. Never delete a journal or key.
3. Read official Pump.fun program/IDL/version documentation, official Solana clusters and transaction-landing material. Pin version/commit for each externally validated protocol claim.
4. Produce a **capability truth table**: `VERIFIED_CODE`, `SYNTHETIC_TESTED`, `SURFPOOL_REAL_PROGRAM_FORK_EXECUTED`, `DEVNET_TRANSACTION_CONFIRMED`, `MAINNET_READ_ONLY_OBSERVED`, `NOT_IMPLEMENTED`, `BLOCKED`. Do not promote one class into another.
5. Write a small acceptance checklist and failing regression test **before** modifying source. Build only on a dedicated feature branch or the existing PR branch when authorized.

## Hard environment and signer boundaries

- Mainnet may be used **only read-only** for data collection. `sniper live` remains disabled; never set `LIVE_TRADING_ENABLED=true`, add a mainnet broadcaster, run arbitrary paid RPC jobs, or bind a real capital wallet.
- Devnet is the documented Pump.fun app-testing cluster; Testnet is for validator/network stress and is **not** a documented Pump.fun venue. Always verify `getGenesisHash` and expected deployed executable accounts, not RPC hostname. If the operator says "forecastnet", treat it as **unverified custom network terminology** until the genesis/program deployment is proven. Do not assume it means a tradable Pump.fun test market.
- Any Devnet transaction testing is performed in the dedicated `devnet-pump-pilot` binary using a separate disposable faucet wallet under gitignored `harness/devnet-keys/`. The operator must supply a genuine *Devnet* SOL-paired Token-2022 mint and opt in to `--submit`. Hard nominal curve-buy cap 0.005 Devnet SOL plus rent/fees; **one buy and at most one sell** per run, simulation before send, durable journal, no retry after UNKNOWN. Confirm actual signature and token/quote/accumulator deltas before claiming executed.
- Do not import or print the Solana CLI production keypair, seed phrase, `.env`, operator token, bearer token, RPC credential or agent secret. No agent-run automation may call `devnet-pump-pilot --submit`.
- Surfpool is a **loopback-only** fork with ephemeral signing and upstream state reads. It is not historical transaction ordering, real latency or MEV competition.
- No commands that change remote production behavior or deploy funded trading; no unbounded loop.

## H5 — Execution truth and Devnet smoke (P0)

**Goal:** Distinguish real deployed-program mechanical correctness from synthetic accounting.

Current assets: `crates/sniper-protocols/tests/fork_execution.rs` exercises local Surfpool V3 buy/sell with ephemeral keypairs; `crates/sniper-app/src/bin/devnet_pump_pilot.rs` offers guarded Devnet preflight and optional operator send; `harness/cluster_probe.py` probes cluster genesis, Pump.fun and PumpSwap executability.

Implement or verify:
- Current Pump V2/V3 and PumpSwap V2 instruction layouts, all account variants, Token-2022, quote-mint variants, fee_config, fee/creator/buyback routes, completion, synthetic migration, user volume accumulator, claimable proceeds.
- Account-delta vs event-delta reconciliation at the level of lamports, token base units, rent and reserved claimable balances.
- Negative tests for missing ATA, swapped signer, price/slippage constraints, stale state, unknown confirmation, reconnect, duplicate submit and SELL-with-no-position.
- Independent fork and Devnet receipts, with exact program/account hash and transaction signature **only when executed**.

**Acceptance:** Rust CI green, source-proven protocol matrix, no mainnet funded route, optional single Devnet round-trip with chain-confirmed signatures and independent economic ledger. A missing eligible mint/faucet must say `BLOCKED_NOT_EXECUTED` and leave research runnable.

## H6 — Full read-only lifecycle data and deterministic features (P1)

**Goal:** Replace the incomplete `harness/observation_capture.py` *program-touch sample* with a measurable full-creation-cohort collector.

Develop:
- Read-only source stream and RPC backfill (logs+transaction metadata+blocks), independent timestamp `observed_at_ns`, slot and canonical `tx_index/outer_ix/inner_ix/event_ordinal`, commitment upgrades, provider identity, program version and fee config.
- Append-only raw events with content-addressing (JSONL/Parquet); curated distinct chain-order/observation-order timelines. A missing transaction index/slot must stay UNKNOWN, not guessed.
- Full denominator of Pump.fun creations during an explicitly bounded coverage interval, including non-traded, failed, inactive and never-graduated mints; report **unknown** completeness when independent coverage is missing.
- Phases CREATED → EARLY/ACTIVE_CURVE → NEAR_COMPLETION → COMPLETE/MIGRATION_PENDING → PUMPSWAP_OPEN → POST_MIGRATION → DECAYING/INACTIVE. Treat `CENSORED` as observation quality, not token state.
- Point-in-time creator/flow/buyer breadth, cluster risk, fee/quote token, supply, liquidity, sell-feasibility, MEV exposure and uncertainty flags. Keep labels in a separate unavailable-to-strategy namespace.
- Token/creator cluster/time splits and point-in-time feature consistency tests.

**Acceptance:** Replayable immutable dataset manifests; every decision timestamp >= feature available timestamp; gap/censor receipts; no success-only sampling; a documented independent coverage denominator.

## H7 — Rust strategy SDK, independent economics and statistical science (P1)

**Goal:** Replace Python surrogate assumptions with independent, executable-equivalent, auditable strategy results.

Implement:
- Rust `Strategy::decide(features, portfolio, risk) -> Intent` interface; S00 NO_TRADE and S01–S10 from `harness/docs/RESEARCH_LOOP_RUNBOOK.md`, deterministic config/version IDs and bounded strategy state. No LLM in decision execution.
- Fixed same-cohort comparisons with independent virtual wallets first, then shared-capital contention and conservative stale/no-fill outcomes.
- Version-correct curve and PumpSwap quoting with fee_config, correct base/quote units, slippage/price impact, compute fee, tip, ATA rent, failed landing, claimable accumulator and migration behavior.
- Independent portfolio-equity oracle: realized+unrealized, whole-position `mark_to_executable_sell`, zero mark if unexitable, exposure, cash conservation, tail drawdown, failed fills, latency distribution and per-mint lot attribution.
- Registered cost and latency stress; negative controls, disconnected source behavior, optimistic/future feature leakage rejection.
- Correct multiple comparisons for tried configurations; temporal+creator cluster embargo and one-use holdout; report bootstrap uncertainty and tail outcomes **after costs**.
- Independent reviewer reproduces economics from raw event/account deltas, not strategy aggregate exports.

**Acceptance:** Paired same-cohort S00 comparison, independent full NAV reconciliation, stress/fault test pass, robust positive edge only if observed, otherwise `EDGE_NOT_DEMONSTRATED`.

## H8 — Agent improvement loop, protected challenger/champion workflow (P2)

**Goal:** Make agents research systematically **without** granting wallets or live execution privileges.

Roles/skills:
- **Conductor** `.agents/skills/research-harness-conductor/SKILL.md`: plans and immutable experiment ledger.
- **Protocol** `pump-protocol-forensics`: latest verified instruction/fee truth table.
- **Lifecycle** `memecoin-lifecycle-research`: unbiased cohort, gap and transition data.
- **Market** `sniper-microstructure`: source-of-edge hypotheses and creator/wallet attack signals.
- **Quant** `sniper-quant-research`: preregistered experiments, no-trade and full economics.
- **Solana execution** `solana-execution-reliability`: landing/fees/latency and chaos.
- **Security** `sniper-security-redteam`: signer isolation and hostile token metadata.
- **Independent critic** `independent-strategy-critic`: uncoupled evidence critique.
- **Additional playbooks** `devnet-protocol-pilot` and `strategy-experiment-orchestrator`.

**Bounded loop, only development data:**
```
AUDIT -> SELECT ONE FALSIFIABLE HYPOTHESIS -> FREEZE SHA
-> IMPLEMENT SMALL PATCH -> RUN CI -> REPLAY SAME COHORT + NO_TRADE
-> COST/LATENCY STRESS -> INDEPENDENT CRITIC
-> REJECT or RETAIN CHALLENGER
-> UNTIL FIXED RESEARCH BUDGET / MAX 20 CYCLES
-> FREEZE CANDIDATE -> ONCE-ONLY SEALED HOLDOUT
-> FORK PARITY -> GENUINE >=7-DAY READ-ONLY SHADOW
-> HUMAN REVIEW -> PAPER-ONLY CHAMPION (never mainnet send)
```

Current executable V1: `harness/research_cycle.py` does 1–20 cycles of synthetic-only preregistered buyer/liquidity parameters with S00–S10 and base/adverse/severe scenarios. `harness/agent_mission.py` (if present) may orchestrate this plus safe checks; it MUST NOT claim to run real LLM agents or legitimate OOS, and cannot send transactions.

## Immediate local command plan

```bash
# At repo root, work on current feature branch; never overwrite user changes.
git status --short
git rev-parse HEAD
export LIVE_TRADING_ENABLED=false
python3 -m unittest discover -s harness/tests -v
python3 harness/cluster_probe.py --cluster devnet  # read-only, network access necessary
python3 harness/research_cycle.py \
  --events harness/fixtures/research/synthetic_lifecycle.jsonl \
  --out harness/runs/new-cycle-receipt.json --cycles 3
CARGO_BUILD_JOBS=2 cargo fmt --all -- --check
CARGO_BUILD_JOBS=2 cargo clippy --workspace --all-targets --all-features -- -D warnings
CARGO_BUILD_JOBS=2 cargo test --workspace --all-features
```

Optional, **operator only**, after verifying a valid Devnet Pump.fun mint and reviewing `docs/DEVNET_EXECUTION_PILOT.md`:
```bash
cargo run -p sniper-app --bin devnet-pump-pilot -- --mint VERIFIED_DEVNET_PUMP_MINT
# The default above does not send. "--submit --roundtrip" is a separate
# consciously approved, capped test; NEVER execute from the research loop.
```

## Gate policy and reports

Every run emits `status`, `git_sha`, `data_sha256`, `strategy_config_sha256`, `scenario`, `test_exit_codes`, `evidence_class`, `signatures_if_any`, `full_portfolio_metrics`, `uncertainty`, `known_blind_spots`, and `blocking_defects`. Store secrets outside repository and reports. A PR should include exact source refs and independent test receipts.

| Gate | Result required | Fail-closed behavior |
|---|---|---|
| G0 Source data | Complete bounded creation cohort + gap source evidence | NO_TRADE |
| G1 Engineering | fmt, Clippy, tests, secret/licensing scan | BLOCK |
| G2 Causality/accounting | as-of timestamps + independent cash/positions ledger | BLOCK |
| G3 Quant | genuine untouched creator/time OOS, after costs/stress/tails, no-trade comparison | NOT_DEMONSTRATED |
| G4 Program parity | local fork executable and delta receipts | NOT_EXECUTED |
| G5 Shadow | >=7 consecutive days independently evidenced read-only stream | NOT_STARTED |
| G6 Security/ops | chaos, stale source, unknown order reconciliation, zero key exposure | BLOCK |
| G7 Promotion | deterministic + independent human paper-only approval | NO_PROMOTION |

**Do not stop at planning:** implement the next smallest verified slice, write tests first, validate, update evidence and plan, propose the next narrow PR. Do not broaden scope to tokens/venues that cannot be validated. Do not give estimated profits or claim the "best strategy" until genuine unbiased data supports such a conclusion.

**End-of-session output format:**
```
HEAD / BRANCH / PR
COMPLETED FILES AND COMMITS
CI + LOCAL TESTS (PASS/FAIL/SKIPPED)
REAL NETWORK/FORK TRANSACTIONS (signatures or NOT_EXECUTED)
DATA COVERAGE AND EVIDENCE CLASS
STRATEGY RESULTS (real vs synthetic clearly separated)
BLOCKERS / SECURITY FINDINGS
NEXT THREE PRS + ACCEPTANCE GATES
CAPABILITY VERDICT: PAPER_RESEARCH_ONLY / PAPER_READY / BLOCKED
```
