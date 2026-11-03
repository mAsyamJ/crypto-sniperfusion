---
name: solana-execution-reliability
description: Research low-latency Solana data feeds, compute budgets, priority fees, Jito submission, blockhash lifetime, retries and reconciliation without enabling live trades.
---
# Solana Execution and Landing Reliability

Read `docs/evidence/M3c.md`, `crates/sniper-runtime/src/{lifecycle,journal}.rs` and `docs/THREAT_MODEL.md`.

## Workflow
1. Decompose latency into feed, decode, decision, build, submit, land, and record event-age versus the time actually observed.
2. Analyze measured complete-delivery rates, dropped slots, provider divergence, queue delay, Jito acceptance versus landed status and blockhash expiry.
3. Derive priority-fee costs from requested compute units and compute unit price; include tip, network costs, ATA/rent and failed transaction losses.
4. Exercise chaos cases: disconnect, stale quote, unknown submission, duplicate requests, process crash after signing and rollback. No blind retry.
5. Reconcile authoritative account balances, claimable accumulators and position state before reopening trading permissions.
6. Benchmark improvements by **after-cost landing-adjusted value**, not a single feed percentile.

## Required deliverable
Deterministic fault-injection matrix, latency provenance, account reconciliation receipts, cost model and blockers. No funded keys or remote broadcaster tests.
