# Project Status

**Repository: `mAsyamJ/crypto-sniperfusion` (PUBLIC — verified 2026-11-03, owner decision: keep public).** Project name: **SniperFusion**. Chain-agnostic naming on purpose: Solana is the only implemented chain today; every doc claim about capabilities remains Solana-scoped until multi-chain code lands behind its own milestone gates.
**Operational consequence of PUBLIC:** strategy logic, risk thresholds, and evaluation results are world-readable. Enforced invariants that make this acceptable: no keys/secrets/`.env` in history (H0.4 scans enforce), no funded credentials, no infrastructure endpoints in docs. Revisit visibility before any live-trading authorization — see ADR-0002.

Living document; update at every milestone close. Format: state, evidence pointer, next.

## Milestones

| Milestone | State | Evidence |
|---|---|---|
| M0 — Audit, baseline, dependency selection, protocol verification | **COMPLETE** (2026-11-02) | `docs/evidence/M0.md` |
| M1 — Functional Rust core, config, risk authz, safe replay runtime | **COMPLETE** (2026-11-02) | `docs/evidence/M1.md` |
| M2 — Pump.fun event decoding, streaming interfaces, fixture parity | **COMPLETE** (M2a differential 320/320; M2b providers+supervision+cache) | `docs/evidence/M2.md` |
| M3 — V2/V3 + PumpSwap lifecycle | **COMPLETE incl. M3c** (real local execution on Surfpool fork, 2026-11-02) | `docs/evidence/M3.md`, `docs/evidence/M3c.md` |
| H2 — Real paper broker (event-sourced) | **COMPLETE** (2026-11-02) | `docs/evidence/H2.md` |
| H4 — Improvement harness CLI skeleton | **COMPLETE** (2026-11-02; H3 shadow window intentionally not started) | `docs/evidence/H4.md` |
| M4 — Chainstack parity scope closure | PLANNED | — |
| M5 — Token intelligence + DevFlowMonitor | PLANNED | — |
| M6 — Multi-DEX + copy trading | PLANNED | — |
| M7 — Execution routes + reconciliation + fault injection | PLANNED | — |
| M8 — Replay/backtest/benchmarks | PLANNED | — |
| M9 — Hardening + release readiness | PLANNED | — |

## M0 results (summary — verbatim detail in evidence file)

1. **Security audit**: no funded credentials in workspace; upstream install surfaces safe; m8s ships apparent live credentials (flagged, unused, uncopied).
2. **Chainstack baseline**: 38 verifier scripts discovered, **38 executed, 38 passed, runner exit 0**, isolated `uv sync --frozen` env.
3. **Dependency spike**: Gen-2 (solana-sdk 2.3.1 + litesvm 0.7 + spl-token-2022 9) **GREEN end-to-end**; Gen-4 **RED** (MSRV wall + litesvm 0.18.1 broken vs agave-4.4 lockstep). Decision: Gen-2 (ADR-0003). Toolchain updated 1.98.0 → 1.99.1 to give Gen-4 a fair trial (kept).
4. **Protocol audit** (`pump-docs` @ 639b87f): Chainstack's vendored IDLs lack the official **V3 curve trades** and **PumpSwap V2 trades** (17-account families) + multi-hop — Rust scope therefore exceeds Chainstack parity on these (G-1/G-2). Fee schedules are on-chain (`fee_config`) — no hardcoded rates. Synthetic migration + negative i128 `virtual_quote_reserves` captured with formulas and PDAs.

## M1 results (summary — verbatim detail in evidence file)

Workspace: 4 crates (core/protocols/runtime/app), solana-sdk =2.3.1 per ADR-0003, lockfile committed, rustc 1.99.1 pinned, build jobs=2. Gates all green: fmt, clippy×2 (`-D warnings`), tests×2 (8 suites each, 0 failures). CLI demonstrates offline replay/paper end-to-end (7 events → 2 dupes deduped → 5 decisions) with honest refusals for observe-streaming (M2) and live (4-gate + M7). Dual event identity, 3-tier shedding bus, crash-tolerant journal, base58 fixture serde landed.

## Open items / blockers

- None blocking M2. Watch item: host RAM — build jobs capped at 2 (held throughout M1).
- Deferred: `pump-rust-client` 0.5.0 evaluation (M3); Yellowstone bump headroom (16→17) harmless.
- Gen-4 revisit trigger documented in ADR-0003 (litesvm ≥0.19 or mollusk maturity).

## M2a results (summary — verbatim detail in evidence file)

Event decoding + log attribution + differential harness complete: 320 synthetic fixtures (8 kinds incl. old-format truncated), oracle = chainstack IDLParser; comparator result ALL MATCH (4792/4792 comparable fields, exit 0). Two oracle limitations discovered, documented, and excluded with rationale: D-1 vec<defined> unsupported with silent offset misalignment (oracle post-shareholders fields are garbage), D-2 EOF artifact synthesis. Rust decoders are prefix-tolerant per-field (decoded_fields counts), garbage-proof (property test), and sha256-verified against both IDLs. All gates green (fmt, clippy ×2, tests ×2).

## Open items / blockers

- M2b (live provider adapters: WSS logs/blocks, Yellowstone gRPC, SubscribeDeshred detection, reconnect/gap/reorder against real streams, blockhash cache) — NOT STARTED; requires opt-in read-only provider credentials. First task next session.
- PostCompleteBuyEvent layout (synthetic migration pool leg) — pending M3 with SYNTHETIC_MIGRATION.md quoting work.
- Host RAM watch item unchanged (jobs=2 held).

## M3 results (summary — verbatim detail in evidence file)

PDA module (doc-constants verified), exact integer math (curve constant-product, i128 signed effective reserves, bounded-output invariant), six instruction builders (curve v3×3 @17 accounts, v2 buy 27 / sell 26, PumpSwap sell_v2 @17) with doc-table structural tests, order lifecycle machine with journal write-ahead + full crash-recovery test matrix (UNKNOWN mapping, duplicate-submission rejection, entry gating), paper fill engine with fee-in/out round-trip economics. Gates all green (fmt, clippy×2, tests×2: 89 tests across workspace).

## Open items / blockers

- **M3c (next session, first task)**: LiteSVM execution vs real pump.fun/PumpSwap program binaries — needs opt-in program-ELF fetch or committed fixture; PostCompleteBuyEvent layout from synthetic-migration fixtures.
- M4 then proceeds: LetsBonk/LaunchLab parity scope, exit-strategy parity, CONFIG_MIGRATION.md, runtime-Python-independence proof.

## Next actions (M3c → M4)

1. M3c: opt-in program-ELF fixture fetch script (read-only RPC), LiteSVM harness executing builders end-to-end, PostCompleteBuyEvent decode.
2. M4: Chainstack parity closure per CAPABILITY_MATRIX rows C2–C24; `docs/CONFIG_MIGRATION.md`;LetsBonk/StonkFun adapter scope decision.
3. Carry: `pump-rust-client` 0.5.0 build-helper evaluation.

## H-mission results (2026-11-02, branch codex/harness-m3c-security)

- **H0** (`bd5c3b0`): sealed `AuthorizedOrder` trust boundary, durable
  checksummed single-writer journal with crash-tail repair, backfill-proof
  EntryGate, history-aware secret scan + cargo-deny CI, docs visibility fix.
- **H1/M3c** (`b305106`): Surfpool v1.7.0 pinned (sha256 recorded); real
  local `buy_v3`/`sell_v3` + negative control against the deployed pump.fun
  ELF on a loopback mainnet fork; official-IDL conformance fixes
  (`partial_fill` arg, account mutability, ATA prerequisite); live-PDA
  ground-truth tests; deployed-program economics documented (volume
  accumulator SOL routing, quote-era event semantics open).
- **H2** (`f38f1cc`): event-sourced PaperBroker (fills/fees/rent/slippage,
  no-lookahead, migration, ledger crash-recovery, deterministic export);
  CLI paper mode rewired with reconciled P&L receipt accepted by the
  independent evaluator (broker realized 2,080,057 vs evaluator net
  2,068,057 = exactly the separately-booked 12,000 network+tip).
- **H4** (harness skeleton): `harness/snf-harness` — doctor/verify/replay/
  fork/shadow/evaluate/compare/promote with write-once run manifests;
  promotion requires the full gate chain INCLUDING the ≥7-day shadow window
  and never enables live trading. Tests: 167/167 both feature sets.
- **H3**: NOT STARTED (needs ≥7 days read-only observation; the harness
  `shadow` command refuses to fabricate it).
