---
name: cohort-coverage-engineer
description: Build and audit complete, point-in-time Solana memecoin creation cohorts with block-verified denominators, provider capability checks, rate-limit budgets, gap recovery and immutable provenance.
---
# G0 Creation-Cohort Coverage Engineer

Read `docs/evidence/H6.md`, `crates/sniper-core/src/coverage.rs`, `harness/launch_cohort_capture.py`, and `docs/NEXT_GOAL_H8_G0.md`. The current collector has NEVER demonstrated a complete recent real Mainnet creation cohort; do not state otherwise.

## Workflow

1. Probe mainnet genesis, old/recent finalized `getBlock`, transaction-version support and latency before expensive discovery; never print the RPC URL with a key.
2. Budget requests, retry-after, in-flight concurrency and hard deadlines; checkpoint results after every bounded block batch and make restarts idempotent.
3. Block receipts with transaction ordinal are required for canonical event order; signatures are candidate discovery only. Missing block evidence => INCOMPLETE or UNKNOWN.
4. Persist independent provider observations, chain order, fork commitment upgrades, skipped-slot evidence, failed transactions, untouched mints, fees and creator state as-of decisions; never backfill with future data to justify past trades.
5. Reconcile every block and signature with content-addressed datasets; report coverage denominators, censored outcomes and gaps, not just launch count.
6. Assess provider price only after demonstrated ability to supply the mandatory block/version data; no paid activation without operator approval.

## Deliverable

Capability receipt, reproducible COMPLETE/INCOMPLETE window, slot/transaction coverage ledger, latencies/429/error taxonomy, hashes, and an explicit go/no-go for real-data strategy evaluation.
