# SniperFusion Dashboard — Architecture & Delivery Plan

**Document type:** point-in-time engineering design, not a trading authorization.
**Review baseline:** `mAsyamJ/crypto-sniperfusion`, `main@fb1a762d344e78fdaaf8e5a4f19ec046134fd506` (2026-11-05). Re-check HEAD before implementation.
**Scope:** local, read-only trading intelligence and operations dashboard; paper/research/fork/chain-observation truth with explicit evidence labels.
**Authority:** `SOUL.md` > `AGENTS.md` safety rules > `docs/PROJECT_STATUS.md`/immutable receipts for actual state. Do not override the existing root `GOAL_PLAN_PROMPT.md` or `SESSION_LAUNCHER_PROMPT.md`.

## 1. North star & non-goals

**North star:** an operator can answer within 10 seconds: Is the bot running? Is its market data reliable? What is my *reconciled* net PnL? Which candidate and which risk triggered a decision? Which paper hypothesis actually beat S00? What is blocked, stale, or unobserved?

- Outcome: trustworthy, investigable operations, not artificially high trading statistics.
- Non-goals: funded trading, key import/signing, transaction broadcast, strategy promotion, paid services, automatic deployment, public web hosting, a second accounting engine, new trading alpha claims, replacement of the root mission files.
- Absence of evidence: return `UNAVAILABLE`, `BLOCKED`, `STALE`, `INSUFFICIENT_SAMPLE`, or `RECONCILING`; never silently fill with zero or dummy rows.
- PnL must be segregated by chain/cluster, mode, portfolio, run, strategy, evidence/proof state, base denomination, and time window. In particular, independent synthetic strategy arms are not additive.

## 1A. Owner requirement — one authoritative system, zero competing dashboard truth

**Hard invariant: THE DASHBOARD IS A READ-ONLY MIRROR OF SNIPERFUSION, NOT A SECOND DATA-COLLECTION OR TRADING SYSTEM.**

1. **Allowed inputs:** versioned events/health snapshots/decisions/portfolio snapshots **published by SniperFusion's existing Rust runtime and harness**, plus immutable receipts and journals **written by SniperFusion**. Historical views may index the verified, previously written program artifacts. The dashboard cannot collect a new market fact itself.
2. **Forbidden dashboard-side data sources:** direct Solana RPC/WSS/Yellowstone, external price/FX/market data feeds, explorer APIs, DEX Screener, independent wallet scans, alternate strategy engines, alternate account/fee calculations, and OS/process probes not emitted as approved SniperFusion telemetry. If extra information is needed, extend the **canonical SniperFusion producer** in its own milestone to publish that fact; only then may the dashboard show it.
3. **Financial single writer:** `sniper-strategy`'s independent PF1 oracle (or SniperFusion service using that same library) produces authoritative reconciled NAV, position, fee, realized/unrealized PnL and *time-bucketed* calendar outputs. The dashboard must never implement a parallel PnL calculator or independently price portfolio inventory. Formatting/visual grouping is permitted; economic reinterpretation is not.
4. **Runtime single writer:** the running bot publishes its exact mode, cluster/genesis, connected stream status, slot/coverage, process heartbeat, risk gating, open orders and decision reasons. No published heartbeat => `OFFLINE`/`UNKNOWN` plus the timestamp of last received state, never a guessed `RUNNING` badge. A historical receipt never proves a running daemon.
5. **Research single writer:** H8 ledger, evidence receipts, Harness² run outputs and source registries are authority for their own research fields. A static Markdown status statement can be displayed as a cited historical document, never promoted to live state over a newer machine receipt.
6. **Projection is disposable:** SQLite is only an index/cache of immutable canonical outputs and versioned bot telemetry. No dashboard row is the primary source for any market, balance, execution or strategy state. Every projection stores source id/hash, run/strategy/cluster/mode, source sequence, source `as_of`, last indexed sequence, and data age.
7. **Fail closed on divergence:** hash/schema/cursor failure, incomplete source, aggregation mismatch, missing economic marks or stale snapshot => `SOURCE_MISMATCH`, `UNAVAILABLE` or `STALE`; block the affected metric and retain a drilldown to the conflicting source. Do not "repair" it by reading a different provider or performing its own calculation.
8. **Comparison gate:** integration tests assert byte/amount equality for raw values and exact Rust-oracle equality for NAV/PnL/time-bucket output; disallow dashboard dependencies and outbound clients for chain/market APIs, and test rebuild-from-receipts, restart, dropped SSE, mode isolation and clock/timezone grouping. A dashboard success state requires source-sequence synchronization.

**Permitted display-only transformations:** timestamp formatting in the selected timezone; unit formatting from exact integer/decimal-string values; sorting, filtering and chart rendering without changing amounts or evidence class. If a new financial/calendar aggregation is required, implement it inside SniperFusion's authoritative Rust accounting/export code and publish the result, rather than calculating it in the browser or dashboard service.

## 2. Verified starting point and gaps

| Existing capability | Primary source | Gap |
|---|---|---|
| PF1 independent double-entry FIFO oracle, fee lines, deterministic PnL V2 receipts | `crates/sniper-strategy/src/{oracle,receipt,eval,engine/protocol}.rs` | No complete timestamped, reconciled NAV calendar projection |
| H2 event-sourced PaperBroker, real virtual fills, ledger recovery | `crates/sniper-runtime/src/broker.rs` | No continuous shared local server/report stream; paper CLI is fixture-based |
| Chain event, coverage and provider supervision | `crates/sniper-core/src/{event,coverage,observation}.rs`, `crates/sniper-runtime/src/ingest/` | Continuous process health and operator telemetry not exposed through API |
| Read-only RPC probe with genesis check | `crates/sniper-runtime/src/rpc_probe.rs` | On-demand probe, not a streaming health monitor |
| H8 registry, one-use holdout, Harness2 deterministic channel | `harness/research_registry.py`, `integrations/harness2/`, evidence | Need safe projections that never expose holdout labels or secrets |
| Wallet Alpha normalized trade observations, follower experiments | `crates/sniper-runtime/src/wallet_observer.rs`, related strategy/harness sources | Honest wallet profile, basis completeness, follower-copyability views |
| G0 verified 3,001 block window, zero observed creates | `docs/evidence/G0-WINDOW-COMPLETE-20261105.md` | Do not infer zero launches globally; current root mission prioritizes decoder/ALT audit |
| S00 champion; EDGE_NOT_DEMONSTRATED | `docs/STRATEGY_REGISTRY.md`, `SOUL.md` | Strategy dashboard should show limits of evidence, not turn trials into live PnL |

The newest root `GOAL_PLAN_PROMPT.md` includes a **D0 create-family / ALT attribution audit**. The dashboard project must not claim that the zero-create finding conclusively proves a venue shift; downstream modules should label it as a single-window observation subject to audit.

## 3. Information architecture: 12 destinations

1. **Command Center:** real process status, mode + genesis, source freshness, risk gates, selected run, account/NAV summary, top anomalies, active research jobs.
2. **PnL & Analytics:** reconciled realized/unrealized/net PnL, NAV, performance %, cost waterfall, ROI/TWR, realized vs mark-to-exit, drawdown, sample warnings, operating costs in a separate accounting layer.
3. **PnL Calendar:** month/week/day, local timezone toggles (default Asia/Jakarta), daily cash-flow-adjusted PnL and return, day drilldown to receipts/positions/trades; no date assignment without timestamps.
4. **Portfolio:** cash, lots, cost basis, reserves, claimable/rent, liabilities, liquidation value, missing marks, stale/unexitable holdings, strategy/venue concentration.
5. **Execution:** lifecycle timeline from INTENT to FINALIZED/FAILED/UNKNOWN, attempts vs fills, signed unknown state, retry reasons, slippage, network/priority/tip burn; paper vs chain confirmations unmistakably distinct.
6. **On-chain & Infrastructure:** genesis, slot/epoch/commitment, RPC latency/errors, WSS/gRPC reconnect, dropped speculative events, critical gaps, backfill, journal integrity, CPU/memory and disk/indexer lag.
7. **Opportunity Radar:** candidate token event stream; eligibility WATCH/BLOCK, liquidity/reserves, verified exit quote, age, creator/flow metrics, fresh data and source quality; score components without false precision.
8. **Token Detail:** lifecycle curve/migration, as-of reserves, creator/developer behavior, holders and concentration, liquidity risk, evidence-backed feature history, timeline and deterministic decline reasons.
9. **Wallet Alpha:** candidate/source roster, owner-graph and basis completeness, on-chain wallet evidence, historical leader PnL vs executable follower PnL and delay arms, provenance/consensus labels.
10. **Strategy Lab:** S00 and challengers, same-cohort comparisons, trial ledger, G-A–G-G, CI/CVaR/stress, rejected-register, not-yet-implemented flags; holdout only metadata.
11. **Harness Operations:** H8 run jobs, stage progress, harness2 environment status, critique artifacts, immutable receipts, pin/SHA and safe source links, recent failures and blocker reasons.
12. **Alerts / Settings:** read-only failure diagnostics, watch-only wallet selection, timezone, table density, retention, log redaction policy; no signing, secret reveal, live-enable, arbitrary subprocess or privileged controls.

**MVP scope:** 1–6, 10–11 historical-only, 12 alert read-only. Defer 7–9 live enrichment until data contracts and source coverage are established. Do not add a ‘fake-live’ ticker.

## 4. Architecture / isolation boundaries

```text
Solana providers --> EXISTING SniperFusion ingest ONLY
                            |
         EXISTING SniperFusion authoritative Rust runtime
         (PF1 oracle, strategy/decision engine, runtime health)
                            |
                 published versioned telemetry
                            |
        SniperFusion immutable journals / harness receipts
                            |
                            v
               read-only source verifier / importer
                            |
                            v
                  SQLite rebuildable cache
                (no new economic computations)
                            |
                            v
                read-only Axum HTTP + SSE API
                  loopback / strict origin policy
                            |
                            v
                 Vite + React + TypeScript UI
               Radix/shadcn + charts + tables

Dashboard API/UI: NO direct Solana/DEX/wallet/market provider queries.
```

- Prefer separate Rust `sniper-observability` for typed telemetry and `sniper-dashboard-api` for projection/API, subject to coupling and dependency review.
- Trading runtime is source-of-truth for event identity and order status; PF1 oracle is source-of-truth for accounting; immutable H8 registry/receipts are source-of-truth for trial states. SQLite is *rebuildable projection* only. The dashboard importer has NO provider-network clients or alternate price/accounting implementation.
- The observation adapters may ingest approved artifact roots using canonical paths and allowlists. Read-only opened files; no unrestricted path supplied by browser; no symlinks outside allowlist.
- Use a bounded asynchronous channel for events, nonblocking on trading path; account for dropped low-priority telemetry. A missed material event requires catchup/reindex from durable receipt or fail-closed state, never silently assumed consistent.
- Use server-side pagination and indexes over **canonical, already-computed program outputs**; only display-preserving operations (sorting/filtering) may occur in the dashboard. New derived economic data must be emitted by the owning program module. TanStack Virtual is only for genuinely long tables; slow pages must not affect ingestion.
- SSE is one-way, with event id, heartbeat, sequence, reconnect and gap detection; REST snapshot refresh after discontinuity. Polling fallback acceptable.
- Do not bind to `0.0.0.0`; no public server. Protect against DNS rebinding, Host/Origin misuse and cross-site fetch; local privileged controls are out of scope.

## 5. Typed evidence contracts and accounting

Every projected source fact includes:

`schema_version`, `record_id`, `run_id`, `session_id`, `strategy_id` (if applicable), `chain`, `cluster`, `genesis_hash`, `mode` (`REPLAY|PAPER|FORK|SHADOW|MAINNET_READ_ONLY|LIVE_CONFIRMED` only when truly applicable), `proof_state` (`SYNTHETIC|FORK_MECHANICS|SHADOW_ESTIMATE|LIVE_CONFIRMED`), `coverage_state`, `observed_at_utc`, `source_as_of_slot`, `materialized_at_utc`, `source_sha256`, `source_path_allowlisted`, `git_sha`, `values_are_model_derived`, `staleness`, `integrity_status`.

Do not conflate *market observation evidence class* with *P&L execution proof_state*. A mainnet read-only token observation is not a LIVE_CONFIRMED fill, and a shadow estimate using real observations is still modeled.

**Money:** Rust integer lamports and exact decimals. API outputs signed monetary amounts as **decimal strings**, with `currency`, `decimals`, and provenance; avoid JS floating-point accumulation of `u64/i128`, use BigInt/decimal formatting. Never perform portfolio accounting in JS.

**NAV:** `cash + executable_exit_value(open_lots) + recoverable_rent + proven_claimable - liabilities`, with UNKNOWN holds excluded and flagged. Don't double-charge fees already embedded in quote net proceeds. Enforce `end NAV − start NAV − external deposits + withdrawals` reconciliation to the oracle for same-denomination PnL.

**Calendar:** separately report settled realized PnL on settlement day, period marked change, net trading PnL, return and completeness. UTC events stored; timezone grouping is presentation logic with precise range boundaries. Cash flows within return periods require chain-linked subperiod TWR (not naive PnL / ending NAV). Missing timestamps/exit marks produce `UNAVAILABLE`, never zero. Inactive day with verified continuous coverage and zero trades may be 0, with explicit conditions.

**Strategy comparison:** matching dataset, capital, fees and risk settings vs S00; no sum across mutually exclusive hypothetical runs/delays; metrics with insufficient N show `INSUFFICIENT_SAMPLE`; no unapproved holdout unseal.

**Cost model:** separate chain protocol/creator/LP fees, network fees, priority/tips, slip, failed attempts, refundable rent, and *measured* provider operating costs. USD conversion is separately sourced with timestamps and uncertainty; SOL/USD beta is not credited as alpha.

## 6. Data tables and API

**SQLite projection tables (suggested):**
`source_receipts`, `runs`, `runtime_heartbeats`, `chain_probes`, `coverage_windows`, `canonical_events`, `market_snapshots`, `opportunity_decisions`, `wallet_observations`, `wallet_profiles`, `execution_intents`, `execution_outcomes`, `positions`, `portfolio_valuations`, `fee_lines`, `strategy_trials`, `risk_events`, `alerts`, `projection_cursors`.

Key constraints: immutable upstream SHA identity, unique event identity per genesis/slot/transaction/instruction/event ordinal, separate observation identities, deterministic idempotent upserts, source-hash verification, foreign keys, schema-version migration tests, indexes on `(run_id, strategy_id, time)`.

**Read-only API examples:**

```text
GET /api/v1/meta
GET /api/v1/status
GET /api/v1/chains
GET /api/v1/health
GET /api/v1/portfolio?run_id=...
GET /api/v1/positions?run_id=...
GET /api/v1/pnl/summary?run_id=...&strategy_id=...
GET /api/v1/pnl/timeseries?run_id=...&granularity=...
GET /api/v1/pnl/calendar?run_id=...&tz=Asia/Jakarta&month=2026-11
GET /api/v1/trades?run_id=...&cursor=...
GET /api/v1/execution/orders?run_id=...
GET /api/v1/coverage/windows?cluster=mainnet-beta
GET /api/v1/opportunities?mode=...&cursor=...
GET /api/v1/tokens/:mint?cluster=...
GET /api/v1/wallets?cursor=...
GET /api/v1/strategies
GET /api/v1/research/runs
GET /api/v1/research/gates
GET /api/v1/alerts
GET /api/v1/events   # SSE, authenticated/validated local read only
```

Every response carries an `as_of` and `data_status`, and every numeric result a source/proof/scope. API accepts **no** arbitrary filesystem path, shell command, RPC URL, signer command, kill-switch mutation, or trading order.

## 7. Design system / AntiSlop / UX Pro Max

**Design direction (`DESIGN.md`, required BEFORE UI implementation):**
- A *research terminal*, not a casino UI or generic AI dashboard: information-dense but quietly readable, neutral surface, deliberate single accent, restrained status colors only for risk/verified gains/losses.
- Desktop-first dual-pane command center, responsive tablets and narrow mobile; keyboard-first search, table filtering and time-range controls. Financial figures set in tabular numerals; small clear source labels.
- Consistent `UNAVAILABLE / STALE / BLOCKED / RECONCILING` presentation and meaningful recovery actions. Real data only; demo fixtures have permanent visual synthetic watermark.
- Persist a token system (typography, spacing, density, color semantics, radii, chart strokes, focus, overlays) and responsive layout rationale.
- Use Radix for dialogs, tooltips, menus, tabs, select, keyboard navigation. shadcn/ui copies are local audited components; style with the actual `DESIGN.md` tokens, not default blue-purple/gradient/bento aesthetics.
- Apply UI/UX Pro Max's research + generated recommendations, then AntiSlop 'during' mode and final Delivery Gate; resolve conflicts by evidence, usability, accessibility, project identity and legal licenses.
- Screen states per page: initial, loading, empty, error, stale, partial coverage, no-access, synthetic/demo, real observed, mobile narrow, long table, keyboard focus, reduced motion.
- Follow WCAG 2.2 AA design targets, no red/green-only encodings, robust color contrast, focus rings, accessible chart summaries and keyboard reachable actions. Account for SSR not needed in local Vite UI.

**Visual testing:** Playwright screenshot baselines at 1440×900, 1024×768, 390×844, 320×700, dark + light (if both are supported) with deterministic fixtures; axe-core tests; manual keyboard traversal; no invented screenshot content. Charts must gracefully handle zero points and sparse data.

## 8. Development stack and open-source procurement

- React + TypeScript + Vite; Tailwind + Radix Primitives + shadcn/ui; TanStack Query/Table/Virtual; Lightweight Charts for temporal markets; Recharts or visx for summary analytics (pick one after bundle/UX review); Lucide icons; Zod for browser boundary validation.
- Rust + Tokio/Axum + serde for local API; rusqlite/sqlx with SQLite WAL, single writer and migration discipline after resource/maintenance review; tracing; Prometheus/OpenTelemetry only where metrics are bounded and operationally needed.
- Vitest + React Testing Library; Playwright E2E + axe accessibility; Rust unit/integration + property/fault tests. Adopt libraries on merit, not to maximize dependency count.
- All new third-party sources must be official/verified, reviewed for license/security/tool execution, version pinned and documented; don't copy code from unlicensed upstreams.
- No external analytics or network calls by the frontend except approved loopback API and expressly authorized external docs links. No telemetry phoning home.

## 9. Milestone contract / acceptance gates

### DASH-00 — Preflight and agent skill provisioning
- Audit repo HEAD, current root mission, target agent (`Codex`, Hermes, etc), Node (skills CLI requires modern Node), Rust toolchain, changed files, available capacity.
- **FIRST INSTALL:** project-local `antislop` + `antislop-ui`, `antislop-human`, `antislop-layoutmobile`, `antislop-code`, optionally copywriting. Inspect official upstream and package/license before running installers; preserve current project instructions.
- Install `ui-ux-pro-max` and Superpowers using the **correct harness-specific official path**, separately from the real Google Harness2 integration. Verify skill resolution in a fresh agent session; record pin/install instructions without secrets.
- Result: `docs/dashboard/TOOLING_AUDIT.md`, license/pin review, tested version listing; no code until AntiSlop installed and loaded.

### DASH-01 — Truth/audit spec & tests
- Map existing source schemas and commands, define exact response scopes, derive NAV and calendar contracts, classify data provenance/freshness.
- Write `docs/dashboard/REQUIREMENTS.md`, `docs/dashboard/DESIGN.md` or root `DESIGN.md` as appropriate without conflict, `docs/dashboard/ARCHITECTURE.md`, next ADR # after HEAD audit.
- Define failing tests: deposits, unknown/unexitable, partial exits, stale snapshot, missing timestamp, duplicate/incomplete receipt, cross-day position, same-run vs cross-run separation, 64-bit money.
- Gate: every requirement has authoritative upstream source, test plan and state behavior; no new accounting truth engine.

### DASH-02 — Read-only projection and API vertical slice
- Implement artifact and broker adapters; receipt SHA validation; idempotent SQLite projection; migrations; versioned API; local auth/origin controls; offline replay data first.
- Gate: fixed fixture reproduces same oracle totals, reimport is idempotent, corrupted receipt fails closed; no new signer/broadcaster in dependency graph.

### DASH-03 — Core UI / validated PnL calendar
- React app, design tokens/AntiSlop gate, overview, evidence filters, calendar with timezone, NAV and fee waterfall, positions and orders, source drilldown, empty/loading/error states.
- Gate: realistic offline fixtures only (visibly labeled), screenshot & a11y tests, browser doesn't calculate money and no invented metrics.

### DASH-04 — Runtime + on-chain health
- Read-only provider status + heartbeats and bounded realtime streaming; historical coverage and latency graph; clear separation of last run vs actively connected runtime.
- Gate: lost provider or stream => STALE/BLOCKED, critical gaps => status fail-closed; UI backpressure cannot affect hot path; negative/rollback tests.

### DASH-05 — Opportunity, token, wallet intelligence
- Only verified source-backed features and as-of snapshots; candidate decisions and reason codes, wallet basis and follower-copyability, data age, missing-state abstention.
- Gate: simulated/fork/shadow not claimed live; token security and model scores never auto-authorize trades; performance on large candidate tables.

### DASH-06 — Strategy/H8/Harness2 control tower
- Map trial registry and immutable receipts read-only, state of S00, comparison/statistical gates, one-use holdout metadata (NO labels), Harness2 deterministic pipeline status; bounded agent status.
- Gate: baseline included, candidate statuses accurate, no holdout leak, no AI signer/kill-switch exposure.

### DASH-07 — Reliability and documentation finish
- Security negative tests, stream disconnect, db rebuild/restart, snapshot/access control fault injection, dependency/license scan, keyboard/mobile/contrast/screenshot audits; one command local launch; update `docs/DOC_INDEX.md`/`PROJECT_STATUS.md` with actual evidence.
- Gate: clean CI + repeated runs, no audit deviations, `LIVE_TRADING_ENABLED=false`, exact final handoff and honest blocked list.

**Execution pacing:** one independently testable vertical slice per bounded agent session; state advances only with exact passing tests and write-once evidence. Local commits on a dedicated feature branch, no push without owner authorization.

## 10. Source references for planning

- `https://github.com/mAsyamJ/crypto-sniperfusion`
- `https://github.com/miqdadbadjuber/anti-slop` (core, UI, human, layout/mobile, code, optional copywriting; use `DESIGN.md`)
- `https://github.com/nextlevelbuilder/ui-ux-pro-max-skill`
- `https://github.com/obra/superpowers` (Codex/Hermes installation differs)
- `https://www.radix-ui.com/primitives/docs/overview/introduction`
- `https://ui.shadcn.com/docs/installation`
- `https://tanstack.com/table/latest/docs/framework/react/guide/virtualization`
- `https://tradingview.github.io/lightweight-charts/docs` (observe attribution terms)
- `https://playwright.dev/`

All design recommendations are prospective until implemented and tested on the developer's actual local machine.