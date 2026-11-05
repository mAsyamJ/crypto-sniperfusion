# SniperFusion H5–H8 Goal Plan and Acceptance Matrix

**Archive status (2026-11-05):** POINT-IN-TIME — H5–H8 acceptance matrix; state strings predate H6/H7.

**Date:** 2026-11-04. **Owner:** SniperFusion engineering. **Authoritative coding mission:** [MASTER_AGENT_MISSION_H5_H8.md](MASTER_AGENT_MISSION_H5_H8.md). **Implementation branch:** `codex/devnet-pilot-research-loop-20261104` / draft PR #6, based on merged PR #2. Verify current HEAD/PR before working.

## North-star: what counts as success?

A deterministic SniperFusion system that can **auditably decide NO_TRADE** and, only if supported by high-quality data, identify candidates with repeatable **positive after-fee, after-tip, after-slippage, after-latency and after-loss risk-adjusted expected value** on a genuinely unseen cohort. The system must prove that account conservation, sold-token proceeds, claimable balances and open-position NAV reconcile.

Neither the current Python synthetic batch nor Devnet faucet transactions are evidence of real-world expected trading profit. A 100% win rate is not feasible as an engineering guarantee; a large backtest win rate without credible economics is not a launch gate.

### Research metrics

**Economic primary:** net mark-to-executable-exit portfolio P&L, net excess value versus S00 NO_TRADE and transparent initial/final cash; each computed across fixed same-cohort capital.

**Risk and operation secondary:** worst rolling drawdown, CVaR/tail losses, open unexitable inventory, transaction landing p50/p95/p99 and fail rate, stale/gap rate, creator concentration, realized/unrealized cost split, fees/tips, sell liquidity, dataset breadth by market regime, bootstrap confidence with creator/day blocks, number of tried variants.

**Pre-register** target effect size, holdout duration, min independent cluster count and risk drawdown tolerances based on measured dataset noise. Do not retroactively pick thresholds or report "alpha" from a single mint or optimized holdout.

## Workstreams with explicit definition of done

| ID | Priority | Engineering work | Evidence/definition of done | Status |
|---|---|---|---|---|
| H5-A | P0 | Devnet RPC genesis, Pump.fun deployment and mint validity preflight | Testnet/non-Devnet MUST fail, wrong-token mint MUST fail; read-only preflight reports status | Implemented, needs operator network run |
| H5-B | P0 | Single bounded Devnet buy → optional sell with dedicated pilot key and WAL | Real confirmed signatures, token and SOL deltas, volume accumulator reconciliation; no key leaks, no blind retry | Code present, **NOT EXECUTED** |
| H5-C | P0 | Surfpool V3 and V2/PumpSwap fork economics parity | Independent account-level receipts for supported paths; failure injection | V3 inherited; other paths partial |
| H6-A | P1 | All-launch creation cohort collector | Measurable bounded denominator, observation/chain order, backfill/reorg handling, missing receipt | NOT IMPLEMENTED |
| H6-B | P1 | Versioned point-in-time feature & label contracts | Strict as-of observability, isolated future labels, complete fee/quote/migration flags | NOT IMPLEMENTED |
| H7-A | P1 | Native deterministic Rust strategy SDK | S00 + ten candidate policies, replay reproducible, same event cohort and virtual capital | NOT IMPLEMENTED (Python surrogate only) |
| H7-B | P1 | Independent economic accountant | Cross-mint lots, full NAV/quote, claimable proceeds, costs/fees, failure/latency stress reconciled | PARTIAL (H2 broker) |
| H7-C | P1 | OOS statistics and genuinely sealed temporal/creator holdout | No data leakage, trial accounting, clustered confidence, adverse scenario robustness | NOT IMPLEMENTED |
| H8-A | P2 | Offline LLM research council with typed hypothesis/criticism | Untrusted data isolation; independent critic; one patch per hypothesis; never signer access | SKILLS ONLY; controller demonstration |
| H8-B | P2 | Repeatable experimental budget | Max 20 iterations, immutable receipts, tests first, rejection/failure provenance | **IMPLEMENTED FOR SYNTHETIC** |
| H8-C | P2 | Real read-only mainnet shadow | >=7 consecutive days, explicit source/slot coverage, no mainnet broadcast | NOT STARTED |
| H8-D | P2 | Paper-only deterministic promotion | Every gate externally attested, human review, versioned config, no funded enable | NOT IMPLEMENTED |

## Three loops at different timescales

### 1. Millisecond market loop — only deterministic, no AI or key by default

`OBSERVE → CHECK_GAP/AGE → DECODE → AS_OF_FEATURES → LIFECYCLE → TOKEN_SAFETY → ROUTER → RISK → PAPER_INTENT → FILL/EXIT_MODEL → LEDGER`. For any stale snapshot, missing source or uncertain P&L account state, use NO_TRADE and preserve evidence. Live execution is not connected.

### 2. Per-commit engineering loop

`AUDIT HEAD → DEFINE AC → FAILING TEST → IMPLEMENT MINIMAL DIFF → fmt → Clippy → unit/integration → Surfpool where required → independent review → evidence + SHA → PR`. No skipped tests counted as PASS. If tests fail, STOP; do not advance.

### 3. Research scientific loop, capped

`FREEZE DEV DATA → HYPOTHESIS → S00 + S01–S10 under matched capital → COST/LATENCY STRESS → NEGATIVE CONTROLS → CRITIC → RETAIN/REJECT → (<=20 total cycles) → FREEZE CHAMPION → ONCE-ONLY OOS → FORK → SHADOW → HUMAN PAPER REVIEW`.

The currently executable `harness/research_cycle.py` implements **synthetic demonstration** of a bounded variation of this loop. An AI agent must not self-select mints and immediately submit orders.

## Network matrix

| Environment | Agent access | Transactions | Scientific meaning |
|---|---|---|---|
| Synthetic JSONL | Offline read | None | Contract/tests only |
| Surfpool loopback mainnet-state fork | Ephemeral local | Locally executed deployed program transactions | Account/IDL mechanics, not MEV/actual historical landing |
| Solana Devnet | Read-only default | Only separately operator-approved 1 buy / ≤1 sell with pilot key | Devnet program mechanics; not financial alpha |
| Solana Testnet | Read-only availability/health | None for Pump.fun in this plan | Validator and network stress, Pump.fun deployment not assumed |
| Mainnet | Read-only only | ZERO | Real source-of-edge market data and shadow-only evidence |
| Unverified "forecastnet" | No automatic access | ZERO | Require chain genesis and program deployments before treating it as any Solana cluster |

Reference official documentation: https://solana.com/docs/references/clusters and https://github.com/pump-fun/pump-public-docs.

## This PR's operational controls

`harness/agent_mission.py` is a **runnable safe mission controller**. It carries an explicit inventory of unverified gates, invokes only a fixed Python unittest command with a sanitized environment, checks synthetic feature chronology and runs a fixed number (1–20) of experiments in `harness/research_cycle.py`. It writes an exclusive, fsynced JSON report with code/data/mission hashes and permanently `promotable=false`. It never connects an RPC, signs, attempts a Devnet buy or uses an LLM.

Commands from repository root:

```bash
python3 harness/agent_mission.py plan
python3 harness/agent_mission.py run \
  --events harness/fixtures/research/synthetic_lifecycle.jsonl \
  --cycles 3 \
  --out harness/runs/h5-h8-agent-mission-001.json

# Existing guarded Devnet preflight (read-only, needs network and eligible mint):
python3 harness/cluster_probe.py --cluster devnet
cargo run -p sniper-app --bin devnet-pump-pilot -- --mint VERIFIED_DEVNET_PUMP_MINT
```

No output of `agent_mission.py` may be imported by H4 `promote` as a real OOS or shadow receipt.

## Hand-off PR order

**PR-next-1, correctness & protocol test:** confirm Devnet pilot preflight and actual signature if available, reconcile the claimable accumulator; freeze protocol truth table and negative tests. If an operator test is not possible, report BLOCKED and continue with offline implementation; do not fake it.

**PR-next-2, lifecycle source:** implement new `CanonicalChainEvent` and `ObservedEvent` types, completeness accounting and all Pump.fun creation event denominator in a bounded read-only window. Include provider gap/reorg tests.

**PR-next-3, economic science:** implement a pure Rust 11-policy replay with a separate full NAV independent oracle and calibrated fork-derived cost model; add negative controls and true temporal/creator splits.

**PR-next-4, research council:** add prompt-injection-resistant agent sandbox, experiment schema/trial registry, independent critic and sealed OOS access control, then **actual shadow only after verified data coverage**.

## Milestone failure conditions

Block advancement for secret exposure, non-Devnet transaction signing, any missing/unreconciled order, stale/missing data used as a positive trade signal, future feature lookahead, corrupted/different dataset hashes, success-only creation cohorts, missing open loser inventory, unverifiable program fee assumptions, tests skipped as PASS, reuse of exposed sealed holdout, or any hard cap violation.

**Actual next executable milestone:** operator collects a valid Devnet mint, runs a no-send pilot, reviews account/program correctness, and explicitly approves a single capped Devnet smoke. The research agents can meanwhile run offline experiments and work on the mainnet read-only collector independently.
