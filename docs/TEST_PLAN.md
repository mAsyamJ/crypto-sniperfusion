# Test Plan

Principles: failing-test-first per milestone loop; offline by default (unit / fixture / LiteSVM / replay); network-dependent tests are explicit opt-in and **skip with a printed reason**, never silently pass; unexecuted ≠ passed; fixture-green ≠ mainnet-verified; replay-profit ≠ live-profit. No test ever requires a funded wallet; all keys ephemeral/fixture.

## Layers

| Layer | Tooling | Applies to |
|---|---|---|
| L1 unit | `cargo test`, inline `#[test]` | sniper-core types/math/authz/config; protocol decoders |
| L2 property | `proptest` (onboarding at M3) | integer math boundaries (u64/u128/i128), slippage/fee arithmetic, state-transition totality, parser invariants (never-panic on arbitrary bytes) |
| L3 fuzz | `cargo-fuzz` targets (T1/T2 surfaces) | ix-data decoders, metadata URI handling, Geyser message decode |
| L4 differential (oracle) | Rust harness vs frozen chainstack output on shared fixtures | create/trade event decode, PDA derivations, instruction bytes (where Chainstack has parity scope), exit-sell verdicts |
| L5 SVM integration | `litesvm =0.7` | instruction builders execute: ATA idempotency, v2/v3 trade account layouts, synthetic-migration boundary, PumpSwap v1/v2, Token-2022 transfer-fee paths |
| L6 replay | recorded event corpus, deterministic clock | end-to-end observe→decision→paper-fill; dedup/reorder/gap/commitment-upgrade; crash-restart recovery at every lifecycle stage |
| L7 fault-injection | mock providers/routers | provider disconnect/reconnect, timeout-then-land, duplicate submissions, ack-but-never-lands, blockhash expiry, fee spikes, kill-switch mid-flight |
| L8 benchmark | Criterion + replay harness | decode p99, risk-check p99, decision→sign p99, per-stage latency attribution (M8) |
| L9 shadow (read-only) | live streams, decisions only | observe-mode parity vs chainstack listener output (opt-in, no keys) |

## Per-milestone acceptance (gates)

- **M1**: fmt clean; `clippy --all-targets --all-features -D warnings` clean; `cargo test --workspace` green; observe/replay/paper CLI runs offline; live path unconstructible in default build (compile-assert); authz unforgeable (unit).
- **M2**: decoder parity on shared fixtures ≥ Chainstack behavior for its 4 listener paths; dedup/reorder/gap property tests; differential report committed.
- **M3**: L5 suite covering v2+v3 curve and PumpSwap v1+v2 builders; lifecycle state machine property tests (every transition defined, UNKNOWN always → reconcile); journal round-trip + crash-recovery at each stage; zero duplicate-buy in fault suite.
- **M4**: chainstack parity matrix rows PASS with evidence; CONFIG_MIGRATION tables complete; no Python at runtime (binary size/link check + CI grep).
- **M5**: intelligence fixtures (creator-buy exact math incl. fee/rent separation; unsafe-extension BLOCK; vault-excluded concentration); fail-closed property (any missing mandatory fact ⇒ not ELIGIBLE).
- **M6**: per-venue entry+exit fixtures; venue kill-switch isolation test.
- **M7**: route adapters against recorded HTTP/RPC exchanges; bundle-status lifecycle; circuit-breaker classification tests.
- **M8**: replay determinism (same corpus ⇒ identical decisions+pseudo-fills); latency distributions with context (hardware/region/provider/commit).
- **M9**: full regression re-run; red-team checklist; release-readiness report with explicit unmet-items list (expected non-empty until authorized).

## Static gates (every milestone)

`cargo fmt --check`; `clippy -D warnings`; secret scan (patterns + entropy on diff); `cargo audit`/`cargo deny` when network allows; license check on new deps (no copyleft/unlicensed into production path).

## Regression-triggering events (re-run full matrix)

Any change to: solana/agave/svm crate versions; protocol account tables; fee handling; authz; journal format; config schema; and any pump.fun/PumpSwap program upgrade observed on-chain.
