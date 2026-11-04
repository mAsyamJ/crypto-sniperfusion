# PF1 Evidence — P&L V2

Date: 2026-11-04. Branch: `codex/profit-first-h8-pnl-goal-20261104`. Commit: `2ed8e8d`.
All work offline/deterministic; synthetic fixtures; nothing promotable.

## Counterexample closures (failing-test-first where a fix was involved)

- **CE-1** (red → green): failing test `oracle_reconstructs_engine_round_trip_cash_independently` reproduced the divergence (oracle 99,923,467 vs engine 99,935,686 = 12,219 lamports of double-counted sell fees). Fix: sell-side fee components moved to the reporting-only `FeeLine` side-ledger; the ledger credits the already-net proceeds exactly once. Round trip now reconciles to the lamport including tips and slippage.
- **CE-3**: `ExternalDeposits`/`ExternalWithdrawals` accounts + `post_deposit/post_withdrawal`; `net_pnl = ΔNAV − deposits + withdrawals`.
- **CE-4**: `attributed()` — exact per-strategy cash delta / realized P&L / fees-by-kind / fills derived from the append-only entries.
- **CE-5**: `slippage_lamports` (quote-equivalent) on Buy/Sell records; buy/sell fee attribution is the exact withheld-fee split (checked-div, no independent recomputation); `FeeKind::Slippage` in the waterfall; failed landings record fee lines.

## New machinery

- `FeeLine` side-ledger + `fee_totals()/fee_lines()` (gross-to-net waterfall source).
- `receipt.rs`: `PnlRunReceiptV2` (schema 2, SOL-lamports, proof_state, NAV/P&L/fee/attribution/comparison/verdict/max-drawdown) with **deterministic sha256 receipt hash** (identical inputs → identical hash; content-change → different hash; both tested); write-once 0444 bundle `run.json, trades.jsonl, equity.jsonl, fees.json, comparison.json, audit.json` + per-strategy receipts.
- `RANDOM_ELIGIBLE` (RX) deterministic baseline policy (FNV(event_id) < 2000/10000).
- `MarketBook::load_jsonl` (invalid lines are hard errors).
- CLI: `sniper research --events X --out R --snapshots S --out-dir D` — Engine-B batch (S00–S10 + RX), per-strategy oracle, **fail-closed audit gate** (any conservation failure or cash≠engine aborts the run), per-record cash curve for equity.jsonl.
- `harness/schemas/pnl_run.v2.schema.json` (draft-07).
- The 9 mandated PF1 tests in `tests/pnl_v2.rs` (deposits≠profit; partial-exit basis conservation; failed-tx fee once; tip/priority units; rent locked; FX not alpha; unexitable not withdrawable; unknown-tx not silently realized; identical receipt hash) + a bonus round-trip test with tips+slippage + CLI bundle test.

## Experiment report (mandated format)

- HEAD: `2ed8e8d` (then G0/H8 commits). Tests: workspace 285/285 both feature sets; harness 103 (at close of all missions); fmt/clippy clean.
- **Sample run** (`/tmp/pnl_v2_demo`, receipt sha `111c6c44…`): Engine B on the synthetic cohort with the OBSERVED 95+30 bps curve fee config, oracle-reconciled.
- Start equity 100,000,000 lamports (0.1 SOL). **S00 NO_TRADE: net 0, drawdown 0, fees 0.**
- Comparison (net P&L lamports, vs S00): RX −64,253; S01/S02 −192,760; S03 −1,138,507 (1 open position); S04–S09 −64,253; S10 −128,506.
- Fees by category for a losing trader: protocol+creator (125 bps observed), network+priority, slippage, ATA rent — full waterfall in each strategy's `fees.json`.
- Uncertainty: CVaR/median over closed trades report `INSUFFICIENT_N` (fewer than 20 trades) — stated, not imputed. Trial count 1 per strategy on one synthetic cohort; no multiple-testing correction applicable yet (registry ready, unexercised).
- **Verdict: every strategy REJECTED on this evidence** (synthetic class, `EDGE_NOT_DEMONSTRATED`); S00 dominates. Next measurable hypothesis: with a real G0 cohort at observed 125 bps fees, does ANY entry policy show positive `incremental_net_edge_vs_no_trade` on the dev split? (H-0001 template exercised in registry tests.)

## Limitations (on the receipt itself)

Synthetic fixture cohort, not market evidence; execution frictions synthetic; fees observed on-chain but reserves/snapshots synthetic; no real shadow data.
