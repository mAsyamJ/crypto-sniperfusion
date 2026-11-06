# D1a — Time-of-Day Probe + create_v2 Re-measurement: the "zero launches" market fact is REFUTED; first COMPLETE cohort containing real launches

Date: 2026-11-06. Branch: `main`. Base HEAD at session start: `54bf3d2` (D0 fixes `f36c1b1` in-tree). Package: **D1a** (`GOAL_PLAN_PROMPT.md` §4). Mainnet strictly read-only; endpoint env-only (`SNF_MAINNET_RPC_URL`, never printed/logged/committed). All outputs `nonpromotable: true`.

Classes: Probe A/B = `MAINNET_READ_ONLY_OBSERVED` (sampled diagnostics); full-window re-capture = `MAINNET_READ_ONLY_CAPTURE`. No strategy evaluation, no P&L (`NOT_APPLICABLE`).

## A. What this package answers

D0 (`docs/evidence/D0-LAUNCH-ATTRIBUTION-AUDIT-20261105.md`) downgraded the prior "zero create attempts" market fact to "zero **legacy** `global:create` attempts; `create_v2` **not measured**". D1a was prescribed as the cheapest next probe: repeat captures at launch-active hours. This session measured first, captured second:

1. **Probe A (quiet window, same 60-slot sample):** `create_v2` attempts were present all along — the "launch activity absent" fact is dead.
2. **Probe B (US-evening hour):** `create_v2` attempts present there too — creation is not hour-gated at the sampled resolution.
3. **Full-window re-capture with the D0-fixed collector:** the identical window previously sealed as `COMPLETE, 0 create events` is re-captured as `COMPLETE` with **232 create events / 232 unique mints** — the first real COMPLETE cohort containing actual launches in repo history. The `SOUL.md` §6 gate for S11+ registration is satisfied at the data level.

## B. Calibration (read-only, 6 RPC calls)

`getEpochInfo` absoluteSlot 465,980,246 @ 2026-11-06T04:48:08Z; anchor `getBlockTime(465625744)` = 2026-11-05T07:45:22Z (consistent with the G0 window receipt); chain cadence ≈ 0.214 s/slot (anchor→tip; implies the 3,001-slot window spans ≈ 10.7 min of chain time, 07:45–07:56 UTC — prior receipts' "62/104 min" figures were wall-clock fetch time, not chain time). Probe-verified: slot 465,831,980 → 2026-11-05T19:59:14Z (−46 s from the 20:00:00Z target). Active-hour probe window: `[465832000, 465848800]` ≈ 2026-11-05 20:00–21:00 UTC. Note: `getBlockHeight` on this provider returned a stale value (443,999,720 @ a Sep-3 timestamp) — `getEpochInfo` is the reliable tip source on this endpoint.

## C. Probe A — create_v2 measured on the exact prior sample (MAINNET_READ_ONLY_OBSERVED)

Tooling: `harness/diag_sample_failed_creates.py` (D0-fixed: both create families, canonical ALT key resolution), N=60, seed 20261105, population = window `[465625744, 465628744]`. The original mutable `ckpt-sweep1.json` is absent from the working tree, so the population was **reconstructed from the immutable completed-window receipt** (`G0-WINDOW-COMPLETE-20261105.md`: 3,001/3,001 verified, 0 gaps ⇒ contiguous range). Reproducibility proof that the identical 60 slots were sampled: this run's aggregates match the original diagnostic's published counts exactly — 17,398 total txs, **3,450 pump txs (624 ok / 2,826 failed, 82% failure)**, top-ok discriminator `e445a52e…` ×845, top-fail `e1f7501e…` ×1,223.

| Measure (60 blocks) | Original diagnostic (legacy-only) | D1a re-run (both families) |
|---|---|---|
| `global:create` attempts | 0 ok / 0 fail | 0 ok / 0 fail (family is dead) |
| `global:create_v2` attempts | *not measurable by that code* | **6 ok / 3 fail** |

**Finding:** launches were occurring in the "zero-launch" window. The prior absence claim was a decoder blind spot, not market structure. The 6 successful `create_v2` were present in the original run's own fetched data (visible only as unlabelled generic-histogram entries below the display cutoff).

## D. Probe B — US-evening hour, same seeded method (MAINNET_READ_ONLY_OBSERVED)

Window `[465832000, 465848800]` (2026-11-05 20:00–21:00 UTC), N=60, seed 20261105:

| Measure | Quiet hour (Probe A) | US evening (Probe B) |
|---|---|---|
| Pump txs in 60 blocks | 3,450 (82% fail) | **8,861 (87% fail)** |
| `create_v2` ok / fail | 6 / 3 | **7 / 9** |
| `global:create` ok / fail | 0 / 0 | 0 / 0 |

**Finding (labeled: 60-block samples):** creation density is roughly comparable across the two sampled hours (6 vs 7 ok); what spikes in the US evening is trading density (2.6×) and failure counts. The US-hours hypothesis may hold for *volume*, but **creation is not hour-gated at this resolution** — the historical zero-create result never needed a time-of-day explanation.

## E. Full-window re-capture — the frontier-unblocking artifact (MAINNET_READ_ONLY_CAPTURE)

Same window, same CLI, D0-fixed collector: `--from-slot 465625744 --to-slot 465628744`, five flock-guarded checkpointed legs (`ckpt-d1a-cap1.json`, mutable working state; legs: 285 + 1,037 + 1,243 + 378 + 58 blocks — two legs ended by provider instability, see §E.1).

| Field | Old capture (2026-11-05, pre-D0 collector) | D1a re-capture (D0-fixed collector) |
|---|---|---|
| Window | `[465625744, 465628744]` | identical |
| Verified blocks | 3,001 / 3,001, 0 gaps | **3,001 / 3,001, 0 gaps** |
| coverage_state | COMPLETE | **COMPLETE** |
| Create events | **0** | **232** |
| Unique mints | 0 | **232** |
| Manifest (write-once 0444) | `manifest-20261105T090624-3953-*` | `manifest-20261106T140626-7720-a3c54ccba14fb2e6.json` |
| Raw evidence (write-once 0444) | empty | `raw_creates-20261106T140626-7720.jsonl` (232 lines) |
| dataset_sha256 | `e3b0c442…` (empty) | `a3c54ccba14fb2e6cf1e3ebceff48967e06fedcceedc62b7ca94e5aeff955f18` |

Cohort composition (census, not sample): **232/232 `create_v2`** (legacy family: zero in-window events); **181 SOL-quote / 51 alternative-quote** launches (the `create_v2` multi-quote feature — Engine B's SOL-only routing will need explicit abstention/handling for the 51 non-SOL pairs); all Token-2022; uniform 1e15 total supply; create times span 07:45:22–07:56:00 UTC (**≈ 1,309 launches/hour** on this slice). Every record carries the decoded `CreateEvent` fields including **launch-time virtual/real reserves** and bonding-curve address — the D2 snapshot capability starts from these.

Manifest honesty notes: `in_window_signatures: 0` with `pages_fetched: 20` — a day after the window, the provider's 20-page `getSignaturesForAddress` cap can no longer walk back from tip to the window, so signature reconciliation returns zero candidates; coverage is unaffected (completeness derives from per-slot `getBlock` evidence, per the manifest's own `completeness_note`). `failed_transactions_in_blocks: 3,435` does not aggregate correctly across resumed legs — do not cite it; the block census (3,001 verified / 0 gaps) and raw create lines (232) are the load-bearing numbers.

### E.1 Engineering during the capture (red-first, per §4 loop)

- **Transport-timeout crash (leg 1):** a provider read timeout (`TimeoutError` inside `http.client` chunked read) escaped `JsonRpc._http_transport` (only `HTTPError` was converted) and killed the leg — 285 blocks kept by checkpoint. Fix: all transport-layer failures (`URLError`, `TimeoutError`, `OSError`, `http.client.HTTPException`, malformed-JSON `ValueError`) now convert to `RpcError`, so the existing bounded-retry + gap-recording path handles them. Red-first: 2 new tests (transport-conversion matrix incl. `IncompleteRead`/`RemoteDisconnected`; composed capture-level persistent-timeout → `ProviderUnavailable` gap, remaining blocks still complete, routed through the real transport path). Suite **132 → 134, exit 0**; `py_compile` clean.
- **Two silent leg deaths (legs 2 and 4):** process gone before budget, stdout empty, no budget-stop file, cause undiagnosed (the leg-2 stderr channel was lost by the background harness; leg 4 had stderr redirected and still shows nothing — consistent with external termination, not a Python exception). Checkpoint + flock made every death resumable with zero corruption; the final leg completed the window cleanly. Operational pattern recorded for future long captures.

## F. Interpretation (labeled)

1. **The S11+ frontier gate is unblocked at the data level.** `SOUL.md` §6 requires "a real COMPLETE cohort containing actual launches" before any S11+ registration. That cohort now exists (232 real launches, block-verified COMPLETE). The H8 registry may lawfully accept real-data trials against it.
2. **The decoder-delta is isolated and clean:** identical window, identical sampled slots, only the counting/decoding code changed (D0 `f36c1b1` + this session's transport fix). Prior "0 create events" windows are decoder-suspect everywhere the old collector ran; this window is re-measured and the earlier artifact is superseded for forward work.
3. **Time-of-day (D1a's original question):** creation is NOT concentrated in US hours at 60-block sample resolution — 6 ok (quiet) vs 7 ok (US evening) per 60 blocks. Cheap implication: future cohort captures need no hour scheduling.
4. **Venue migration (D1b) urgency drops** from "strategic" to "normal parity work": pump.fun launches continue at ≈1.3k/hour via `create_v2`; nothing here supports or refutes LetsBonk/LaunchLab share shifts. M4 stays on its own track with measured evidence.
5. **S11/S12 entry conditions re-price:** 82–87% pump-tx failure rates persist in both regimes — the adverse-stress frame (fee doubling, failed-landing burn, latency) remains the correct evaluation basis.

## G. Open observations (not blockers)

- **`e445a52e51cb9a1d`** — the dominant successful pump discriminator (845× quiet / 1,017× evening samples) is **not** in the official/vendored IDL's 56 instructions (checked exhaustively across instruction and event namespaces). The prior receipt's "trade-family" label was an assumption. Decoder-completeness follow-up: identify before any instruction-level strategy work relies on it.
- Provider instability remains the operational ceiling: two silent leg deaths plus one crash in one session; checkpointing + flock + the new transport conversion make every interruption resumable. Long captures should expect multi-leg execution.
- `getSignaturesForAddress` 20-page cap cannot reach day-old windows from tip (see §E honesty notes) — historical-window candidate discovery is block-scan-only on this provider tier.

## H. What is NOT claimed

- No strategy was evaluated; no trial registered; no P&L computed; S00 NO_TRADE remains champion (`EDGE_NOT_DEMONSTRATED` everywhere); nothing here is promotable.
- Probe A/B create counts are sample-level (60/3,001 and 60/16,801 blocks); the full-window census is §E's manifest, not the probes.
- Time-of-day conclusion is bounded to the two sampled hours/days; it does not exclude multi-day or seasonal effects.
- No claim about venue share, absolute launch volume on other programs, or create-attempt success rates beyond the sampled blocks.
- The dominant-unknown discriminator is documented, not identified.

## I. Next (in order)

1. **D2 — reserve/fee snapshot capability on the found launches** (launch-time reserves already captured in the create events; as-of-decision states for subsequent trading are the remaining piece) → Engine B end-to-end on the real cohort. The 51 non-SOL-quote launches need explicit SOL-routing abstention semantics.
2. **PR-2/D3 — Engine A/B + independent NAV on this identical cohort** (`docs/NEXT_GOAL_H8_G0.md`).
3. **S11 preregistration** becomes lawful via the H8 registry once D2/D3 land; candidate hypotheses must come from the frontier families (`SOUL.md` §5) and clear the rejected-registry check.
4. H-stream follow-up: `wallet_observer.rs` ALT-shape fix (red-first), still open from D0 §8.
