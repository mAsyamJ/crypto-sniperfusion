# SniperFusion codebase audit — November 3, 2026

**Archive status (2026-11-05):** POINT-IN-TIME — 2026-11-03 audit.

## Scope and evidence

Observed repository mAsyamJ/crypto-sniperfusion at main SHA 2d51a32b4a480618483e2018d9f234d0906594e0. Six commits and four Rust crates. GitHub Actions run 41483671606 passed CI steps for formatting, Clippy, workspace tests, simple regex secret and license scans. docs/evidence/M3.md states 89 Rust tests; they were not independently rerun in this audit. M3c real-program execution remains open. This document does not certify security or profitability.

## Prioritized defects

**P0 — Paper is currently a decision counter.** sniper-app/src/main.rs uses ReplayRunner and CountingHook for Observe/Replay/Paper; CountingHook produces a PaperIntentBuy with zero lamports. sniper-runtime/src/paper.rs implements pricing helpers, but no CLI-connected position ledger, marked-to-market equity, fills, or realized P&L.

**P0 — Authorization cannot yet be trusted as an execution capability.** sniper-core/src/authz.rs has public, serializable RiskAuthorization fields, so it is constructible outside the risk engine. Comment claims it is signed, without signature protection. Bind wallet, chain, quote mint, max total spend, compute-unit price and limit, lamport-denominated total fee, tip, slippage, risk snapshot, expiry and durable nonce. Validate checked arithmetic: valid_from_slot + valid_for_slots can overflow.

**P0 — Journal durability and replay integrity.** journal.rs append flushes buffers without mandatory sync_all before an external side effect. The current recovery routine can categorize a malformed nonfinal line as an apparent crash tail; reject mid-stream corruption, missing or duplicated sequence numbers and bad checksums. Enforce a single durable writer, fail closed on IO error, replay terminal balances and reconcile unknown statuses before new buys.

**P0 — Actual Solana program execution not demonstrated.** Structural account-layout and instruction tests do not prove that Pump.fun/PumpSwap program binaries execute with correct accounts. Implement and record real local SVM or Surfpool tests of executable program state, positive/negative balances, program logs, CU, Token-2022, V2/V3, and migration.

**P1 — Stream gap auto-resumption lacks recovery proof.** EntryGate resumes after future consecutive observations without proving the missed history was backfilled. A critical gap requires verified backfill and actual position/account reconciliation before opening new entries. Transaction-only stream gaps cannot be inferred from slot continuity without a proper slot/root source.

**P1 — Repository visibility.** GitHub metadata reports the repo PUBLIC, while docs/PROJECT_STATUS.md says private. Verify desired state; do not silently change visibility.

**P1 — CI security scans insufficient.** Regex grep is not a history-aware secret scanner or dependency advisory/license audit. Add Gitleaks or equivalent, cargo audit, cargo deny, property/fuzz tests and SBOM/provenance. Keep CI no-secrets and non-broadcasting.

**P1 — Event/version drift.** Add real mainnet fixture corpus for IDL schema changes, event-prefix/truncated variants, CPI attribution and commitment upgrades. Synthetic IDL-perfect fixtures alone do not verify observed mainnet behavior.

**P2 — Risk intelligence and execution routing are not yet implemented.** Creator-linked wallets, dev-buy/bundle detection, Jito/Jupiter routes and multi-DEX execution remain future work (M5–M7).

## Strengths to preserve

- Pinning upstream references; clean-room licensing and official IDLs as source of truth.
- Rust Gen-2 compatibility decision backed by a compile spike and Cargo.lock.
- Four meaningful crates, typed primitives, events, bounded queues, mocked provider adapter and offline order-state tests.
- 320 differential fixtures and 4,792 comparable fields reported with documented oracle anomalies.
- Independent GitHub Actions run succeeded against the inspected head.

## Decision

Do not rewrite the Rust foundation or add another trading framework wholesale. H0: fix authorization/journal/gap behavior. H1: execute real deployed Pump programs locally. H2: wire full paper broker, journal and portfolio accounting. H3: collect real read-only data with creator intelligence. H4: build research and improvement harness. Optimize latency and add other DEXs after correctness and repeatable paper results.

## Source references

- https://github.com/mAsyamJ/crypto-sniperfusion/tree/main/crates
- https://github.com/mAsyamJ/crypto-sniperfusion/blob/main/crates/sniper-app/src/main.rs
- https://github.com/mAsyamJ/crypto-sniperfusion/blob/main/crates/sniper-core/src/authz.rs
- https://github.com/mAsyamJ/crypto-sniperfusion/blob/main/crates/sniper-runtime/src/journal.rs
- https://github.com/mAsyamJ/crypto-sniperfusion/blob/main/crates/sniper-runtime/src/ingest/supervisor.rs
- https://github.com/mAsyamJ/crypto-sniperfusion/blob/main/docs/evidence/M3.md
- https://github.com/mAsyamJ/crypto-sniperfusion/actions/runs/41483671606
