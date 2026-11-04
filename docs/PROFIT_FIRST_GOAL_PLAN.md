# Profit-First SniperFusion — H8 / P&L V2 Goal Plan

**Date:** 2026-11-04. **Reviewed baseline:** main `f19319f374267ccef33031f35c934188a9213219`.
**Purpose:** turn the H7 deterministic strategy SDK into a scientifically defensible **net-profit evaluation and strategy improvement system**. This is an engineering goal plan; none of its new milestones are claimed implemented. The user seeks a profitable trading bot, but profits are not guaranteed.

## One-sentence H8 definition

**H8 = a bounded research loop that proposes strategies, tests real attainable after-cost outcomes, independently reconciles P&L, rejects fragile or losing candidates, and allows paper-only promotion of evidence-backed improvements.** It does not grant agents keys or mainnet execution access.

## Current truth: build on H7, do not rewrite

- Rust `sniper-strategy` already has S00–S10, `LegacySurrogateV1`, `ProtocolEconomicV1`, version-aware fee model, `oracle.rs` double-entry ledger/FIFO, `eval.rs` NAV, drawdown, fill and tail metrics.
- H7 reports 266 Rust / 55 Python tests at its merge and independently checked GitHub CI. Exact current test counts must be rerun.
- H7 Engine A synthetic cohort starts at 0.1 SOL; S00 no-trade = 0; best S04 and S06 = −823,780 lamports; worst S03 = −3,354,605 lamports. **No positive edge proven; do not promote.**
- Engine B lacks a verified real as-of reserve/fee snapshot cohort and does not yet feed a real-data, same-cohort leaderboard.
- H6 collector code exists, but G0 block-verified COMPLETE launch window is not demonstrated with existing RPC.
- H3 ≥7-day independent Mainnet read-only forward shadow is NOT_STARTED. A Surfpool fork tests protocol semantics, not live landing and profitability.

## North-star financial objective (separate science from marketing)

Maximize **expected net withdrawable portfolio profit** over repeated independent periods, *subject to risk caps, realistic execution, positive out-of-sample evidence and data quality*. The primary reported value is `net_liquidation_pnl` in SOL, with independently priced USD equivalent for investor reporting.

Never optimize only win rate, number of trades, gross ROI, speed, backtest Sharpe, or an isolated lucky winner. Do not assume the no-trade comparator is universally worse or guarantee consistent gains.

Two separate economic measurements are required:
1. **Net trading P&L** = initial-to-final marked executable equity change, adjusted for external funding and withdrawals, inclusive of all chain/trade fees.
2. **All-in operating P&L** = net trading P&L minus attributable RPC/provider/VPS/data/infra expenses for the period. S00 must receive consistent infrastructure-cost treatment in comparisons. Report all-in only when expenses are actually known; otherwise label incomplete.

For multiple quote currencies, report **SOL-equivalent and USD-equivalent series separately**, using as-of conversion marks with explicit source/age. Decompose change caused by underlying SOL/USD fluctuations from trade alpha.

## P0 fee-model risk discovered during this review

The existing `crates/sniper-strategy/src/fees.rs` `PumpSwapFeeState::canonical_reference()` returns **25 bps** total (20 LP + 5 protocol + 0 creator). The currently published https://pump.fun/docs/fees canonical PumpSwap tiers list **125 bps for the lowest-market-cap tier**, decreasing with market cap. This is not proof that every Engine B result is incorrect, because `canonical_reference` is a reference/test fixture, not an observed on-chain fee account. It **is a material risk of underpricing PumpSwap execution** if any real batch treats the reference value as the actual canonical fee.

Before any real-data leaderboard: trace every use of `canonical_reference()`; add failing tests for market-cap-dependent canonical fees and the boundary tiers using current authoritative program/fee_config accounts; verify protocol + creator + LP components, cashback and custom pairs as applicable, measured paid amounts against deployed-program Surfpool account deltas, and fail closed when fee provenance is not observed as of decision. Do not substitute a web schedule for on-chain fee_config at an arbitrary historic slot. **Gate: no silent 25-bps canonical fee default in real P&L evidence.**

## P&L V2 ledger contract

Use the existing independent `crates/sniper-strategy/src/oracle.rs` as the financial control plane; enhance without discarding double-entry accounting.

`equity_at_t = free_quote_cash + verified_recoverable_rent + verified_claimable + conservative_executable_sell_value(open_positions) - liabilities - conservative_unknown_outcome_haircut`

For shadow/paper use simulated quotes and marked uncertainty (never assert withdrawability); for confirmed on-chain transactions use actual wallet/account deltas. Unknown/failed submissions and unexitable token inventory must not count as realized cash.

`net_pnl = final_equity - starting_equity - external_deposits + external_withdrawals`

Attribution per order, mint, strategy, venue, risk mode and period:

- Gross price P&L (entry/exit actual or conservatively simulated executed amounts).
- Protocol / creator / LP fees, spread, price impact/slippage, base tx fee, CU priority fee, Jito/relay tips and failed transaction costs.
- ATA rent (locked vs independently proven recoverable), claimable proceeds (verified vs uncertain), conversion FX and unexitable inventory.
- Holding time, available vs landed quotes, decision-to-submit and event-to-landing latency.
- Creator/holder/market phase, entry and exit decision reason, why abstained, and the alternative best action *as known at decision time*.

Never double count a fee withheld from trade input and then separately post it again from cash. Recompute all aggregates from raw immutable execution/journal records with an **independent** accounting oracle.

## Required reporting contract

Create one versioned, immutable `PnlRunReceiptV2` carrying:
- run_id, source Git SHA, data SHA, strategy hash, fee_config/program version hashes, first/last as-of times, evidence_class, completeness and currency.
- starting NAV, deposits, withdrawals, ending marked-to-exit NAV, realized/unrealized net P&L, recoverable/unknown/open inventory, and all-in infrastructure net if known.
- win/loss counts, net return on start capital and capital deployed, per-trade expectancy, net profit factor, exposure/holding time, max peak-to-trough drawdown, worst loss, tail loss/CVaR (or INSUFFICIENT_N), fill and exit rates, latency p50/p95/p99, provider gaps, creator concentration and fees by category.
- per-trade and per-token breakdown, per strategy/market phase and date, and clear **NO_TRADE/DELAYED_ENTRY/RANDOM_ELIGIBLE** comparison on identical capital/opportunity windows.
- statistical uncertainty, multiple-trial count, rejected/unknown coverage, and proof/endorsement state: `SYNTHETIC | FORK_MECHANICS | SHADOW_ESTIMATE | LIVE_CONFIRMED` only after evidence exists.

Outputs: `run.json`, `trades.jsonl`, `equity.jsonl`, `fees.json`, `comparison.json`, `audit.json`, and optional static HTML dashboard. Raw secret-bearing RPC URLs/key strings never appear.

## Core comparison metric

Paired `incremental_net_edge = challenger_net_pnl - baseline_net_pnl` for identical cohort, period, starting capital, risk caps, fee model, missing-data decisions and opportunity set. Compute confidence by creator/day clustered bootstrap, not by treating adjacent ticks as independent bets.

Secondary objective: `net_pnl / maximum_capital_at_risk`, evaluated beside max drawdown, drawdown duration, unexitable inventory, tail losses, and position churn. Report average, median, cluster uncertainty intervals and per-regime robustness; never choose a policy solely by top mean ROI. Require meaningful effect after operating costs and realistic adverse-fee/latency/no-fill stresses.

## Architecture: keep deterministic Rust, place research outside hot path

```text
Read-only RPC + independent block coverage (G0)
 -> immutable event/account/fee-config snapshots with availability times
 -> strategy versions S00..S10 + future challengers (Rust, deterministic)
 -> Engine A (legacy diagnostic) AND Engine B (real protocol economics)
 -> PaperBroker/fill model with unknown outcome / no-fill / latency scenarios
 -> append-only execution + cash and token inventory journal
 -> independent double-entry Oracle / executable-liquidation NAV
 -> P&L V2 receipt + attribution / dashboard
 -> H8 bounded researcher / critic / sealed holdout
 -> shadow-only champion selection (separate human-approved paper config)
```

Security boundary: research/LLM workers can read sanitized receipts and propose offline patches. They cannot access key material or signer socket, mainnet broadcaster, `--submit`, or production risk configuration.

## Profit-first milestones and acceptance criteria

| Phase | Deliverable | Concrete gate |
|---|---|---|
| **PF0** | Financial and code gap audit against H7 `oracle.rs`, `eval.rs`, `paper.rs`, `broker.rs`, H6 dataset and current CLI; freeze P&L units and comparison contract | Document unit invariant, receipt schema, explicit known blind spots and 5 independent counterexamples; no duplicated ledger |
| **PF1** | **P&L V2**: net-to-liquidation accounting, end-of-run equity, full fee/exposure and strategy attribution, immutable JSONL receipts + HTML/readable scoreboard | Trade cash + inventory conservation to lamport; deposits/withdrawals excluded from P&L; fees once; open/unexitable inventory explicit; crash/partial-sell/unknown tx pass; Rust↔oracle exact |
| **PF2** | **G0 real data**: provider capability, block-verified COMPLETE cohort with as-of account/reserve/fee/config state and gap receipts | Independently verified 1–4-slot COMPLETE window first; then meaningful multi-window sample; no invented order; incomplete data blocked, not optimistically priced |
| **PF3** | **Real Engine B** paired S00–S10 on same G0 data, conservative fill uncertainty and independent NAV | Verified account and fee snapshots; Engine A/B outputs independently reported; no synthetic fallback misrepresented as real; all missing-state refusals included |
| **PF4** | **Profit tournament**: liquidity-aware entry/exit, creator-veto, delayed momentum, migration and cost/tip sensitivity; add exactly one strategy hypothesis per run | Same-cohort paired P&L, no-trade and delayed baselines, cluster/date holdout, adverse conditions, all attempted variants counted, positive edge only if genuinely observed |
| **PF5** | **H8 research governance**: finite trial registry, one-use holdout, separate proposer/critic, repeatability and multiple-comparison correction | ≤20 trials structurally; second holdout access denied across processes/crashes; all 12 negative controls; no signing or mainnet send route |
| **PF6** | **Prospective shadow** for ≥7 consecutive days with read-only observed launches, modeled attainable prices and independent coverage audit | Forward time-stamped as-of receipts; realistic p95/p99 age/fees/landing risks; compare daily/weekly per-strategy net P&L; no mainnet broadcasts |
| **PF7** | **Human review & release**: risk-adjusted all-in net edge, capacity, worst cases, kill switch and strategy downgrade/rollback | Paper-only promotion at first; any future funded test needs separate owner authorization, wallet compartmentalization and a bounded test protocol; no autonomous mainnet activation |

PF1 may be implemented with synthetic fixtures to test accounting while PF2 data acquisition is blocked. PF3 and meaningful profitability conclusions cannot be marked complete without real G0 and forward shadow.

## Proposed release board

Report every week: **NET P&L (SOL and USD)**, all-in P&L if infra known, return on start equity, max drawdown, closed/open positions, profitable/losing fills, trade count, risk-corrected expectancy, P&L per strategy/phase/creator cohort, protocol+network+tips+rent costs, data-coverage percentage, independent uncertainty, and edge vs S00 and delayed-entry. Separate live-realized, paper-estimated and synthetic tabs; never add their totals together.

Define statuses `LOSING | INCONCLUSIVE | PROMISING_SHADOW | ACCEPT_FOR_PAPER_REVIEW` instead of `BEST`/guaranteed.

## The next three code PRs, in order

1. **P&L Contracts / PF1**: add JSON schema, full portfolio receipt, independent oracle parity and informative CLI report. Must pass no-fee-doublecount, lost-position and cashflow invariants. File scope: `crates/sniper-strategy/src/{oracle,eval,engine/protocol}.rs`, `crates/sniper-runtime/src/{broker,paper,journal}.rs`, `crates/sniper-app/src/main.rs`, new `harness/schemas/pnl_run.v2.schema.json` and tests.
2. **Real G0 / PF2**: test bounded provider `getBlock` capability, durable cursor/chunk recovery, full block transactions, read-only as-of reserves/fee accounts and coverage receipt. File scope: `harness/rpc_capability_probe.py`, `launch_cohort_capture.py`, `crates/sniper-core/src/{coverage,observation}.rs` and tests.
3. **Engine B / PF3**: compare all strategies on real account state, receipt marked withdrawal-infeasible when sell route unknown, independent NAV recovery. File scope: `crates/sniper-strategy`, data adapter, receipt CLI and tests.

H8 council (PF5) follows those financial/data gates instead of hiding missing profitability evidence behind an agent-loop demo.

## Mandatory engineering loop

HEAD & STATUS AUDIT → register hypothesis + profit/risk metric → failing regression → smallest code change → Rust fmt/clippy/test + Python → protocol fork/replay where relevant → adverse landing/fee/censor stress → independent accounting → compare vs S00 + delayed → independent critic → provenance/uncertainty receipt → local commit/PR. Never auto-push/merge or activate funded trades. STOP when data is incomplete, proof weak or strategy has worse adverse P&L than no-trade.

## Provenance and secrets

Current operator RPC key remains unchanged. It must never be echoed into a prompt, manifest, test log, public code, request trace or agent environment. Agent roles operate on sanitized read-only inputs. This plan does not authorize real spending, paid API upgrades, order submission or a background agent service.
