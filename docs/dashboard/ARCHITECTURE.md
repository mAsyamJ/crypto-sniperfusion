# Control Tower Architecture (implementation design)

**Status:** LIVING (DASH-01 baseline) · **Date:** 2026-11-05 · **Implements:** `docs/dashboard/REQUIREMENTS.md` under the invariant of `SNIPERFUSION_DASHBOARD_ARCHITECTURE_PLAN.md` §1A. Decision record: `docs/ADRS/0013-dashboard-read-only-mirror.md`.

## 1. Pipeline

```
Solana mainnet (read-only)  ──▶  SniperFusion Rust runtime + Python harness   [UNTOUCHED PRODUCERS]
                                        │
                                        ▼  write-once artifacts (0444, hash-sealed)
                          harness/runs/**/run.json · manifests · registries · bundles
                                        │
                        ┌───────────────┴────────────────┐
                        │  crates/sniper-observability   │   importer + verifier + projector
                        │  · allowlisted-root discovery  │   (lib; no network)
                        │  · sha256 verification         │
                        │  · shape-aware parsers (P4/P5/P6)
                        │  · calendar via sniper-strategy::calendar (PF1 extension)
                        │  · SQLite WAL projection       │   disposable cache
                        └───────────────┬────────────────┘
                                        │  (in-process, mpsc, bounded)
                        ┌───────────────┴────────────────┐
                        │  crates/sniper-dashboard-api   │   Axum, 127.0.0.1, GET-only
                        │  · /api/v1/** (REQUIREMENTS §5)│
                        │  · SSE stub → 501 until DASH-04│
                        └───────────────┬────────────────┘
                                        ▼  fetch (loopback only)
                        ┌────────────────────────────────┐
                        │  apps/dashboard  React+TS+Vite │
                        │  Radix + shadcn + TanStack      │
                        │  Lightweight Charts + Recharts │
                        └────────────────────────────────┘
```

Hard boundary: **no component right of the artifacts line may open a network connection except to `127.0.0.1:<api-port>`**, and none may import or link signer/broadcast/holdout code. Enforced by crate dependency graph + tests (T-SEC5).

## 2. Components

### 2.1 `crates/sniper-observability` (new, DASH-02)
- `discovery.rs` — walks **allowlisted roots** from `config/dashboard-roots.toml` (committed: `harness/runs/wallet-alpha/`, `harness/runs/launch-cohort/`, `harness/runs/` top-level manifests, `harness/runs/harness2/`, `harness/runs/rpc-capability/`, `harness/runs/fee-config/`; operator may extend locally, uncommitted). Discovery is read-only; symlinks are refused (no escape).
- `verify.rs` — recomputes each artifact's integrity: for `run.json`, `receipt_sha256` per the producer's canonical serialization rule (re-implemented from `receipt.rs` and cross-checked byte-exact in tests against real receipts); manifests checked against embedded hashes where present (`evaluation_sha256`, `dataset_sha256` when 64-hex per D-2). Failure → `SourceMismatch` record (quarantined, listed, never served as data).
- `parse/` — one parser per artifact family: `pnl_run.rs` (P1), `follower_bundle.rs` (P5 array shape), `research_bundle.rs` (P4 JSONL), `g0.rs`, `harness_manifest.rs`, `registry.rs` (H8 chain walk + verification), `harness2.rs`. Unknown shapes → typed `Unsupported` (fail-closed per file, not per corpus).
- `project.rs` — idempotent upserts into SQLite keyed by content hash; re-import of identical bytes is a no-op; changed bytes at the same path = new run version (append-only), old version retained.
- `rebuild.rs` — `rebuild(db, roots)`: wipes and reconstructs; determinism test compares full API dumps (T-DET1).

### 2.2 `crates/sniper-strategy/src/calendar.rs` (PF1 extension, DASH-02)
Pure function per REQUIREMENTS §6.3: `bucket_daily(records) -> CalendarProjection` with the reconciliation invariant. No I/O; unit-tested against oracle-generated fixtures. This is the **only** new aggregation in the whole system and it lives beside the oracle, not in the server.

### 2.3 `crates/sniper-dashboard-api` (new, DASH-02)
- Axum + tokio, bind `127.0.0.1:7433` (default; env override honored only for loopback addresses).
- Middleware: Host allowlist (`127.0.0.1:7433`, `localhost:7433`) → else `403`; `Origin`/`Referer`, when present, must be same-origin → else `403` (DNS-rebinding defense); method guard (GET/HEAD only → else `405`); no CORS headers ever.
- Handlers map projection rows to the REQUIREMENTS §5 envelope; money → decimal strings; `501` for `/events` until DASH-04.
- Import trigger: on boot + `POST`-free manual rebuild is NOT an endpoint — rebuild is a CLI flag (`--rebuild`) and a local launcher step (NFR-5: no mutating endpoints at all).
- Bin name: `sniper-dashboard-api` under `crates/sniper-app` sibling (own crate with `[[bin]]`).

### 2.4 `apps/dashboard` (DASH-03)
React 18 + TypeScript + Vite; Tailwind tokens from DESIGN.md; Radix primitives via shadcn-style local components (copied, tokenized, not default-styled); TanStack Query (server state) + Table (virtualized); Lightweight Charts (attribution in About) for the staircase, Recharts for waterfall/calendar heatmap. Dev server proxies `/api` to loopback API; production mode = API serves the built static bundle from `apps/dashboard/dist` (single process, one port).

### 2.5 `scripts/dashboard-dev.sh` (DASH-03)
One command: rebuild projection → start API (serves static build if present) → (dev mode) start Vite. Prints the verified URL `http://127.0.0.1:7433/`.

## 3. SQLite projection (schema v1)

Disposable cache (WAL, single writer, `harness/runs/dashboard-cache/projection.sqlite3` — gitignored path under runs). Tables (from REQUIREMENTS §2):

```
projection_meta(key TEXT PK, value TEXT)                -- schema_version, built_at, roots, git_sha
source_artifacts(id TEXT PK, path TEXT, sha256 TEXT, kind TEXT, status TEXT,  -- OK|SOURCE_MISMATCH|UNSUPPORTED
                 first_seen_utc TEXT, bytes INTEGER)
runs(id TEXT PK, artifact_id TEXT REFERENCES source_artifacts, engine TEXT, evidence_class TEXT,
     proof_state TEXT, completeness TEXT, verdict TEXT, promotable INTEGER,
     starting_nav_lamports INTEGER, external_deposits_lamports INTEGER, external_withdrawals_lamports INTEGER,
     ending_nav_lamports INTEGER, realized_pnl_lamports INTEGER, unrealized_pnl_lamports INTEGER,
     net_pnl_lamports INTEGER, max_drawdown_lamports INTEGER, recoverable_rent_lamports INTEGER,
     open_inventory_cost_basis_lamports INTEGER, open_positions INTEGER, unexitable_exits INTEGER,
     unknown_at_risk_lamports INTEGER, dataset_sha256 TEXT, receipt_sha256 TEXT, limitations_json TEXT)
run_comparisons(run_id TEXT, strategy TEXT, net_pnl_lamports INTEGER, incremental_net_edge INTEGER,
     max_drawdown_lamports INTEGER, open_positions INTEGER, status TEXT, PRIMARY KEY(run_id, strategy))
run_fees(run_id TEXT, kind TEXT, lamports INTEGER, PRIMARY KEY(run_id, kind))
run_trades(run_id TEXT, ord INTEGER, strategy TEXT, record_kind TEXT, mint TEXT, slot INTEGER NULL,
     at_ms INTEGER NULL, net_pnl_lamports INTEGER NULL, raw_json TEXT, PRIMARY KEY(run_id, ord))
-- calendar_days: NOT materialized (DASH-02 amendment) — daily buckets are a
-- pure function of run_trades via sniper-strategy::calendar (producer-side
-- Rust), computed at query time; caching a pure deterministic function in
-- SQL would add schema without adding truth.
coverage_windows(id TEXT PK, from_slot INTEGER, to_slot INTEGER, coverage_state TEXT, gaps INTEGER,
     raw_creates_count INTEGER, unique_mints INTEGER, dataset_sha256 TEXT, evidence_class TEXT)
registry_records(seq INTEGER, registry_id TEXT, kind TEXT, payload_json TEXT, record_sha256 TEXT,
     prev_hash TEXT, PRIMARY KEY(registry_id, seq))
harness2_runs(id TEXT PK, done INTEGER, failed INTEGER, execution_count INTEGER, model_backed INTEGER,
     promotable INTEGER, evidence_class TEXT)
harness_manifests(id TEXT PK, command TEXT, exit_code INTEGER, git_sha TEXT, recorded_at_unix INTEGER, summary_json TEXT)
```

Indexes: `runs(verdict)`, `run_trades(run_id, ord)`, `calendar_days(run_id, day_utc)`. Money columns are INTEGER (SQLite 64-bit signed — the 64-bit precision test T-MON1 pins this). No SQL ever performs arithmetic on money columns; SELECTs copy values (verified by test asserting server sums happen only in Rust `calendar.rs`/handlers).

## 4. Determinism & rebuild

- Import order is stable (sorted by (path, content-hash)); ids derive from content hashes; all handler output sorted by declared keys → identical corpus yields identical `payload` bytes (T-DET1 compares serialized dumps).
- `as_of` = projection build time (from `projection_meta.built_at`), NOT request time, so responses are reproducible; `served_at` header may vary and is excluded from the comparison.

## 5. Security model & threat model (dashboard-scoped)

| ID | Adversary / threat | Mitigation | Test |
|---|---|---|---|
| DA-1 | Remote host connecting to the API | loopback-only bind; Host allowlist; same-origin Origin/Referer enforcement; no CORS | T-SEC1, T-SEC2 |
| DA-2 | DNS-rebinding page reaching `127.0.0.1:7433` | Host header check (rebind yields attacker domain ≠ allowlist) | T-SEC2 |
| DA-3 | Path traversal / arbitrary file read via params | `:id` values are projection ids only (content-hash namespace), never paths; static file serving jailed to `apps/dashboard/dist` with canonicalization + prefix check + symlink refusal | T-SEC3 |
| DA-4 | Tampered/corrupt artifacts poisoning views | sha256 verification per §2.1; mismatch → quarantined `SOURCE_MISMATCH` record shown as such | T-VER1 |
| DA-5 | Holdout/label leakage through registry view | `research/registry` handler excludes holdout payloads server-side; never SELECTed into projection | T-SEC5 |
| DA-6 | Capability creep (signer/broadcast imports) | crate graph: observability/api depend only on serde/axum/sqlite/crypto — a dependency test forbids linking `sniper-runtime` authz/signer modules | T-SEC5 |
| DA-7 | Supply chain (new deps) | licenses pre-reviewed (TOOLING_AUDIT §4); cargo-deny + pinned versions; npm lockfile committed; no postinstall scripts beyond documented tooling | CI |

## 6. SSE / heartbeat (design only — DASH-04)

Producer-side: a small `sniper observe`-adjacent emitter (or snf-harness wrapper) writes `harness/runs/heartbeat/beat-<pid>.jsonl` (append, seq, monotonic ts, run context). API tails the newest stream, re-emits as SSE with `id:` = seq; client detects gaps → full refetch; silence > 2× interval → UI flips the card to `STALE`, never keeps claiming `RUNNING`. No heartbeat file exists today → `/events` returns `501` (honest).

## 7. What this architecture forbids (checklist for every future PR)

No dashboard-side RPC/WSS/explorer/price/FX; no second PnL engine (JS or SQL); no mutation endpoints; no writes outside the cache DB; no secrets; no holdout reads; no cross-run arithmetic; no `RUNNING` without heartbeat; no push. Every bullet has a test or CI hook (TEST_PLAN, T-SEC*, T-ISO*, T-MON*).
