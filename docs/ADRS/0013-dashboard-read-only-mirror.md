# ADR-0013: Control Tower Dashboard — Read-Only Mirror Architecture

Date: 2026-11-05. Status: accepted.

## Context

The owner's historical dashboard mission (original root prompts, archived under `docs/history/dashboard/` on 2026-11-07; published via PR #12) requires a local, trustworthy observability surface over SniperFusion's verified outputs. Verified ground truth at HEAD `de1a8be` (audit receipt `docs/evidence/DASH-01-20261105.md`): the corpus on disk is hash-sealed write-once artifacts (`run.json` PnL V2 receipts, G0 manifests, snf-harness manifests, H8 hash-chained registry, Harness² summaries) with **no daemon, no heartbeat, no NAV series, and no wall-clock timestamps on research trade records**. The owner override forbids any dashboard-side market/accounting data path.

## Decision

Build the Control Tower as a **read-only mirror** with a strict producer/consumer boundary:

1. **Two new crates + one frontend app:** `crates/sniper-observability` (allowlisted-root discovery, sha256 verification, shape-aware parsers, disposable SQLite WAL projection; no network) and `crates/sniper-dashboard-api` (Axum, loopback-only, GET-only, Host/Origin enforcement) serving `apps/dashboard` (React+TS+Vite, Radix+shadcn-styled-with-DESIGN-tokens, TanStack, Lightweight Charts + Recharts). The API serves the built SPA itself in production mode (single local process).
2. **PF1 stays the single financial writer.** The only new aggregation is `crates/sniper-strategy/src/calendar.rs` — a pure day-bucketing of producer-emitted `exit_at_ms` records with an exact reconciliation invariant — living beside the oracle, tested against it. No money arithmetic exists in SQLite, handlers, or JS.
3. **Truth states over fabrication:** runtime status is `OFFLINE/UNKNOWN` until a heartbeat producer exists (DASH-04); NAV history/% returns/TWR are `UNAVAILABLE` until a NAV series producer exists; every response carries `as_of` + `data_status` + provenance; synthetic proof states are watermarked; S00 is mandatory in every comparison.
4. **Milestone sequencing DASH-00→DASH-07** (tooling → contracts → backend → frontend → runtime telemetry → intelligence → hardening), one bounded session each, evidence receipts per package, local commits only.

Rejected alternatives: static HTML generator over receipts (no interactivity for run switching/filters; revisit never needed); Tauri desktop app (extra surface, no offline benefit); dashboard-side RPC/price feeds for "richer" data (explicitly prohibited — dishonest by the repo's own laws); Node backend (second language runtime for zero benefit; Rust keeps the crate graph auditable and cargo-deny-gated).

## Consequences

- Positive: the dashboard cannot corrupt or contradict the research record even if fully compromised locally (it holds no keys, has no write path to producers, and fails closed on any mismatch); determinism is testable (rebuild equality); the corpus's immutability maps directly onto an append-only projection.
- Negative: several honest `UNAVAILABLE` surfaces until DASH-04+ (runtime status, provider health, NAV series, research-run calendars — slot-only records); calendar coverage depends on producer timestamp adoption (wallet-follow family first).
- Follow-ups: DASH-04 producer-side heartbeat + persisted rpc-probe reports; optional PF1 NAV-mark emission; schema `required` fix for `max_drawdown_lamports` (producer-side, DASH-02).
