# PF0 — P&L V2 Financial and Code Gap Audit

Date: 2026-11-04. Baseline: `codex/profit-first-h8-pnl-goal-20261104` @ `af7586b`.
Scope: audit of the H7 accounting layer against the P&L V2 contract in
`docs/PROFIT_FIRST_GOAL_PLAN.md`. This document freezes the unit
invariants and the receipt contract, lists known blind spots, and records
**five independent counterexamples** — concrete, reproducible ways the
current code would misstate P&L if used as-is. No code is changed by this
document; each counterexample becomes a failing test in PF1.

## 1. Unit invariants (frozen)

1. **Money is integer lamports** (1 SOL = 1e9) everywhere in ledgers,
   records, and receipts. No floats in accounting; ratios only in derived
   metrics.
2. **Ledger currency is SOL-denominated quote.** USD equivalents are
   *marks*, computed at as-of conversion prices, and never enter the
   double-entry ledger. A change of USD mark must change reported USD
   numbers but never SOL P&L (SOL/USD FX is not sniper alpha).
3. **Marks never post.** Executable-exit valuations of open inventory are
   model-derived inputs to NAV reporting only.
4. **Fees are counted exactly once.** Where a fee is withheld inside a
   trade amount (fee-on-input / fee-inside-output), the ledger carries the
   net cash movement and the fee appears once in the fee-attribution
   side-ledger — never as a second cash debit.
5. **Conservation is per posting.** The accounting equation
   (debit-natured total == credit-natured total; cash and inventories
   non-negative) is verified after every entry.
6. **Comparison contract**: any strategy P&L is reported as
   `incremental_net_edge = challenger_net_pnl − S00_NO_TRADE_net_pnl` on
   the identical cohort, capital, risk caps, fee model, and
   missing-data policy. Delayed-entry (S02) and deterministic
   RANDOM_ELIGIBLE baselines ride the same contract.

## 2. PnlRunReceiptV2 contract (summary; schema lands with PF1)

One versioned immutable receipt per run (deterministic sha256 over
canonical JSON — identical inputs MUST yield an identical hash), carrying:
run identity (run_id, git SHA, dataset/strategy/fee-config hashes, as-of
bounds, evidence_class, completeness, currency); NAV (starting, deposits,
withdrawals, ending marked-to-exit, realized/unrealized net P&L,
recoverable rent/claimable, unknown-hold haircut, open/unexitable
inventory); risk (max peak-to-trough drawdown on full NAV, worst loss,
CVaR or `INSUFFICIENT_N`, exposure/holding time); execution (win/loss,
per-trade expectancy, net profit factor, fill/exit rates, latency
percentiles, provider gaps); **gross-to-net fee waterfall by category**
(protocol/creator/LP, slippage, base fee, priority, tips, failed-tx costs,
rent locked/recovered); attribution (per strategy/mint/phase/date); the
NO_TRADE/DELAYED/RANDOM_ELIGIBLE comparison; statistical uncertainty and
trial count; proof state `SYNTHETIC | FORK_MECHANICS | SHADOW_ESTIMATE |
LIVE_CONFIRMED`. Bundle files: `run.json`, `trades.jsonl`, `equity.jsonl`,
`fees.json`, `comparison.json`, `audit.json`.

Core equations (from the goal plan, restated as tested invariants):

```text
equity_t = free_quote_cash + verified_recoverable_rent + verified_claimable
         + conservative_executable_sell_value(open_positions) − liabilities
         − conservative_unknown_outcome_haircut
net_pnl = final_equity − starting_equity − external_deposits
        + external_withdrawals
```

## 3. Five independent counterexamples (all verified in code at `af7586b`)

### CE-1 — Oracle double-counts sell-side fees against cash

`crates/sniper-strategy/src/oracle.rs` `post_sell_fill`: the engine's
`SellFill.quote_out_net` is ALREADY net of protocol/creator/LP fees,
slippage, network and priority costs (`engine/protocol.rs` `sell()`), and
the engine adds only `quote_out_net + rent_recovered` to cash. The oracle
credits cash with `quote_out_net` and THEN debits
`ProtocolFees/CreatorFees/LpFfers` **against Cash a second time**
(oracle.rs, sell-costs loop). Oracle cash therefore ends BELOW engine cash
by the fee amount on every fee-bearing sell — the PF1 gate
"Rust↔oracle exact to the lamport" fails. The existing cross-engine test
only exercised a buy, so the divergence shipped unnoticed. Repro (PF1's
failing test): run Engine B with a curve snapshot book that enters AND
exits, ingest records, compare `oracle.cash()` to `outcome.ending_cash_lamports`.

### CE-2 — PumpSwap canonical fee reference (25 bps) vs on-chain tiered feeConfig

`fees.rs` `PumpSwapFeeState::canonical_reference()` returns lp 20 + protocol
5 + creator 0 = 25 bps with `FeeSourceKind::GlobalConfig` provenance. Per
the pinned docs themselves (`FEE_PROGRAM_README.md`), since 2026-09-01
canonical pools are priced from the market-cap-tiered `feeConfig` accounts
when present — the GlobalConfig numbers are the `feeConfig == null`
fallback, and published pump.fun tiers start at **125 bps** for
low-market-cap coins. Using the reference on a real batch would understate
costs by up to ~5× and could flip a losing strategy to apparent profit.
The pinned clone cannot show current numbers (shallow clone @ `639b87f`;
the tier table is a PNG). Resolution: read-only `getAccountInfo` decode of
the four fee accounts (`4wTV1Y…` Global, `ADyA8h…` AMM GlobalConfig,
`8Wf5Ti…` curve FeeConfig, `5PHirr…` AMM FeeConfig — constants already in
`crates/sniper-protocols/src/pda.rs`) + `from_observed_fee_config`
constructors; `canonical_reference()` becomes test-fixture-only with a
fail-closed guard on real-batch paths.

### CE-3 — External deposits/withdrawals are unrepresentable

The oracle has no deposit/withdrawal API. An operator top-up posted
through the only available opening path (Cash/Equity) inflates starting
equity understates P&L — or worse, a naive "ending − starting" computation
counts the deposit as profit. The goal-plan equation
`net_pnl = Δequity − deposits + withdrawals` is currently unrepresentable.
PF1 adds `ExternalDeposits`/`ExternalWithdrawals` accounts with postings
that bypass trading P&L, plus the mandated test "deposits are not profits".

### CE-4 — Shared-wallet balances are not strategy-attributable

`LedgerEntry.strategy` is a tag, but `balances`, FIFO `lots`, and
`rent_parked` are global. Two strategies holding the same mint share one
lot queue (one strategy's sell relieves the other's cost basis), and
`trial_balance()` cannot answer per-strategy questions. A shared-wallet
tournament would attribute P&L arbitrarily. PF1 adds per-strategy
sub-ledger re-derivation from the append-only entries (each entry is
self-contained, so filtering by strategy tag and re-posting into
subsidiary oracles is exact), with the invariant that subsidiary P&L sums
to the total.

### CE-5 — Gross-to-net waterfall component gaps in Engine B records

`ExecutionRecord::BuyFill/SellFill` omit slippage as a field (it is folded
into `tokens_out`/`quote_out_net`), curve fills never split creator from
protocol (`creator_fee: 0`, "component split carried via bps"), and
PumpSwap buys record `lp_fee: 0` while computing `creator_fee` from gross
spend independently of the withheld `protocol_fee` (rounding mismatch). A
"fees by category" report from today's records is wrong or incomplete, so
the V2 waterfall cannot be audited against it. PF1 extends the records
with `slippage_lamports` and exact component splits and derives
`fees.json` from records only.

## 4. Known blind spots (explicit, not silently deferred)

1. **No real-data cohort**: every number today is `SYNTHETIC_TESTED`
   (Engine A) or `VERIFIED_CODE`-on-fixtures (Engine B). PF2/PF3 gates
   real data; nothing here may be quoted as market evidence.
2. **Fee tiers unobserved until the probe runs** — the on-chain read is
   the only authoritative source (pinned docs carry a PNG table).
3. **Claimable-accumulator accounting** requires on-chain reconciliation
   evidence before posting (M7 dependency); oracle API exists, unused.
4. **Latency percentiles** in receipts derive from synthetic
   `feature_available_at` timestamps until PF6 shadow data exists.
5. **Engine A (legacy surrogate) is a diagnostic parity harness**, not a
   performance estimate; V2 profitability figures come from Engine B
   records only.

## 5. No-duplicated-ledger confirmation

P&L V2 uses the EXISTING `oracle.rs` double-entry ledger as the single
control plane (enhanced per CE-1/CE-3/CE-4); Engine A/B records remain the
only execution-event sources; `eval.rs` remains the metric layer. No new
parallel ledger is introduced.
