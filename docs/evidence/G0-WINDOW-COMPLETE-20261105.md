# G0 Window Complete — 3,001-Block COMPLETE Cohort, Zero Launch Attempts

Date: 2026-11-05. Branch: main (`b90d7e5` collector with flock; sweep executed in four lock-guarded legs). Mainnet strictly read-only; endpoint env-only. All outputs `nonpromotable: true`.

## A. What completed

The first at-scale block-verified Pump.fun creation-cohort window in repo history:

| Field | Value |
|---|---|
| Window | `[465625744, 465628744]` (3,000 slots ≈ 104 min mainnet) |
| Verified blocks | **3,001 / 3,001** — every slot block-verified |
| Gaps | **0** |
| Signatures without block evidence | 0 |
| coverage_state | **COMPLETE** |
| Manifest (write-once 0444) | `harness/runs/launch-cohort/manifest-20261105T090624-3953-e3b0c44298fc1c14.json` |
| Raw evidence | `harness/runs/launch-cohort/raw_creates-20261105T090624-3953.jsonl` (empty: zero creates) |
| evidence_class | MAINNET_READ_ONLY_CAPTURE; genesis verified; dataset sha `e3b0c442…855` |
| **Create events (successful launches)** | **0** |
| **Unique mints** | **0** |
| Failed transactions in blocks (all programs) | 9,519 |

Executed as bounded checkpointed legs (540 s / request budgets, `ckpt-sweep1.json` resume), under the new checkpoint flock (`b90d7e5`). PR-1's "complete cohort at meaningful scale" gate: **ACHIEVED at the coverage level** — the cohort is complete and honest; it simply contains zero launches.

## B. Sampled diagnostic — were launches attempted and failing? (MAINNET_READ_ONLY_OBSERVED)

The checkpoint persists signatures, not payloads (by design), so a 60-slot uniform random sample (seed 20261105) of the completed window was re-fetched and instruction-scanned (versioned-tx ALT key resolution; outer+inner instructions; anchor discriminator `sha256("global:create")[:8] = 181ec828051c0777`):

| Measure (60 blocks, 17,398 txs) | Value |
|---|---|
| Pump-program-invoking txs | 3,450 (624 ok / 2,826 failed) |
| Program-level failure rate | **82%** (failure-storm regime persists) |
| `global:create` instructions — successful | **0** |
| `global:create` instructions — FAILED | **0** |

**Finding: zero create attempts, not zero successful creates.** Launch activity on this program in this window is *absent*, while secondary trading on existing tokens is heavy. Dominant successful discriminator `e445a52e…` (845×) and failed `e1f7501e…` (1,223×) are trade-family instructions; the create family does not appear at all.

Note on the signature sweep: `in_window_signatures = 19,970` equals the 20-page `getSignaturesForAddress` cap — a candidate-discovery ceiling, not an activity census. Block-level evidence (complete) shows true program activity ≈ 57 pump txs/block (sampled). Coverage is unaffected: completeness derives from blocks, never signatures.

## C. Interpretation (labeled: single window, single time-of-day)

1. **S11/S12 are blocked by market structure, not machinery.** The collector, budgets, checkpointing, locking, and reconciliation all work at scale; there was simply nothing to capture. Zero launches in 104 minutes with ~172k program-invoking txs (extrapolated) is a strong absence signal for this window.
3. **Time-of-day clustering is untested** — this window spans ~07:00–08:50 UTC. Repeat captures at different hours/days are the cheapest next probe.
3. **Launchpad migration is now a first-class hypothesis** (M4 decision becomes strategically urgent): if new-launch volume moved to LetsBonk/LaunchLab/other programs, G0 must widen program coverage or the S11/S12 frontier measures a dead venue. Clean-room scope: `docs/CAPABILITY_MATRIX.md` M4 rows already anticipate LetsBonk adapters.
4. **The 82% program-level failure rate re-prices landing costs** for every entry family; SOUL.md adverse-stress gates (+100% fees, failed-tx burn) are the correct evaluation frame, and this figure should inform their calibration once real launches exist.

## D. Engineering notes from this session (receipts already pushed)

- Budget-stop summary KeyError regression — found live, fixed, regression-tested (`268654f`).
- Checkpoint flock mutual-exclusion — zombie-sweep incident (two writers corrupted a 47 MB checkpoint; recovered losslessly via `raw_decode`), fixed with `CheckpointLocked` fail-fast + kernel-released lock, 2 tests (`b90d7e5`).
- Operational: `--max-requests` is cumulative across resume legs; the 46 MB-per-save checkpoint rewrite (852k stored signatures) is a known scaling cost, acceptable at 3k-slot windows, to be revisited before 10k+ windows.

## E. What is NOT claimed

- No strategy evaluation occurred; no cohort with launches exists yet; S00 remains champion (`EDGE_NOT_DEMONSTRATED`).
- "Zero launches" is a fact about *this window*, not a universal claim about pump.fun launch volume; alternative venues and hours are untested.
- The diagnostic sample is 60/3,001 blocks (uniform, seeded); its zero-create-attempt finding is sample-level evidence consistent with the full-window zero create events.

## F. Next (updates reflected in GOALS.md §2 / STRATEGY_REGISTRY / RPC_PROVIDERS)

1. Repeat window captures at launch-active hours (US-hours hypothesis) — cheapest discriminator between time-clustering and venue migration.
2. M4 launchpad-coverage decision (LetsBonk/LaunchLab program IDs into the collector's program set) — now strategic, not just parity.
3. Reserve/fee snapshot capability (Engine B prerequisite) — build it against trading-active tokens now, so it is ready the day a launch appears.
4. Failed-create-attempt scan as a standing collector output (log the create-discriminator count among failed txs per window).
