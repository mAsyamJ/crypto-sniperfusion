# D0 — Launch-Attribution Completeness Audit (2026-11-05)

**Status:** ACTIVE (write-once receipt) · **Session package:** D0 (`GOAL_PLAN_PROMPT.md` §4, "FIRST NEXT PACKAGE") · **Branch:** `main` · **Base HEAD:** `fb1a762` · **Class:** `VERIFIED_CODE` (decoder audit + red→green fixes) · **Market claims:** none — P&L `NOT_APPLICABLE`.

## 1. Identity

Audits launch-attribution completeness of the G0 capture/diagnostic pipeline before any D1 (time-of-day cohort), M4 (venue adapter), or S11+ (strategy) work, per the D0 charter. Two statically-confirmed decoder defects and one attribution weakness were found and fixed red-first; the official pump.fun IDL was re-pinned against the vendored copy; the completed 3,001-block window was NOT re-evaluated (preconditions absent — §5). This receipt **supersedes the "zero create attempts" wording** in `G0-WINDOW-COMPLETE-20261105.md` (that file stays immutable): the sampled diagnostic measured only the LEGACY `global:create` family, so the correct statement is **"zero legacy `global:create` attempts; `create_v2` attempts not measured"**.

## 2. Confirmed defects (file:line at base HEAD `fb1a762`)

1. **ALT key flattening — `harness/launch_cohort_capture.py:513-515`.** `keys += list((meta.get("loadedAddresses") or []))` ran `list()` over the real wire shape — a dict `{"writable":[…],"readonly":[…]}` — yielding the literal key names `["writable","readonly"]`. Any versioned tx whose pump program id was ONLY ALT-resident was silently skipped BEFORE the log scan. Effect: creates could only be missed, never invented — every prior "0 create events" window result is bounded to "among successful txs whose pump program id was static-key-visible". The sibling diag script already flattened correctly (`diag_sample_failed_creates.py:66-67`), and the test fixtures enshrined the wrong list shape (`loadedAddresses: []`).
2. **Create-family blind spot — `harness/diag_sample_failed_creates.py:38,87-88`.** The failed-instruction diagnostic counted create attempts only for legacy `global:create` (`181ec828051c0777`); `create_v2` (`d6904cec5f8b31b4`) attempts fell into the generic discriminator histogram. The diagnostic was also a linear top-level script (unimportable, zero tests).
3. **Log-attribution weakness — `harness/launch_cohort_capture.py:187-221` (docstring overstated).** `Program log: Instruction: <Name>` lines are program-agnostic (every Anchor program emits them); the gate attributed `Program data:` payloads to the most recent create-named marker regardless of emitter, and ANY program's marker could open or clear the gate. The Rust decoder already did frame-stack attribution (`crates/sniper-protocols/src/logs.rs::LogAttribution`); the Python collector did not.

## 3. Fixes (red→green; commit `f36c1b1`, local only)

- `harness/launch_cohort_capture.py`: new `resolved_account_keys()` (canonical static + ALT-writable + readonly index order; legacy list tolerated defensively) replacing the defective call site; new `build_log_attribution()` mirroring Rust `LogAttribution` semantics exactly (invoke-push / success|failed-pop; frame lines carry None); `extract_creates_from_logs()` rewritten so only pump-attributed markers open/keep the gate and only pump-attributed payloads decode — other programs' markers neither open nor clear it; docstring corrected.
- `harness/diag_sample_failed_creates.py`: restructured into an importable pure core (`discriminator()`, `resolved_keys()`, `scan_block()`, `aggregate()`, `empty_totals()`, `main()` under `if __name__ == "__main__"`); counts create attempts for BOTH families (`create`, `create_v2`) ok/fail, outer+inner instructions; histograms now annotate both discriminators; same CLI contract and seeded sampling; `SNF_MAINNET_RPC_URL` read only inside `main()` (never at import).
- `harness/tests/test_launch_cohort_capture.py`: fixture corrected to the real wire dict shape; new `VersionedTxAltResolution` (pump id in ALT-writable/readonly must be scanned; canonical key order) and `LogAttributionParity` (foreign marker+payload rejected; foreign marker must not clear pump's gate; foreign CreateV2 frame rejected) classes.
- `harness/tests/test_diag_sample_failed_creates.py` (new): discriminator pins vs the Rust table (`181ec828051c0777`, `d6904cec5f8b31b4`, `66063d1201daebea`, `9beae792ec9ea21e`), both-family ok/fail counting through outer+inner instructions with an ALT-resident program id, canonical key order, import side-effect freedom.

## 4. Reproduction (exact commands, exit codes)

| Step | Command | Result |
|---|---|---|
| Baseline (pre-work) | `python3 -m unittest discover -s harness/tests` | `Ran 119 tests … OK` (exit 0) |
| RED (tests first, at `fb1a762` code) | `python3 -m unittest discover -s harness/tests` | `Ran 126 tests … FAILED (failures=5, errors=2)` (exit 1) — ALT-writable 0≠1, ALT-readonly 0≠1, 3 attribution failures, `resolved_account_keys` missing, diag module unimportable |
| GREEN (after fixes) | `python3 -m unittest discover -s harness/tests` | `Ran 132 tests … OK` (exit 0) — baseline 119 + 13 new, zero regressions |
| Compile check | `python3 -m py_compile harness/launch_cohort_capture.py harness/diag_sample_failed_creates.py` | exit 0 |
| IDL cross-check | fetch `raw.githubusercontent.com/pump-fun/pump-public-docs/main/idl/pump.json`, compare against `upstream/pump-docs/idl/pump.json` (pinned `639b87f`, 2026-10-13) | identical: 56 instructions each, `only-in-official: []`, `only-in-vendored: []`, `discriminator mismatches: []`; address both `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P` |

No Rust code changed; no cargo cycle required.

## 5. Re-evaluation of the completed window — BLOCKED

D0 charter item 3 conditions re-evaluation on **local checkpoint AND authorized read-only RPC**. At session preflight BOTH preconditions failed: `ckpt-sweep1.json` / `manifest-20261105T090624-*` are absent from the working tree (`find` over the repo; `harness/runs/launch-cohort/` holds only Nov-3/4 `ckpt-g0*`, all `creates: []`), and `SNF_MAINNET_RPC_URL` is UNSET. Per the charter, **no new observation was fabricated**; the offline audit and fixes above are the deliverable. The corrected diagnostic is ready to re-run (same seed `20261105`, N=60) the moment both preconditions hold — it now counts both creation families.

## 6. IDL / discriminator pinning (official-source verification)

- Instruction discriminators are Anchor `sha256("global:<name>")[:8]`: `create` = `181ec828051c0777`, `create_v2` = `d6904cec5f8b31b4`, `toggle_create_v2` = `1cffe6f0ac6bcbab`, `migrate` = `9beae792ec9ea21e`, `migrate_v2` = `bbcb121fceedfe29` — byte-identical in the official IDL JSON discriminators, the Rust pins (`curve.rs::discriminators_match_idl_derived_constants`), and (after D0) the Python diagnostic. Event discriminators are a DIFFERENT namespace, `sha256("event:<name>")[:8]`: `CreateEvent` = `1b72a94ddeeb6376` (collector constant, official IDL, Rust `events.rs` all agree). Instruction-level `create`/`create_v2` (what a tx ATTEMPTS) and event-level `CreateEvent` (what a tx ACHIEVED) must never be conflated.
- Single program id for pump.fun bonding curve (no V2 program exists): `6EF8…F6P` in official IDL = vendored IDL = `registry.rs`. PumpSwap (`pAMM…XEA`) graduation/migration remains event-inferred (`CompleteEvent`), untouched by D0.
- **`create_v2` applicability to the window `[465625744, 465628744]`:** the official Coin Creation docs (`upstream/pump-docs` @ `639b87f`; IDL identical at official HEAD) document `create_v2` as THE current creation instruction (default SOL pairing, USDC, and pump-coin quote pairing; `create_v2_and_buy` helpers; cashback deprecated). No activation-slot gate is documented for it; `toggle_create_v2` (`1cffe6f0ac6bcbab`) is an admin on/off control whose current on-chain state was not readable offline. Therefore `create_v2` was plausibly the DOMINANT — possibly the only — creation family during the window, and the unmeasured family is exactly the one the old diagnostic could not see. This is why the "zero create attempts" claim is downgraded rather than carried over.

## 7. Evaluation — block completeness vs decoder completeness (separated per charter item 4)

- **Block completeness of the 2026-11-05 window STANDS** (`G0-WINDOW-COMPLETE-20261105.md`): 3,001/3,001 blocks verified is a coverage fact independent of decode logic.
- **Decoder completeness was overstated.** Corrected statements: (a) "0 create events" → "0 create events among successful txs whose pump program id was static-key-visible (ALT-resident pump txs were skipped by the old flattening)"; (b) "zero create attempts, launch activity absent not failing" → "**zero legacy `global:create` attempts** in the 60-block sample; `create_v2` attempts **not measured**; 'absence' is therefore NOT established for the create_v2 family". The 82% pump-tx failure rate and ~57 pump txs/block secondary-trading observations are unaffected (the diagnostic's key resolution was already correct).
- **Venue-migration (LetsBonk/LaunchLab) hypothesis: unchanged, unproven.** D0 neither confirms nor refutes it. Per charter item 5, no M4 adapter is justified by this audit alone; measured evidence (D1b protocol) is still required.
- Family verdict table (decoder coverage, post-fix): legacy `create` — covered (events + attempts); `create_v2` — covered (events + attempts); ALT-resident pump txs — covered (canonical key resolution); CPI/log attribution — pump-frame-gated (Rust-parity); PumpSwap migration attribution — pre-existing, untouched.

## 8. Open defects / limitations

- **`crates/sniper-runtime/src/wallet_observer.rs:178-225` (NEW finding, DEFERRED):** `detect_venue` expects `loadedAddresses` as an array of objects with a `pubkey` field — the real shape is the writable/readonly dict — so ALT-loaded keys are always empty there, and its `instruction_programs` fallback reads a `programId` string field that `encoding:"json"` receipts do not carry (`programIdIndex` only). Wallet-venue attribution rests on static `accountKeys`. Same defect class as §2.1 but on the wallet-alpha venue path, NOT launch attribution; left unfixed to keep D0 one bounded package. Recommended: its own H-stream red-first fix (its tests at `crates/sniper-runtime/tests/wallet_observer*.rs:56,34` also pin the wrong shape). `sniper-protocols/tests/fork_execution.rs:187` pins `maxSupportedTransactionVersion: 0` for the LOCAL Surfpool fork only — noted, not a mainnet-capture risk.
- The completed window's true `create_v2` attempt count is unknown until the §5 re-evaluation runs.
- `getSignaturesForAddress` 20-page cap and provider ≈0.8 getBlock/s throughput are unchanged operational ceilings (`docs/RPC_PROVIDERS.md`).

## 9. What is NOT claimed

- No claim that launches occurred (or did not occur) via `create_v2` in the window — that is unmeasured (§5, §7).
- No claim that launches "disappeared", and no claim of venue migration — D0 explicitly does not establish either.
- No new market data was captured; no strategy was evaluated; no P&L was computed (P&L `NOT_APPLICABLE`); S00 NO_TRADE remains champion with `EDGE_NOT_DEMONSTRATED` everywhere; nothing here is promotable.
- Rust decoder behavior is unchanged (audit verified it, did not modify it); the `wallet_observer.rs` defect is documented, not fixed.
- No push occurred; mainnet was never contacted (RPC env UNSET all session).
