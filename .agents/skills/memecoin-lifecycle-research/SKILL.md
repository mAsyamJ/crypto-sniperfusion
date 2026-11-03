---
name: memecoin-lifecycle-research
description: Reconstruct the complete creation-to-inactivity lifecycle of newly launched Solana memecoins with unbiased cohorts and versioned event timelines.
---
# Memecoin Lifecycle Intelligence

Read `harness/docs/MEMECOIN_RESEARCH_QUESTIONS.md` and `harness/docs/AI_STRATEGY_HARNESS_V1.md`.

## Workflow
1. Define denominator as all observed canonical Pump.fun creation events, **not** only trending or graduated coins. Audit missing slots, duplicate events, provider restarts and censoring.
2. Model CREATED → EARLY_CURVE → ACTIVE_CURVE → NEAR_COMPLETION → CURVE_COMPLETED → MIGRATION_PENDING → PUMPSWAP_OPEN → POST_MIGRATION/DECAYING/INACTIVE. CENSORED is a data quality label, never proof of coin death.
3. Preserve both canonical chain order (slot, tx index, instruction coordinates, event ordinal) and actual provider observation order, with commitment upgrades.
4. Detect V3 crossing trade events without double counting, creator initial inventory, quote exposure and whole-position sellability.
5. Segment cohorts by date, launch variant, quote asset, creator, buyer-cluster evidence, and market regime. Annotate confidence in wallet-linkage heuristics.
6. Publish a lifecycle state/coverage report; never label a migration as organic demand without independent evidence.

## Required deliverable
Versioned per-mint timeline, raw event provenance, completeness/censor flags and measurable hypotheses with independently sourced citations.
