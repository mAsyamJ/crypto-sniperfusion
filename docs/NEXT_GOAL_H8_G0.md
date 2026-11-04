# Next Mission — Unblock G0 and build H8 research council

**Reviewed baseline:** 2026-11-04 `main@69380c8`. Reinspect head and CI before any changes. This document adds a plan and does NOT claim the planned code exists.

## North-star and stop condition

Achieve one independently verified, `coverage_state=COMPLETE` mainnet read-only Pump.fun creation cohort with credible point-in-time state; run S00–S10 through both economic engines and independent NAV oracle, then implement an experimentally sound H8 challenger/council. If the provider cannot deliver complete getBlock evidence, STOP alpha claims, record BLOCKED and continue offline correctness work. Mainnet broadcasting stays disabled.

### PR-1 — Block-serving RPC capability & bounded collector (P0)

**Files:** `harness/launch_cohort_capture.py`, `harness/tests/test_launch_cohort_capture.py`, `docs/evidence/H6.md`, new bounded data-provider capability test/receipt.

Design: genesis-verify; probe getBlock on old and recent finalized slots with version support; distinguish 429, 403, 504, UnsupportedTxVersion, Pruned and SkippedSlot; fail fast if completeness is impossible. Add time/request budgets, cursor/checkpoint/resume without duplicate blocks, capped concurrency and latencies, tamper-proof output receipts. Use `maxSupportedTransactionVersion` only where supported, never assume provider compatibility. Reconcile block-level indexes; no optimistic completeness from signatures. Do not expose endpoint credentials.

**Gate:** small complete 1–4 slot window, zero missing block evidence and independent signature reconciliation, all output provenance hashes, 0 agent send transactions; verified read-only provider permissions. A provider failure becomes `BLOCKED_RPC_CAPABILITY`, not a green run.

### PR-2 — Real snapshot economics (P0)

**Files:** `crates/sniper-strategy/src/engine/protocol.rs`, `src/fees.rs`, `src/oracle.rs`, `harness/launch_cohort_capture.py`, `harness/docs/RESEARCH_CYCLE_V1.md` and tests.

Link every decision to as-of fee_config/program version, bonding-curve or PumpSwap reserves, actual market phase, and observable event latency. Refuse unobserved reserves and retroactive fee tiers; report abstentions and known missing-state counts. Compare Engine A surrogate vs Engine B protocol quote side-by-side on identical G0 cohort and isolated cash. Independent accountant must reconstruct ending cash and marked executable NAV at every decision.

**Gate:** deterministic same-dataset results, legitimate fee/config provenance, no synthetic fallback silently treated as mainnet evidence, full ledger reconciliation, negative controls and an honest S00 comparison.

### PR-3 — H8 experiment council and one-use holdout (P1)

**Files:** new `harness/research_registry.py`, tests, `harness/docs/RESEARCH_CYCLE_V2.md`, critic receipt schema; extend `harness/agent_mission.py`.

Implement typed hypothesis preregistration, immutable 1–20 trial ledger, content-hash data partitioning, creator/day-block temporal split and label horizon embargo. Seal holdout with independent reviewer and one-use access, with a durable cross-process open counter and fault-injection tests. Council agents may author hypothesis JSON and offline challenger patch; cannot read holdout labels, change live config, use signer, call mainnet sends, or access raw key files. Independent critic's rejection/acceptance is a separate artifact. Correct for multiple comparisons and report uncertainty. A simple Python `if` flag is NOT a secure sandbox boundary.

**Gate:** bounded cycles enforced structurally; second holdout-open denied and audited even after crash; at least 12 established negative controls retained; all runs deterministic and non-promotable until real G0+shadow evidence. No LLM interface to mainnet signer.

### PR-4 — Genuine read-only shadow (P1, cannot be instant)

Observe at least seven consecutive days of real launches with provider event-age, data gaps, token lifecycle, forecast prices, exposure, executable exits and pessimistic landing costs. Produce daily immutable health summaries. Do not declare shadow done from historical replay or synthetic data. Human may review for paper-only promotion after real out-of-sample validation.

## Roles and proof

Maintainer sets PR contracts; coverage engineer validates denominator; protocol engineer validates accounts/fees; trader proposes one falsifiable mechanism; quant runs unchanged sample and independent accounting; critic sees holdout once; security vetoes privileged signer/network access.

Every PR receipt: HEAD, origin/pin, fixtures and data hash, exact commands+exit codes, source coverage, failure counts, latency p50/p95/p99, financial results separated by evidence class, errors, residual risks and next blocker.

## Read-only environment

All agent-facing environment should use unprivileged account; shell commands must never print the secret. The operator's current RPC key remains unchanged. Do not feed the key to LLM prompts, public CI, logs or agent run manifests. Distinguish operator-owned read-only endpoint from test faucet wallet and any production signer.

This roadmap does not authorize purchase of an RPC plan, executing mainnet trades, or unattended strategy iteration.
