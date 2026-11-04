---
name: sniper-pnl-architect
description: Audit and implement SniperFusion net-to-liquidation P&L, double-entry portfolio accounting, fee attribution, strategy leaderboards, paired benchmarks and independent evidence-based profit reviews.
---
# SniperFusion P&L Architect — Profit Accountability

Read `docs/PROFIT_FIRST_GOAL_PLAN.md`, `docs/evidence/H7.md`, `crates/sniper-strategy/src/{oracle,eval,engine/protocol,fees}.rs`, and `harness/docs/PROFIT_FIRST_AGENT_MISSION.md`. This skill is a procedure, not trading permission.

## Requirements

1. The goal is credible **positive after-cost, risk-constrained net P&L**, not win rate, gross backtest returns or speed. H7 synthetic cohort currently has no demonstrated edge.
2. Reuse and verify the existing double-entry FIFO oracle and protocol economics. Never duplicate inventory or fees, treat deposits as profit, mark unknown balances as realized, or invent executable exit liquidity.
3. Track initial and ending conservative liquidation NAV, external cash flows, realized/unrealized P&L, fees/tips/failed transaction costs, rent/claimables, open inventory, risk exposure, worst drawdown, gain/loss distribution and duration.
4. Separate SOL-denominated, stable-coin and USD-reported returns with as-of conversion prices, market beta and data-source ages.
5. Compare every strategy on the SAME complete point-in-time cohort, fees, capital and risk budget against S00 NO_TRADE and delayed entry. Include operating infrastructure only if measured and apply consistent allocation.
6. Specify the falsifiable edge hypothesis, adversarial execution shocks, trials count, holdout creator/day clustering, negative controls, P&L uncertainty and independent critic. Report insufficient samples as INCONCLUSIVE.
7. Use only read-only sanitized data and ephemeral fork keys. No live wallet access, funded mainnet orders, relay sends, or autonomous activation.

## Deliverable

Immutable P&L receipt with per-fill/strategy/mint/venue/source attribution, a gross-to-net waterfall, ending NAV reconciliation to the lamport, credible delta vs S00, and explicit evidence class. Include exact executed tests and a single ranked next profit hypothesis.
