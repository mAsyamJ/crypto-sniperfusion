# Control Tower Requirements & Provenance Matrix

**Status:** LIVING (DASH-01 baseline; extends per milestone) · **Date:** 2026-11-05 · **Branch:** `feat/dashboard-control-tower`
**Authority order:** `SOUL.md` > `AGENTS.md` §2 safety > `docs/PROJECT_STATUS.md`/receipts > this file. The original dashboard scope is preserved under `docs/history/dashboard/SNIPERFUSION_DASHBOARD_GOAL_PLAN_PROMPT.md` and design invariant §1A of `docs/history/dashboard/SNIPERFUSION_DASHBOARD_ARCHITECTURE_PLAN.md`, but those archived prompts are **not executable entrypoints**. Current dashboard truth and security obligations are governed by this living requirements file, ADR-0013 and the sole root project goal/launcher.

This document records **verified producer truth** (every claim audited at HEAD `de1a8be` with file:line evidence in `docs/evidence/DASH-01-20261105.md` §2) and turns it into module requirements, an API contract, and financial contracts. It is the input to `ARCHITECTURE.md`, `DESIGN.md`, and `TEST_PLAN.md`.

## 0. The one rule

The dashboard is a **read-only mirror of SniperFusion**, never a second data-collection, accounting, or trading system. If SniperFusion has not emitted a fact, the dashboard shows a typed unavailable state — it never computes, estimates, or fetches a replacement. All money is computed by the Rust PF1 oracle side exclusively; the API/DB/UI move integers and strings, never arithmetic on money (except the producer-side calendar module defined in §6.3, which lives in Rust next to the oracle).

## 1. Scope

**MVP modules (DASH-02/03):** M1 Command Center, M2 PnL & Analytics, M3 PnL Calendar, M4 Portfolio & Positions, M5 Execution & Trade History, plus read-only historical views M10 Strategy Lab (registry/verdicts) and M11 Harness Ops (snf-harness + Harness²), and M12 Settings/About (read-only).

**Deferred:** M7 Opportunity Radar, M8 Token Detail, M9 Wallet Alpha live enrichment (post-MVP; wallet-alpha *results* still appear via M2/M5 as runs). **Runtime liveness (heartbeat/SSE):** DASH-04; until then every runtime status renders `OFFLINE/UNKNOWN` (§7).

## 2. Producer truth — provenance matrix

Statuses: `EXISTS` (artifact on disk, shape verified) · `NEEDS_ADAPTER` (data exists; importer/parser work needed) · `NEEDS_NEW_TELEMETRY` (producer change required before the dashboard may show anything) · `BLOCKED_DATA` (blocked upstream, display as such).

| # | Fact / dataset | Producer | Artifact & format (verified) | Status | Consumed by |
|---|---|---|---|---|---|
| P1 | PnL run summary | `sniper research --out-dir` / `wallet-pnl` / `wallet-follow` | `<dir>/run.json` — `PnlRunReceiptV2`, 28 fields, write-once 0444, `receipt_sha256`-sealed (`crates/sniper-strategy/src/receipt.rs:34-65`) | `EXISTS` | M1,M2,M3,M4,M10 |
| P2 | Comparison vs S00 | same | `run.json.comparison[]` + `<dir>/comparison.json` — rows `{strategy, net_pnl_lamports, incremental_net_edge_vs_no_trade, max_drawdown_lamports, open_positions, status}`; statuses `LOSING\|CANDIDATE_FOR_INDEPENDENT_REVIEW\|NO_TRADE_BASELINE\|OBSERVED` | `EXISTS` | M1,M2,M10 |
| P3 | Fee waterfall | same | `run.json.fees_by_kind_lamports` + `fees.json` — keys `protocol, creator, lp, network, priority, tip, rentlocked, rentrecovered, rentwrittenoff, slippage` (u64 lamports) | `EXISTS` | M2 |
| P4 | Trade records (research) | `sniper research` | `<dir>/trades.jsonl` — JSONL `{"strategy","record":ExecutionRecord}`; fills carry `slot: u64` ONLY — **no wall-clock timestamp**; rejected/failed rows carry no slot (`engine/protocol.rs:142-195`) | `EXISTS` (calendar for these runs: `NEEDS_NEW_TELEMETRY`) | M5 |
| P5 | Trade records (wallet-follow) | `sniper wallet-follow` | `<dir>/trades.jsonl` — **pretty JSON array** of `FollowerRun{delay_ms, entries[{signal_id, mint, entry_at_ms, entry_effective_lamports, tokens, exit_at_ms?, exit_net_lamports?, net_pnl_lamports?, refusal_reason?}], …}` (`follower.rs:70-93`) | `NEEDS_ADAPTER` (shape + per-entry ms timestamps enable the calendar) | M3,M5 |
| P6 | Trade records (wallet-pnl) | `sniper wallet-pnl` | `<dir>/trades.jsonl` — observed-amount records; `equity.jsonl` and `audit.json` are empty strings for wallet-* runs | `NEEDS_ADAPTER` | M4,M5 |
| P7 | Equity staircase | `sniper research` | `<dir>/equity.jsonl` — rows `{"strategy","cash_lamports"}`; **no time axis, no NAV marks** (cash only; `main.rs:372-375`); empty for wallet-* | `EXISTS` (as cash staircase; **not a NAV series**) | M2 (labeled "cash staircase, not NAV") |
| P8 | NAV / mark-to-market series | — | **nothing emitted anywhere**; `NavBreakdown` is in-memory only (`oracle.rs:821-845`); persisted NAV exists only as run-level scalars | `NEEDS_NEW_TELEMETRY` — until emitted: NAV history, % returns, TWR = `UNAVAILABLE` | M2,M3 |
| P9 | Oracle audit gate | `sniper research` | `<dir>/audit.json` — `{strategy: {conservation_ok, cash_matches_engine}}`; `{}` for wallet-* runs | `EXISTS` | M2 (integrity chip) |
| P10 | Runtime heartbeat / liveness | — | **no daemon, no interval, no SSE exists** (verified grep across `crates/`) | `NEEDS_NEW_TELEMETRY` (DASH-04) → status `OFFLINE/UNKNOWN` until then | M1,M12 |
| P11 | G0 cohort window | `harness/launch_cohort_capture.py` | `harness/runs/launch-cohort/manifest-*.json` — `coverage_state (COMPLETE\|INCOMPLETE\|CENSORED\|UNKNOWN)`, `window{from_slot,to_slot,…}`, `gaps[]`, `dataset_sha256`, `raw_creates_count`, `unique_mints`, `in_window_signatures`, `failed_transactions_in_blocks`, `nonpromotable:true`, `evidence_class:"MAINNET_READ_ONLY_CAPTURE"`, 0444 | `EXISTS` | M1,M11 |
| P12 | G0 checkpoint (mutable) | same | `ckpt-*.json` + `.lock` — atomic-replace mutable progress | `EXISTS` (freshness signal only; never a green "running" proof) | M1 |
| P13 | Harness run manifests | `harness/snf-harness` | `harness/runs/<stamp>-<cmd>-<pid>.json` — `{command, argv, exit_code, git_sha, inputs[{path,sha256}], outputs{…}, live_trading_enabled:"false"}`; evaluate embeds `{win_rate, trades, net_pnl_lamports, latency{p50,p95,p99}, scope, validation}` honesty labels | `EXISTS` | M5,M11 |
| P14 | H8 registry | `harness/research_registry.py` | hash-chained JSONL `{seq, prev_hash, kind: hypothesis\|trial, payload{hypothesis_id, claim, status, …}, record_sha256}` | `EXISTS` (verify chain on import; holdout labels never read) | M10 |
| P15 | Harness² runs | `harness/harness2ctl.py` | `harness/runs/harness2/<run>/{result.json,summary.json}` — `{done, failed, execution_count, evidence_class:"DETERMINISTIC_RESEARCH_HARNESS", model_backed:false, promotable:false}` | `EXISTS` | M11 |
| P16 | RPC provider health | `sniper rpc-probe` | **stdout-only report; no file artifact** | `NEEDS_NEW_TELEMETRY` (persist probe reports before display; DASH-04 scope) — provider cards `UNAVAILABLE` until then | M1 |
| P17 | Closed paper trades | PaperBroker (H2) | stdout `PaperRunReport.closed_trades[]` — `ClosedTrade` **carries `decision_at_ms, entry_at_ms, exit_at_ms, entry_slot, exit_slot`** (`broker.rs:169-184`); on-disk presence only via evaluate manifests (P13) | `EXISTS` (via P13; direct broker capture `NEEDS_NEW_TELEMETRY`) | M5 |

### 2.1 Known defects & drift (displayed honestly, never hidden)

| ID | Finding | Dashboard consequence |
|---|---|---|
| D-1 | JSON Schema `harness/schemas/pnl_run.v2.schema.json` `required` omits `max_drawdown_lamports` though the Rust struct and every real receipt include it | importer trusts the Rust struct field list, not the schema `required`; schema fix is a DASH-02-adjacent producer fix (own commit, own test) |
| D-2 | `dataset_sha256` is free text in some receipts (`"signals+quotes"`, `"live-tx-receipts"`), not always a hash | validation treats it as provenance label unless it matches `^[0-9a-f]{64}$` |
| D-3 | `trades.jsonl` is a JSON **array** for wallet-follow despite the `.jsonl` name | importer sniffs shape per subcommand family; never assumes JSONL |
| D-4 | `equity.jsonl` empty for wallet-*; `audit.json` `{}` for wallet-* | typed `UNAVAILABLE` per field, not global failure |
| D-5 | PaperBroker default fee is **flat synthetic 100 bps** (`broker.rs:51`); CLI overrides only `slippage_bps` — NOT the observed 125 bps tiered curve | every paper-run view carries the chip: `fee model: SYNTHETIC FLAT 100 bps ≠ observed 125 bps` |
| D-6 | `sniper observe` is fixture-only (`main.rs:803-806`) | observe runs are labeled `REPLAY/FIXTURE`, never "live feed" |
| D-7 | G0 create_v2-aware diagnostic exists but unexecuted; `ckpt-sweep1.json` absent | G0 "0 launches" shown with the standing D0 caveat (legacy-family diagnostic only) |
| D-8 | No wall-clock emission timestamps anywhere (determinism by design); temporal anchors are `slot` (P4), `*_at_ms` (P5/P13/P17), manifest `recorded_at_unix`, registry `registered_at` | freshness = artifact mtime + embedded anchors; `as_of` semantics defined in §5 |

## 3. Non-functional requirements

- **NFR-1 Locality:** API binds `127.0.0.1` only; frontend makes zero external network calls (fonts bundled, no CDN).
- **NFR-2 Determinism:** projection is a disposable cache; deleting the SQLite file and re-importing identical artifacts yields byte-identical API responses (modulo `served_at`).
- **NFR-3 Fail-closed:** corrupt/tampered artifact → `SOURCE_MISMATCH` quarantine of that artifact only; never silently skipped, never a guessed value.
- **NFR-4 Performance targets (measured at DASH-03, not claimed):** API p95 < 50 ms on the on-disk corpus (<200 runs); frontend interactive < 200 ms on 1,000-row virtualized tables.
- **NFR-5 Safety:** no endpoint mutates anything; no endpoint accepts paths, shell, RPC URLs, or order text; GET-only (405 elsewhere). No secrets in any layer.
- **NFR-6 Accessibility:** WCAG 2.2 AA; keyboard-complete; visible focus; `prefers-reduced-motion` honored; no color-only meaning (signed values + glyphs).

## 4. Module requirements (MVP)

Each requirement: source (provenance §2), state behavior (§7), tests (TEST_PLAN.md ids).

### M1 Command Center (overview)
- **FR-1.1** Answer the north-star questions in one screen: Is anything running? (→ `OFFLINE/UNKNOWN` card, P10, last artifact age from P1/P13 mtimes) · Latest verdict vs S00 (P2: latest run comparison incl. `EDGE_NOT_DEMONSTRATED`) · Reconciled net PnL of the selected run (P1) · What is blocked/stale (aggregated from data statuses). Tests: T-CC1..2.
- **FR-1.2** Run switcher: list all imported runs (P1) with `engine`, `proof_state`, `completeness`, artifact time; selecting one scopes M2–M5. Tests: T-RUN1.
- **FR-1.3** Fee-model chip on any paper/synthetic run (D-5). Tests: T-CC3.

### M2 PnL & Analytics (per selected run)
- **FR-2.1** KPI strip from `run.json`: `starting_nav`, `external_deposits`, `external_withdrawals`, `ending_nav`, `realized/unrealized/net_pnl`, `max_drawdown`, `unexitable_exits`, `open_positions`, `unknown_at_risk` — each with provenance + proof class. Tests: T-PNL1 (exact equality vs run.json).
- **FR-2.2** Comparison table (P2): every strategy arm vs **S00 mandatory** (missing S00 row → `SOURCE_MISMATCH` per repo law). Tests: T-S001.
- **FR-2.3** Fee waterfall (P3) with signed labels; chart + table fallback. Tests: T-PNL1.
- **FR-2.4** Cash staircase (P7) labeled `"cash staircase — not NAV"`; NAV/returns/TWR show `UNAVAILABLE (NEEDS_NEW_TELEMETRY)` (P8). Tests: T-NAV1.
- **FR-2.5** Deposits displayed as a separate line with the explicit caption `deposits are not PnL`. Tests: T-DEP1.

### M3 PnL Calendar
- **FR-3.1** Month grid of **daily net realized PnL (lamports)** for the selected run/strategy arm, produced ONLY by the Rust calendar module (§6.3) from producer-emitted timestamps (P5/P17 family). Default display timezone `Asia/Jakarta`; tz is display-only. Tests: T-CAL1..5.
- **FR-3.2** Cells with no timestamped data render `UNAVAILABLE` (hatched), never 0. Tests: T-CAL3.
- **FR-3.3** Reconciliation footer: Σ day buckets == strategy's canonical net PnL for closed positions (byte-equal integers), else `SOURCE_MISMATCH`. Tests: T-CAL4.
- **FR-3.4** Research runs whose trades carry slots only (P4) → whole-calendar `UNAVAILABLE` with reason `records carry slots, no timestamps (NEEDS_NEW_TELEMETRY)`. Tests: T-CAL5.

### M4 Portfolio & Positions
- **FR-4.1** Open positions per run: count + `open_inventory_cost_basis_lamports` + `recoverable_rent_lamports` + `unknown_at_risk_lamports` (P1); unexitable inventory shown as **inventory at risk, never cash**. Tests: T-POS1.
- **FR-4.2** Cross-run "portfolio" is **explicitly per-run**; runs are never summed silently (independent arms are not additive). Any aggregate view is a labeled table of per-run rows. Tests: T-ISO2.

### M5 Execution & Trade History
- **FR-5.1** Trade table from P4/P5/P6 (shape-aware): fills, refusals, failed landings, exits; slot/timestamp columns per record type; refusals visible (fees and failures are the strategy's shadow). Tests: T-TRD1..2.
- **FR-5.2** Per-trade drilldown: every fee component + `fee_synthetic_fallback` flag + provenance (source file + sha). Tests: T-TRD2.
- **FR-5.3** Run detail header: `engine`, `evidence_class`, `proof_state`, `completeness`, `limitations[]` rendered verbatim, oracle audit chips (P9; `{}` → `UNAVAILABLE`). Tests: T-RUN1.

### M10 Strategy Lab (read-only, historical)
- **FR-10.1** Strategy verdict registry view: per-strategy latest standing from P2 across runs + `verdict` lineage; S00 pinned as champion; rejected registry entries surfaced from `docs/STRATEGY_REGISTRY.md` as **cited historical document** (never over a newer machine receipt). Tests: T-STR1.

### M11 Harness Ops (read-only)
- **FR-11.1** G0 windows (P11/P12) with coverage states + the D-7 caveat; snf-harness manifest timeline (P13); Harness² run summaries (P15). Tests: T-HAR1.

### M12 Settings / About
- **FR-12.1** Read-only: versions (git SHA from manifests), `LIVE_TRADING_ENABLED=false` proof (P1/P13 fields), projection rebuild button (local DELETE+re-import), evidence-class legend. **No** signing, secret reveal, live-enable, or subprocess controls. Tests: T-SEC4.

## 5. API contract v1 (served by `sniper-dashboard-api`, DASH-02)

Envelope for every response:

```json
{
  "as_of": "<RFC3339 of projection build>",       // projection time, not "now"
  "data_status": "OK|UNAVAILABLE|STALE|BLOCKED|RECONCILING|SOURCE_MISMATCH|INSUFFICIENT_SAMPLE",
  "data_status_reason": "string | null",
  "provenance": { "source_path": "…", "source_sha256": "…", "schema_version": 2, "run_id": "…", "git_sha": "…" },
  "payload": { }
}
```

Rules: all monetary values are **JSON strings** carrying signed decimal lamports (e.g. `"-250308"` — unit is lamports, stated in `meta`); u64 lamports never pass through JSON numbers where >2^53 is possible (string everywhere for uniformity). Endpoints are GET-only:

| Endpoint | Payload | Source |
|---|---|---|
| `GET /api/v1/meta` | versions, corpus counts, `live_trading_enabled:false` | importer state |
| `GET /api/v1/status` | runtime card: `{state:"OFFLINE", last_artifact_age_secs, note:"no heartbeat producer (DASH-04)"}` | P10/P1 mtimes |
| `GET /api/v1/runs` | run index (id, engine, proof_state, completeness, verdict, artifact_time) | P1 |
| `GET /api/v1/runs/:id` | run detail incl. limitations, audit chips | P1/P9 |
| `GET /api/v1/runs/:id/pnl/summary` | KPI strip + fees + comparison (S00 mandatory) | P1/P2/P3 |
| `GET /api/v1/runs/:id/pnl/calendar?strategy=&tz=Asia/Jakarta` | day buckets `{date, realized_net_pnl_lamports, trades_closed, status}` + reconciliation | §6.3 module |
| `GET /api/v1/runs/:id/positions` | open inventory (at-risk framing) | P1 (+P5 open entries) |
| `GET /api/v1/runs/:id/trades?cursor=` | shape-aware trade records, server-side pagination | P4/P5/P6 |
| `GET /api/v1/coverage/windows` | G0 windows + checkpoints freshness | P11/P12 |
| `GET /api/v1/research/registry` | H8 chain (verified) — **holdout labels excluded server-side** | P14 |
| `GET /api/v1/research/harness2` | run summaries | P15 |
| `GET /api/v1/harness/manifests` | snf-harness timeline | P13 |
| `GET /api/v1/events` | SSE stream — **DASH-04**, returns `501` until the heartbeat producer exists | — |

`:id` is the projection's opaque run id (content-hash derived); never a filesystem path. Unknown ids → `404`; method ≠ GET → `405`; `tz` must be an IANA name from an allowlist (default `Asia/Jakarta`); invalid → `400`.

## 6. Financial contracts

### 6.1 Money
Lamports are `u64`/`i64` integers in Rust and SQLite (INTEGER). API emits signed lamports as decimal **strings**. The browser formats for display only (grouping, SOL conversion at a **fixed 10⁹ lamports/SOL** — a unit formatting fact, not a price feed). No USD anywhere (no FX source exists; USD = `UNAVAILABLE` by construction).

### 6.2 NAV identity (from PF1, producer-owned)
`nav = cash + executable_exit_value(open_lots) + recoverable_rent + claimable − liabilities`; `net_pnl = ending_nav − starting_nav − external_deposits + external_withdrawals`. Unresolved/unknown holds are excluded from NAV and surfaced as `unknown_at_risk`. The dashboard restates these identities **as labels only**; every displayed number is copied from a producer artifact.

### 6.3 Calendar module (the one new aggregation — producer-side Rust)
Lives in `crates/sniper-strategy/src/calendar.rs` (PF1 extension, DASH-02): input = producer-emitted per-position records with `exit_at_ms` (FollowerEntry family) or ClosedTrade family; output = per-day `{date_utc (or tz-shifted at display), realized_net_pnl_lamports, closed_count}`. Rules:
1. **Settlement-day attribution:** a position's realized PnL lands on the day of `exit_at_ms` in the requested fixed offset. *DASH-02 amendment:* the producer function `bucket_daily(strategy, tz_offset_secs, records)` takes a whole-hour fixed offset from an allowlist (`UTC`, `Asia/Jakarta` — no DST zones: they are not deterministic day boundaries); UTC is the canonical stored bucketing, tz variants are computed by the same producer-side function on demand. **Sums are invariant under the offset** (only labels shift) — tested in T-CAL tz tests.
2. Open positions (no `exit_at_ms`) are excluded from realized buckets and reported separately as inventory.
3. Records lacking timestamps → that record cannot be bucketed; if any such record exists for a strategy, the calendar for that strategy carries `partial: true` + `untimestamped_count`; if **all** records lack timestamps → `UNAVAILABLE`.
4. **Reconciliation invariant:** Σ buckets == Σ per-record `net_pnl_lamports` for closed positions (exact integer equality; the test fixtures must also reconcile to the comparison-row total where the producer guarantees it).
5. No interpolation, no zero-filling: a day with no closed positions is `0 (no activity)` ONLY when the run's window covers it and all records are timestamped; otherwise `UNAVAILABLE`.

### 6.4 Prohibited by construction (negative requirements — tested)
No endpoint or UI path may: sum across runs into one PnL number; recompute fees/prices/winning; mark any synthetic thing `LIVE_CONFIRMED` (proof_state copies from receipts only); show a "running" state without a heartbeat event; or accept an external market/RPC URL.

## 7. State catalog (single source for DESIGN.md)

| Product state | Meaning | Trigger |
|---|---|---|
| `OK` | data served from verified artifact | import verified |
| `UNAVAILABLE` | no producer emission exists | P8/P10/P16/D-4 |
| `STALE` | artifact older than corpus-defined freshness window | mtime/anchors |
| `BLOCKED` | upstream blocked (e.g. G0 window re-eval) | D-7 |
| `RECONCILING` | projection rebuilding | rebuild in progress |
| `SOURCE_MISMATCH` | hash/consistency check failed | §5 rules, T-CAL4 |
| `INSUFFICIENT_SAMPLE` | sample below thresholds (e.g. <3 entries) | strategy stats |
| `SYNTHETIC` (watermark) | proof_state synthetic | permanent overlay on synthetic views |
| `OFFLINE/UNKNOWN` | no runtime heartbeat exists (default truth today) | P10 |

## 8. Acceptance mapping

Every FR maps to TEST_PLAN.md task ids (T-CC*, T-PNL*, T-CAL*, T-POS*, T-TRD*, T-ISO*, T-SEC*, T-DET*). DASH-02 implements backend tasks red-first; DASH-03 binds the UI and browser tests. A requirement without a test is a defect of this document.
