# Deep Research Review → Engineering Decisions (2026-11-03)

Input: user-provided `deep-research-report (3).md`, `harness/docs/AI_STRATEGY_HARNESS_V1.md` and `harness/docs/NEXT_AGENT_RESEARCH_PROMPT.md`. This document is a derivative engineering decision record, not a claim of having replicated the report's third-party observational data.

## Verified code problems (P0)
1. `ReplayRunner::run` ingested all events then priority drained `MigrationObserved` ahead of earlier creation/trade events. This branch uses a per-event bounded handoff and preserves input observation order. It does **not** invent chain transaction indices: a future canonicalizer must verify those before chain-order replays.
2. `PaperBroker` previously paired a sell with `open_entries.remove(0)` independent of mint. This branch matches FIFO within a mint, tracks remaining lot quantity and apportions partial entry and exit expenses. Integration tests cover A-buy/B-buy/B-sell, partial exits and ledger reconstruction.
3. Broker fixed 70ms landing and globally assumed 100-bps protocol fee/25-bps slippage remain research limitations; the V0 synthetic batch is explicitly not promotion evidence.
4. H4 `shadow` remains NOT_STARTED and independent H2 evaluator still covers CLOSED round trips only. Neither may be presented as complete portfolio evidence for real markets.

## Research conclusions (not implemented fact)
- Latest Pump.fun and PumpSwap paths include synthetic migration, versioned account layouts and quote/fee variants. Fork receipts for the complete matrix are pending, beyond M3c's tested V3 path.
- Third-party launch graduation and wallet-following studies are useful hypotheses, not unconditionally portable statistical priors. Reconstruct all creation events and audit collection gaps.
- No current strategy has proved positive repeatable out-of-sample after-cost expected value.
- Default action on missing safety/data/liquidity is abstention. Risk approval remains separate and non-forgeable.

## New running surface
- `harness/research_batch.py` is a deterministic local 11-policy **surrogate demonstration**, using strict JSONL observations and integer accounting; it has zero mainnet integration or signing capability.
- `harness/tests/test_research_batch.py` contains negative controls and reproducibility checks.
- `.agents/skills/*/SKILL.md` are human/agent playbooks, not installed AI models nor autonomous jobs.
- `harness/docs/RESEARCH_LOOP_RUNBOOK.md` is the operator workflow and explicit future milestone boundary.

## Known residuals
- Paper execution still depends on static fee/latency defaults and untested claimable accumulator semantics.
- No valid complete mainnet creation cohort, provider completeness guarantee or shadow window has been collected by this branch.
- V0 uses an uncalibrated token-price/liquidity model, not the actual Pump.fun/PumpSwap reserve model. Never optimize production strategy weights on these receipts.
- Rust tests are to be verified by GitHub Actions; any unexecuted local test is UNKNOWN, not PASS.
