# Control Tower Test Plan (TEST_PLAN.md)

> **For agentic workers:** this is the red-first execution plan for DASH-02 (backend) and the binding test definitions for DASH-03 (frontend/browser). DASH-02 implements Tasks 1–8 task-by-task, each starting from the failing test written first. Steps use checkbox syntax for tracking. Structured with the `writing-plans` skill (repo-mandated location overrides the skill default).

**Goal:** prove, with executable tests, that every number the Control Tower serves is exactly what SniperFusion produced, every absence is a typed state, and every boundary holds.

**Architecture / Tech:** per `docs/dashboard/ARCHITECTURE.md` — `sniper-observability` + `sniper-dashboard-api` (Rust/Axum/SQLite) + `sniper-strategy::calendar`; React frontend at DASH-03.

**Spec:** `docs/dashboard/REQUIREMENTS.md` (provenance §2, contracts §5–7).

## Global Constraints (apply to every task)

- Money is `i64`/`u64` lamports in Rust/SQLite; **decimal strings in JSON**; no arithmetic on money outside `sniper-strategy::calendar` and producer artifacts.
- API: loopback bind only; GET/HEAD only (`405` otherwise); Host allowlist; no CORS headers; `:id` params are content-hash ids, never paths.
- Gates per repo loop: `CARGO_BUILD_JOBS=2 cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --locked` (+`--all-features`); `python3 -m unittest discover -s harness/tests`. New crates: `publish = false` (cargo-deny private-ignore rule).
- Fixtures: real corpus at `harness/runs/wallet-alpha/wa7/` (SYNTHETIC, watermarked in any UI use) + oracle-generated edge fixtures built **in-test via the public PF1 API** (never hand-written PnL numbers).
- No push; one conventional commit per task (or per task cluster as marked).

## Review Focus (failure classes the spec implies; each pinned below)

1. **A receipt that parses but was tampered with** → must fail verification, not serve plausible numbers (T-VER1).
2. **Money > 2^53 or near i64::MAX** → must survive JSON round-trip as strings without float corruption (T-MON1).
3. **Duplicate import / rebuild** → must be idempotent; identical corpus ⇒ byte-identical API dump (T-DET1).
4. **A run whose trades have no timestamps** → calendar `UNAVAILABLE`, never zero-filled (T-CAL5).
5. **Cross-run summation** → must be impossible through any endpoint (T-ISO2) — the classic dashboard lie.

---

## Part A — DASH-02 backend tasks

### Task 1: `sniper-strategy::calendar` (PF1 extension)

**Files:** Create `crates/sniper-strategy/src/calendar.rs`; Modify `crates/sniper-strategy/src/lib.rs` (module export); Test `crates/sniper-strategy/tests/calendar.rs`.

**Interfaces:**
- Produces: `pub struct DayBucket { pub day_utc: String /* YYYY-MM-DD */, pub realized_net_pnl_lamports: i64, pub closed_count: u32 }`; `pub struct CalendarProjection { pub strategy: String, pub buckets: Vec<DayBucket>, pub untimestamped_count: u32, pub open_positions: u32, pub reconciled_total_lamports: i64, pub status: CalendarStatus }`; `pub enum CalendarStatus { Ok, Partial, Unavailable }`; `pub fn bucket_daily(strategy: &str, records: &[CalendarRecord]) -> CalendarProjection`; `pub struct CalendarRecord { pub exit_at_ms: Option<u64>, pub net_pnl_lamports: Option<i64> }` (built by adapters from FollowerEntry/ClosedTrade families).

- [ ] **Step 1 — failing tests** (in `tests/calendar.rs`, ids used everywhere below):
  - `T-CAL1` settlement-day attribution: records exiting 2026-11-04T23:59Z and 2026-11-05T00:01Z land in different UTC buckets; a record entered day-1 and exited day-2 lands ONLY on day-2.
  - `T-CAL2` sign preservation: −83,436 and +50,000 in one day → bucket −33,436 (exact i64).
  - `T-CAL3` missing timestamps: mix of `Some`/`None` exits → `status == Partial`, `untimestamped_count` correct, open (never-exiting) records counted in `open_positions`, excluded from buckets.
  - `T-CAL4` reconciliation: `reconciled_total_lamports == Σ records' net_pnl (closed only)` and `== Σ buckets` (property loop over 100 pseudo-random cases via a fixed-seed LCG — no external rand dep).
  - `T-CAL5` all-untimestamped → `status == Unavailable`, `buckets` empty.
  - `T-CAL6` empty input → `Unavailable` (no activity is not zero).
- [ ] **Step 2** run `cargo test -p sniper-strategy --test calendar` → expect FAIL (module absent).
- [ ] **Step 3** implement `calendar.rs` (pure, no I/O; UTC day from ms via `unix_ms / 86_400_000`; no chrono needed).
- [ ] **Step 4** re-run → PASS. **Step 5** gates + commit `feat(dash): calendar producer module with reconciliation invariant`.

### Task 2: verification + discovery (`sniper-observability`)

**Files:** Create `crates/sniper-observability/{Cargo.toml,src/{lib.rs,discovery.rs,verify.rs}}`; Test `crates/sniper-observability/tests/{verify.rs,discovery.rs}`.

**Interfaces:** `pub fn receipt_sha256_matches(raw: &str) -> Result<PnlRunReceiptV2, VerifyError>` (recompute rule mirrors `receipt.rs` serialization; cross-checked against the real wa7 `run.json` bytes); `pub fn discover(roots: &Roots) -> Vec<ArtifactRef>`; `VerifyError::{SourceMismatch{expected,actual}, Unsupported, Parse(String)}`.

- [ ] **Step 1 — failing tests:** `T-VER1` tamper: take real wa7 `run.json`, flip one lamport digit, verify → `SourceMismatch` (and the untouched file verifies OK); `T-VER2` `dataset_sha256` free-text values pass through as labels (D-2); `T-DISC1` discovery finds only allowlisted roots; a symlinked dir outside roots → refused (`VerifyError::Parse("symlink")` class).
- [ ] **Step 2** run → FAIL (crate absent). **Step 3** implement. **Step 4** PASS (verify prints the real wa7 sha `1ea8bf…`). **Step 5** gates + commit `feat(dash): artifact discovery + hash verification`.

### Task 3: shape-aware parsers + SQLite projection

**Files:** Create `src/parse/{mod.rs,pnl_run.rs,follower_bundle.rs,research_bundle.rs,g0.rs,harness_manifest.rs,registry.rs,harness2.rs}`, `src/project.rs`; Test `tests/projection.rs`.

**Interfaces:** `pub fn import_corpus(roots: &Roots, db: &Connection) -> ImportSummary` (idempotent); schema per ARCHITECTURE §3 (migration `v1` embedded).

- [ ] **Step 1 — failing tests:**
  - `T-IMP1` idempotence: import wa7 twice → row counts unchanged, run id unchanged.
  - `T-SHAPE1` wallet-follow `trades.jsonl` parsed as JSON array (D-3); research bundle as JSONL; wallet-pnl empty equity tolerated (D-4).
  - `T-REG1` H8 chain walk verifies `prev_hash`/`record_sha256` links; a broken link → artifact quarantined `SOURCE_MISMATCH`.
  - `T-HOLD1` (T-SEC5 part): projection never stores holdout label payloads (registry trial `result` field kept, labels dir untouched by design — assert handler output contains no `labels` key).
- [ ] **Step 2** FAIL. **Step 3** implement. **Step 4** PASS. **Step 5** gates + commit `feat(dash): parsers + sqlite projection (idempotent, fail-closed)`.

### Task 4: financial acceptance fixtures via oracle (the mandate cases)

**Files:** Test `crates/sniper-observability/tests/financial_contracts.rs` (fixtures built in-test with `AccountingOracle` public API; also produces the DASH-03 fixture corpus under `harness/fixtures/dashboard/` via a `#[ignore]` writer test).

- [ ] **Step 1 — failing tests (each = oracle fixture → import → assert served values):**
  - `T-DEP1` deposits-are-not-profit: oracle run with 1 SOL external deposit, zero trades → served `net_pnl == 0`, `external_deposits == 1_000_000_000`, displayed separately (assert both fields, assert caption field exists in envelope meta).
  - `T-POS1` unexitable inventory ≠ cash: open lot at basis X with unknown-exit hold Y → `open_inventory_cost_basis == X`, `unknown_at_risk == Y`, `ending_nav` excludes unresolved hold (mirror `oracle.rs` NAV identity).
  - `T-PART1` partial exits: one lot, two partial sells → FIFO basis consumption per `Lot` semantics; per-sell PnL sums to run realized total.
  - `T-MON1` 64-bit money: values `9_007_199_254_740_993` (2^53+1) and `i64::MAX` path through SQLite → JSON as exact strings (no float drift: assert string equality, not approximate).
  - `T-S001` S00 mandatory: comparison without an S00 row → endpoint returns `SOURCE_MISMATCH` envelope for that run.
- [ ] **Step 2** FAIL. **Step 3** implement fixture builders + assertions. **Step 4** PASS. **Step 5** gates + commit `test(dash): financial contract cases (deposits, unexitable, partial, 64-bit, S00)`.

### Task 5: Axum API + envelope

**Files:** Create `crates/sniper-dashboard-api/{Cargo.toml,src/{main.rs,routes.rs,envelope.rs}}`; Test `crates/sniper-dashboard-api/tests/api.rs` (in-process tower client, no socket needed except Task 6).

**Interfaces:** endpoints per REQUIREMENTS §5; envelope types; `run id == sha256(run.json bytes)[..16]`.

- [ ] **Step 1 — failing tests:** `T-RUN1` run list + detail serve wa7 fields exactly (`engine=="wallet_follower_v1"`, `verdict=="EDGE_NOT_DEMONSTRATED"`, 7 comparison rows, S00 present with `NO_TRADE_BASELINE`); `T-PNL1` summary KPIs byte-equal to `run.json` values; `T-NAV1` no NAV-series field ever present; `data_status=="UNAVAILABLE"` with reason for `equity` when empty (D-4); `T-CAL7` calendar endpoint serves Task-1 buckets for wa7 arm `FOLLOW_DELAY_500MS` (Σ == −250,308); `T-ISO1` per-run isolation: two imported runs never merge (assert separate ids/rows); `T-ISO2` no endpoint returns a cross-run sum (enumerate routes, assert no `total`/`aggregate` key across runs); `T-CC1` `/status` returns `{state:"OFFLINE", note:"no heartbeat producer (DASH-04)"}`; `T-EVT1` `/events` → `501`.
- [ ] **Step 2** FAIL. **Step 3** implement. **Step 4** PASS. **Step 5** gates + commit `feat(dash): loopback GET-only API with typed envelopes`.

### Task 6: boundary/security tests (real socket)

**Files:** Test `crates/sniper-dashboard-api/tests/security.rs` (binds ephemeral loopback port).

- [ ] **Step 1 — failing tests:** `T-SEC1` non-GET → `405`; `T-SEC2` Host `evil.example` → `403`; Origin `https://evil.example` → `403`; absent Origin/Referer → OK (same-machine curl); `T-SEC3` traversal ids (`../runs/run.json`, `%2e%2e`) → `404`, never file content; `T-SEC4` `/api/v1/meta` exposes no filesystem paths beyond allowlisted roots and no env values; `T-SEC5` dependency audit: `cargo tree -p sniper-dashboard-api` contains no `sniper-runtime` signer/authz deps (grep test in CI script sense — implement as a `build.rs`-free unit test reading `Cargo.toml` deps list of the crate, asserting the deny-list).
- [ ] **Step 2** FAIL → **Step 3** implement middleware → **Step 4** PASS → **Step 5** gates + commit `test(dash): loopback/origin/path/capability boundary enforcement`.

### Task 7: determinism/rebuild

**Files:** Test `crates/sniper-observability/tests/determinism.rs`.

- [ ] `T-DET1`: full rebuild twice into fresh DBs → identical serialized dumps of every endpoint (excluding `served_at`); `T-DET2`: corrupt one artifact after first import → rebuild yields that artifact quarantined and others unchanged (fail-closed blast radius = 1). Steps as above; commit `test(dash): rebuild determinism + quarantine blast radius`.

### Task 8: DASH-02 close-out

- [ ] Full gates ×2; schema-drift fix D-1 (producer-side `required` += `max_drawdown_lamports` in `harness/schemas/pnl_run.v2.schema.json`, with its own commit `fix(schemas): pin max_drawdown_lamports as required`); evidence receipt `docs/evidence/DASH-02-<date>.md`; indexes updated; local commit only.

## Part B — DASH-03 frontend & browser test definitions (detailed at DASH-03 kickoff; binding now)

- `T-UI1` every module renders against the wa7 corpus with correct numbers (Playwright asserts DOM text equals API values — the browser never recomputes).
- `T-UI2` full state matrix per module: loading/empty/error/stale/unavailable/synthetic-watermark visible at DOM level (fixtures + server states).
- `T-UI3` calendar reconciliation footer visible and equal to Σ cells; `UNAVAILABLE` cells hatched, not zero.
- `T-UI4` keyboard-only walkthrough (rail → tables → drilldown → back) with visible focus; axe-core zero serious/critical; contrast token assertions from DESIGN.md §3 (measured pairs).
- `T-UI5` viewports 1440×900 / 1024×768 / 390×844 / 320×700 screenshots (light+dark) committed under `docs/evidence/dash03-screenshots/`; no horizontal overflow at 320 (R-03).
- `T-UI6` launcher e2e: `scripts/dashboard-dev.sh` → real browser loads `http://127.0.0.1:7433/`, run switcher changes scoped data, filter by proof_state works.
- `T-UI7` AntiSlop Delivery Gate PASS/FAIL report with evidence lines (R-02…R-40), delivered with the milestone receipt.

## Self-review (writing-plans checklist; run at DASH-02 kickoff again)

Spec coverage: every REQUIREMENTS FR maps to ≥1 test id (M1: T-CC1-3/T-RUN1; M2: T-PNL1/T-NAV1/T-DEP1/T-S001; M3: T-CAL1-7; M4: T-POS1/T-ISO1-2; M5: T-SHAPE1/T-TRD(drilldown via T-RUN1+T-PNL1 fields)/T-PART1; M10: T-REG1/T-S001; M11: via projection tests + endpoints; M12: T-SEC4). Type consistency: `CalendarRecord`/`DayBucket` names fixed in Task 1 and reused in Tasks 3/5. Proportion: this plan stays a plan — fixture builders generate numbers from the oracle, never hand-typed.
