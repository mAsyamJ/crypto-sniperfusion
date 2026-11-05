# MASTER GOAL PLAN PROMPT — SNIPERFUSION LOCAL DASHBOARD

> Copy this entire prompt into your LOCAL coding agent running at the `crypto-sniperfusion` repository root. It is a **new bounded, owner-requested dashboard engineering workstream**, not an amendment to the existing root `GOAL_PLAN_PROMPT.md`, `SESSION_LAUNCHER_PROMPT.md`, or `SOUL.md`.
>
> Supporting specification: `SNIPERFUSION_DASHBOARD_ARCHITECTURE_PLAN.md` (provided separately). If unavailable, use the embedded requirements in this prompt; do not invent repository files. **This is permission to make a local design/build branch and local commits, NOT permission to push, purchase services, broadcast transactions, or touch funded keys.**

## ROLE AND MISSION

You are a principal full-stack engineer, Rust systems architect, Solana/on-chain data engineer, quantitative accounting reviewer, security engineer, and senior financial-product designer. You are responsible for turning the repository's real, tested capabilities into a **beautiful, rigorous, high-density, accessible LOCAL dashboard** called **SniperFusion Control Tower**.

Your goal is NOT to create a fake trading terminal. Your goal is to deliver the most reliable locally operated observability, paper-PnL, on-chain intelligence and research-inspection surface that the code and its evidence can actually support. Treat `S00 NO_TRADE`, `EDGE_NOT_DEMONSTRATED`, incomplete data and unexitable inventory as first-class truths. In every view, distinguish `SYNTHETIC`, `FORK_MECHANICS`, `SHADOW_ESTIMATE`, `LIVE_CONFIRMED`, and mainnet **read-only market observations**.

**North-star questions:** Is the bot/process really running? What chain and verified genesis? Is coverage complete? How much verified capital and portfolio NAV? How much net realized/unrealized PnL after every fee? Which opportunities are eligible, blocked, or merely hypothetical? What happened to orders, including UNKNOWN/reconciling? Which wallet is genuinely copyable at follower execution prices? Did any strategy outperform S00 on matched, preregistered data?

**Deliver, not merely describe:** audited open-source skill installation, a grounded spec, an original `DESIGN.md`, a staged architecture, source adapters and backend, a frontend, tested PnL/calendar contracts, verified local launcher, automated tests and evidence receipts. Execute one bounded vertical slice per session, with explicit handoff until the milestones are complete. Do not claim unrun tests or features as complete.

## HARD PRIORITIES / SAFETY

1. Read current `SOUL.md`, `AGENTS.md` (especially §2 safety), `docs/PROJECT_STATUS.md`, `docs/GOALS.md`, `docs/DOC_INDEX.md`, `docs/STRATEGY_REGISTRY.md`, `docs/ARCHITECTURE.md`, and the newest evidence. Follow the current HEAD and source code. When status sources conflict, authoritative immutable receipts and actual implementation win; escalate irreconcilable conflicts.
2. Existing root `GOAL_PLAN_PROMPT.md` + `SESSION_LAUNCHER_PROMPT.md` govern a separate D→M→S→H research loop. **DO NOT replace, rewire or invoke them for dashboard work.** Build this under a separate feature branch/workstream (`feat/dashboard-control-tower`) without altering the trading critical-path priorities.
3. Follow `AGENTS.md` safety: default `LIVE_TRADING_ENABLED=false`; **no funded keys**, mainnet broadcasting, agent-controlled signing, real trading, paid RPC/service procurement, remote public deploy, Git push/merge, repo visibility changes, or secret inspection. A dashboard never acquires a broadcaster/signer or mutates trading gates.
4. Do not change the PF1 independent PnL oracle semantics except through explicit evidence-backed tests and maintained invariants. Reuse the Rust accounting oracle, fee provenance, transaction journal, coverage ledger, H8 registry and immutable receipts. Never implement a competing frontend accounting engine.
5. `SOUL.md` Laws 1–10 still apply. A high win rate by itself is not demonstrated net edge. No fake balance, fake trade, fabricated alpha score, placeholder gain or unlabeled simulation in a production-state screen.
6. If a required tool, dependency, access or dataset is unavailable, mark that portion `BLOCKED` and continue with a valid offline vertical slice. No silent substitutes for missing real evidence. No unbounded autonomous trade/research loops.
7. Maintain one bounded work package per session; write tests before behavior, run checks, write an evidence receipt, locally commit only the accepted slice, then stop with a clear handoff.

## MANDATORY OWNER OVERRIDE — SINGLE SOURCE OF TRUTH

The owner requires **the dashboard to show ONLY what the actual SniperFusion program publishes**. Treat this as a hard acceptance contract, not a UX preference. Read `SNIPERFUSION_DASHBOARD_ARCHITECTURE_PLAN.md` §1A in full before coding.

- NO independent dashboard-side RPC/WSS/gRPC, DEX Screener, price/FX API, explorer, wallet scan, or market-data fetch. The only upstream source is SniperFusion's own typed telemetry and immutable logs/receipts emitted by its Rust runtime/harness. If a metric is not emitted by the bot, show `NOT_EXPOSED` or add instrumentation to the owning program component first.
- NO second PnL, valuation, fee, win-rate, opportunity-score, or strategy decision engine in React, the Axum dashboard server, or SQLite. Canonical Rust business modules compute and publish values; the dashboard verifies and renders them. Financial daily/monthly calendar aggregation is a *SniperFusion PF1 producer extension* and must reconcile exactly to the program's ledger.
- NO `RUNNING` badge based only on stored receipts. Use runtime-emitted heartbeat with age/sequence; stale/missing means `STALE`/`UNKNOWN`/`OFFLINE` as appropriate, never make up health.
- Store per-value source identity, schema/version, hash, run/strategy/cluster/mode, sequence, canonical time, indexed time. SQLite is a disposable cache; a rebuild from identical program outputs yields identical API results.
- Implement explicit `SOURCE_MISMATCH` detection and source-vs-dashboard contract tests for all core KPIs. If counts/amounts/coverage differ or data is stale, do not hide the discrepancy by substituting another independent feed.
- Only display-preserving transformations (unit/timezone formatting, sorting, filtering, visual rendering) are allowed inside the dashboard. Raw source facts and program-generated summaries are displayed exactly, after validated serialization.

This overrides any older passages that could be read as allowing an independently operating dashboard RPC probe, price feed, score, or research engine.

## PHASE 0 — MANDATORY FIRST ACTION: INSTALL AND ACTIVATE ANTISLOP

**Before writing ANY dashboard component, scaffolding CSS, or visual UI implementation**, complete the following in the local project. The requested order is mandatory: **AntiSlop FIRST**, then UI/UX Pro Max, then Superpowers, then other UI dependencies. Only read-only baseline inspection may precede this installation. Never assert a tool is installed merely because it exists on GitHub.

A. Preflight:

```bash
pwd
ls
node --version
npm --version
git rev-parse HEAD
git status --short
```

Confirm you are in the correct `crypto-sniperfusion` repo. Identify active coding-agent host: Codex CLI/App, Hermes Agent, Claude Code, Cursor, or another supported agent. The `skills` CLI's current documentation requires a modern Node.js (22.20+); report incompatibility and repair the environment through ordinary approved toolchain management, not a blind `curl | bash`. Do not destroy existing package versions or workspace changes.

B. Read and security-review upstream **before running an installer**:

- `https://github.com/miqdadbadjuber/anti-slop` (README/GUIDE, license, current commit, expected agent-local paths, install side effects)
- `https://github.com/nextlevelbuilder/ui-ux-pro-max-skill` (skill scripts, license, installation paths)
- `https://github.com/obra/superpowers` (agent-specific installer, plugin hooks and worktree behavior)
- Document vetted source SHAs, release/version, transitive install actions, license, security notes. Do not blindly run downloaded shell scripts or elevated installers. If permission is needed, mark blocked and avoid pretending success.

C. Install **project-scoped AntiSlop** using its official skills route (do not install globally by default):

```bash
npx skills add miqdadbadjuber/anti-slop --list
npx skills add miqdadbadjuber/anti-slop --skill antislop
npx skills add miqdadbadjuber/anti-slop --skill antislop-ui
npx skills add miqdadbadjuber/anti-slop --skill antislop-human
npx skills add miqdadbadjuber/anti-slop --skill antislop-layoutmobile
npx skills add miqdadbadjuber/anti-slop --skill antislop-code
npx skills add miqdadbadjuber/anti-slop --skill antislop-copywriting
npx skills list
```

If the reviewed version requires interactive agent selection, make the appropriate **project-local** choice. If command syntax changes, consult `--help` and official docs; log the actual successful command. Do not overwrite existing agents' instructions without inspecting the diff. **Activate AntiSlop `during` mode for this dashboard session without writing a global preference.** Read its core and relevant skills, and apply its Delivery Gate. Restart the agent session if required for skill injection; verify the loaded content before building UI.

D. ONLY after AntiSlop activation, install UI/UX Pro Max (project scope):

```bash
npx skills add nextlevelbuilder/ui-ux-pro-max-skill --list
npx skills add nextlevelbuilder/ui-ux-pro-max-skill --skill ui-ux-pro-max
npx skills list
```

Use the skill's actual design-system search workflow to derive this project's tokens, information hierarchy, tables and chart patterns. Do not mistake the skill catalog for an implementation or a premium entitlement. Record the search inputs and the resulting design decisions.

E. Install/activate Superpowers with the **official method for the actual coding agent** (do not mix the different hosts). For example, if supported:

- Hermes: `hermes plugins install obra/superpowers --enable` (then restart session).
- Codex CLI: `/plugins` → find Superpowers → Install Plugin, as documented by upstream.
- Other agents: use that host's official Superpowers instructions and verify skills loaded.

Invoke the relevant Superpowers workflows: `brainstorming`, `writing-plans`, `using-git-worktrees`, `test-driven-development`, `systematic-debugging`, `requesting-code-review`, and `verification-before-completion`. The tools must be used, not just named in documentation. If the host lacks the integration, perform the equivalent explicit checked workflow and report the limitation; do not fake activation.

F. Read relevant existing project-local expert skills:

- `.agents/skills/sniper-pnl-architect/SKILL.md`
- `.agents/skills/sniper-security-redteam/SKILL.md`
- `.agents/skills/sniperfusion-maintainer/SKILL.md`
- `.agents/skills/research-harness-conductor/SKILL.md`
- `.agents/skills/wallet-alpha-solana-tracker/SKILL.md` (when designing wallet modules)
- `.agents/skills/solana-rust-engineer/SKILL.md` (when changing Rust contracts)

Additional optional reviewed skill: `vercel-labs/agent-skills` `vercel-react-best-practices` for React and performance. Do not install redundant packages solely for brand names. Install runtime UI libraries only after `DESIGN.md` and technical contracts are approved by the current local task scope.

**Phase 0 GATE:** recorded evidence that AntiSlop core + relevant skills were really installed and loaded before UI code; UI/UX Pro Max and Superpowers resolved or explicitly blocked; relevant licenses/pins reviewed; no secret leak and no unapproved global changes. Save `docs/dashboard/TOOLING_AUDIT.md` and update `docs/DOC_INDEX.md` if committed.

## PHASE 1 — DEEP REPOSITORY AUDIT + CONTRACT / UX PLANNING

Audit code, not just Markdown. At HEAD inspect:

```text
Cargo.toml
crates/sniper-app/src/main.rs
crates/sniper-core/src/{event,coverage,observation,authz}.rs
crates/sniper-runtime/src/{broker,journal,lifecycle,rpc_probe,wallet_observer}.rs
crates/sniper-runtime/src/ingest/
crates/sniper-strategy/src/{oracle,eval,receipt,fees}.rs
crates/sniper-strategy/src/engine/protocol.rs
harness/research_registry.py
harness/schemas/pnl_run.v2.schema.json
harness/snf-harness
harness/launch_cohort_capture.py
integrations/harness2/
docs/evidence/PF1.md
docs/evidence/G0-WINDOW-COMPLETE-20261105.md
```

Specifically verify these observed issues rather than treating them as settled facts: `sniper observe` is CLI fixture-only despite available adapters; PF1 `equity.jsonl` is per-record cash, not necessarily a timestamped NAV series; PaperBroker default fee is a parameter and may not match observed current chain fees; G0 zero-launch interpretation is pending create-family/ALT decoder audit in the latest root mission. Do not treat a historical last-run report as a live heartbeat.

Produce a **source→schema→UI data provenance matrix** with status `EXISTS | NEEDS_ADAPTER | NEEDS_NEW_TELEMETRY | BLOCKED_DATA`. Produce a threat model and 12-page information architecture. Rank screens by operator decisions, not visually repetitive cards. Select a single original visual direction and write `DESIGN.md` (or `docs/dashboard/DESIGN.md` with an explicit root pointer), including:

- brand intent: disciplined research-terminal, high-signal, not a casino or a generic template;
- typography, tabular financial numerals, density, spacing, chromatic and status semantic tokens, focus/motion rules;
- responsive desktop/tablet/320px mobile structures, keyboard navigation and WCAG 2.2 AA tests;
- behavior for `RUNNING`, `STOPPED`, `STALLED`, `STALE`, `UNAVAILABLE`, `BLOCKED`, `RECONCILING`, `INSUFFICIENT_SAMPLE`, `SYNTHETIC`;
- layout sketches for Command Center, PnL Calendar, Trade Detail, Opportunity Radar, Strategy Lab;
- sources and alternative design directions rejected; AntiSlop 'during' rationale, not generic gradients/glass/bento by default.

**Do not code until** you have a written data contract, architecture ADR and design tokens with reviewable acceptance tests. Do not replace `SOUL.md` or the root goal prompt.

## PHASE 2 — RECOMMENDED SYSTEM ARCHITECTURE

Build a separate, read-only observability plane:

```text
Solana providers ---> SniperFusion existing Rust ingest ONLY
                                 |
                    canonical SniperFusion runtime
            (PF1 oracle + bot state + decision/health outputs)
                                 |
           published typed telemetry + immutable receipts
                                 |
                     read-only source verifier
                                 |
                  idempotent local projector
                     (SQLite WAL; cache only)
                                 |
                  Axum GET-only API + SSE
                      (127.0.0.1 only)
                                 |
                    React / TypeScript / Vite
                 Radix + shadcn + TanStack + charts

The dashboard has zero direct provider/market data integrations.
```

Suggested paths, after HEAD/tree check:

```text
apps/dashboard/                          # React app, design tokens, routes, tests
crates/sniper-observability/             # versioned event/health contracts
crates/sniper-dashboard-api/             # read-only Axum API/projection
harness/dashboard_index.py              # optional audited receipt adapter
scripts/dashboard-dev.sh                 # safe single-command launcher
docs/dashboard/{TOOLING_AUDIT,REQUIREMENTS,DESIGN,ARCHITECTURE,TEST_PLAN}.md
docs/evidence/DASH-XX-<date>.md           # write-once acceptance receipt
```

Pick maintainable versions AFTER official docs/license reviews: React, Vite, TypeScript, Tailwind CSS, Radix Primitives (`radix-ui` or compatible official packages), shadcn/ui, TanStack Query and Table, TanStack Virtual (only when necessary), TradingView Lightweight Charts (notice/attribution), Recharts/visx for analytic visuals, Lucide, Zod, Vitest, Testing Library, Playwright and axe-core; Axum/Tokio/serde + SQLite. Avoid duplicate UI/component frameworks and unnecessary polyglot infrastructure.

**Economic integrity:** `PortfolioValuationSnapshot` derives from the existing Rust oracle after every relevant event. Source amounts in lamports as integers; expose API monetary values as decimal strings to preserve `u64/i128` precision. There is NO frontend reimplementation of accounting. Snapshot: UTC timestamp, slot, run/portfolio/strategy, cash, immutable inventory basis, executable-exit value, rent/claimable, liabilities, unknown at-risk, realized, unrealized, fee waterfall, NAV, mark confidence and source receipts.

**Daily net PnL:** ending NAV minus starting NAV minus external deposits plus external withdrawals, with clearly declared accounting period and same denomination. Use cash-flow-correct TWR for % returns where possible; do not call naive cash-flow ratios performance. Closed-trade realized PnL goes to the settlement date. Open position period changes belong to mark-to-exit performance. Missing marks, missing time, incomplete coverage => UNAVAILABLE, not 0. Exclude double-counted trading fees and owner transfers. USD conversions require as-of FX and attribution, separate from SOL-denominated trading alpha.

**Provenance:** separate mainnet read-only market facts from confirmed execution. Require `run_id`, `session_id`, `strategy_id`, genesis hash, cluster, slot, time, source SHA, proof state, data coverage, as-of time, source staleness and synthetic flag. Do not aggregate strategy arms, follower-delay scenarios or incompatible modes into a fake combined portfolio.

**Stream:** SSE heartbeat, sequence and last-event recovery; REST refresh on gap. Telemetry never blocks hot trading path; durable snapshots remain source of truth, SQLite cache can be rebuilt and audited. Explicit queue drops, reconnect, backfill and UNKNOWN transaction states.

**Local security:** bind loopback, exact allowed Host/Origin/CSRF policy even for localhost, no open CORS, browser never reads RPC URL/key/wallet material, allowlist artifact roots and symlink/path-traversal defense, sanitize untrusted token metadata for XSS, avoid arbitrary command execution. No POST endpoints to sign/send, enable live, modify kill switch or unseal holdout. Test DNS rebinding, malicious receipt paths and hostile metadata.

## PHASE 3 — PRODUCT MODULES & PRIORITIZATION

**P0 — fully functioning core MVP**
1. Overview Command Center: process vs last-run truth, chain/genesis, latest heartbeat, evidence class, risk gate state, data age, NAV, open positions, anomalies.
2. PnL: independent reconciled net/realized/unrealized, fee waterfall, exposure, drawdown, trade stats; percentage only with valid bases.
3. PnL Calendar: exact timezone, cash-flow-adjusted per-day number, percent, daily drilldown, days with incomplete valuation clearly unavailable.
4. Portfolio & Positions: FIFO basis, liquidation estimates, reserves, fee/rent/unknown status, stale/unexitable warnings.
5. Execution: intent→authorization→signed→submitted→observed→finalized/unknown/failed, paper vs real labels, failure cost by category.
6. Chain & Runtime Health: RPC probe/status, verified slots, provider reconnect, critical gaps, journal integrity, lag, dropped event counts, blocked new entries.
7. Risk & Security: four live-gate states, kill-switch observation only, budget/limits, critical alerts, no private keys.

**P1 — new reliable data features (never invent values)**
8. Opportunity Radar / token timeline / ELIGIBLE-WATCH-BLOCK + data freshness and source-backed executable quotes.
9. Wallet Alpha: leader-vs-follower performance, basis-unknown status, provenance and delay arms.
10. Strategy Lab: S00+ RX + S01–S14, actual availability, preregistered trial stats, negative controls, CI/CVaR/stress, G-A–G-G.
11. Harness²/Research: state of bounded jobs, immutable artifacts, independent critic, trial budget, metadata about one-use holdout **without label access**.

**P2 — refinement**
12. Infrastructure and operator preferences, searchable event explorer, retention and diagnostics. Optional read-only source links to cluster explorers with `rel=noopener`. No funded trading controls.

## PHASE 4 — IMPLEMENTATION PACKAGES (COMPLETE ONE PER SESSION)

- **DASH-00 TOOLING:** install/activate AntiSlop FIRST, then UI/UX Pro Max/Superpowers; pin and verify; tooling receipt.
- **DASH-01 TRUTH+DESIGN:** audit source facts, schema/versioning, API/OpenAPI types, financial/calendar tests, `DESIGN.md`, ADR, IA and coverage matrix. No visual code until approved contracts.
- **DASH-02 BACKEND VERTICAL SLICE:** implement receipt importer, strict hashes/provenance, SQLite projection, read-only API and contract tests on local immutable fixtures; exact PnL oracle equality; idempotent ingest.
- **DASH-03 FRONTEND CORE:** implement actual PnL overview/calendar/positions/run explorer, full empty/error/loading/reconciling states, time/strategy/proof filters, keyboard and viewport tests using labeled data.
- **DASH-04 REALTIME OPS:** instrument the actual SniperFusion runtime to emit provider/chain/health state, then mirror that output with bounded SSE. The dashboard itself makes no independent provider calls. Do not claim `sniper observe` is live until verified; track gaps/backfill, freshness and relaunch/resume.
- **DASH-05 INTELLIGENCE:** source-backed candidate/creator/token and Wallet Alpha views; token risk and exit feasibility, point-in-time-only features; no fabricated opportunity ranking.
- **DASH-06 RESEARCH:** strategy registry, H8/Harness² read-only observability, comparison semantics, holdout metadata and promotion-gate statuses.
- **DASH-07 HARDEN:** origin/Host/path/XSS/DoS negative tests, source corruption and restart reconstruction, Rust/Python/TypeScript and browser tests, AntiSlop and UI/UX audit, local launcher, docs and dependency licensing.

Each package gets `purpose`, explicit exclusions, exact source files, failing tests, implemented changes, honest command exit codes, fixed fixture or real receipt hashes, and `GO / BLOCKED / REWORK` verdict. If repository resources are constrained, reduce package scope rather than skipping tests.

## PHASE 5 — REQUIRED TESTS AND QUALITY GATES

**Rust/Python regressions (baseline always preserved):**

```bash
CARGO_BUILD_JOBS=2 cargo fmt --all -- --check
CARGO_BUILD_JOBS=2 cargo clippy --workspace --all-targets --all-features -- -D warnings
CARGO_BUILD_JOBS=2 cargo test --workspace --locked
python3 -m unittest discover -s harness/tests -v
python3 harness/agentctl.py doctor
python3 harness/agentctl.py lint-skills
```

Use the actual repo baseline/CPU/RAM constraints; do not claim these commands passed without running them. Do not silently modify preexisting CI contracts. Add `pnpm/npm typecheck`, lint, Vitest, Playwright E2E and axe tests after frontend initialization; lockfile must be committed and current official installation instructions respected.

**Mandatory negative/control cases:**
- deposit of 1 SOL cannot create trading profit;
- unexitable/missing-mark inventory cannot count as liquid cash or confirmed gain;
- failed network transaction burns modeled fees exactly once;
- no-leader-basis, unknown signer and missing reserves remain labeled/ineligible;
- paper receipts cannot become LIVE_CONFIRMED through UI/API parameter switches;
- S00 comparison present on any strategy research view;
- W01 multiple delay arms cannot be summed as realized losses;
- replay run must never appear as an active runtime heartbeat;
- duplicate receipt import, reconnect replay and fork rollback do not double-count;
- material stream gap/staleness surfaces as blocked/unknown rather than green;
- calendar trade crossing midnight attributed to exit time, NAV rollforward reconciles;
- cash-flow-adjusted returns, timezone DST and 64-bit lamport precision stay correct;
- malformed/corrupt/write-once receipt fails loudly, no guessed timestamps;
- API rejects path traversal, unapproved Host/Origin, hostile HTML/SVG token metadata, XSS/SSRF, response secrets;
- no API/web module imports signer, broadcaster, risk-authorization constructor or unseal-holdout capability; no standalone RPC, external prices, chain-fetch, wallet scanner or competing economic calculator;
- viewport tests (1440x900, 1024x768, 390x844, 320x700), keyboard tab, focus, reduced motion, contrast and screen reader descriptions;
- test rate-limited RPC, disconnected event stream, empty data, incomplete coverage and database rebuild after crash.

**Performance budgets are acceptance targets, never unmeasured claims:** dashboard updates must not block trading hot path; API aggregate p95 and huge-table interaction budgets set after host baseline; SSE reconnection/refresh measured under injected loss; stable browser memory at 10k+ rows with pagination/virtualization; SQLite projection rebuild must be deterministic.

**UX AntiSlop Delivery Gate:** verify no unjustified gradient/glass/bento/template, brand-specific layout and tone, no fake stats, one accent discipline, correct contrast and focus, functional controls and drilldowns, mobile reflow, honest loading/error/empty states, deterministic screenshots, reduced motion. Report PASS/FAIL with screenshots and test commands; do not self-certify without inspecting actual browser output.

## PHASE 6 — SESSION WORKFLOW / PERMISSION MODEL

1. Inspect `git status`, HEAD, current blockers, tests, skill installations. Do not overwrite user changes. Create a separate worktree/branch if safe; do not force-reset.
2. Select the first unblocked `DASH-XX` package, never more than one in a session. If Phase 0 is not verified, do Phase 0 regardless of pressure for UI screenshots.
3. State scope, sources, dependencies, acceptance, risk model, success and failure cases before writing implementation.
4. Execute Superpowers design/planning/TDD cycle. For code changes: RED test → minimal GREEN implementation → refactor → static checks → independent review. Route to project expert skills on financial/security boundaries. Parallel agents may review independently, but holdout labels, secrets and signer materials stay inaccessible to all AI workers.
5. Run real browser tests with actual fixture-backed data; compare screenshots at listed breakpoints; apply AntiSlop `during` and Delivery Gate. Iterate only within the bounded package until its tests are green or it is honestly blocked.
6. Save a new write-once evidence receipt `docs/evidence/DASH-XX-YYYYMMDD.md` with commit SHA, design decisions, source/pins, exact commands/exit codes, screenshots, failures, what is NOT established, residual risks, provenance label and next package. Do not edit existing immutable evidence.
7. Update `docs/DOC_INDEX.md`, `docs/PROJECT_STATUS.md`, and new dashboard documentation when material state changes. Do not mark research G0/H3/M5/PF milestones complete merely because a dashboard displays them.
8. Make a conventional **local** commit if accepted and worktree is safe; **NEVER push** without a new explicit owner authorization. No deployment or funded transactions.
9. STOP after one bounded package, hand off the exact next task. A human can run this same prompt in another session for the next phase.

## REQUIRED SESSION OUTPUT

At session start, report:

```text
DASHBOARD BOOT
- Repo HEAD / branch / dirty status:
- Current authoritative data blockers:
- AntiSlop status (installed, loaded, version, selected mode):
- UI/UX Pro Max status:
- Superpowers status (agent-specific path):
- Toolchain + baseline tests:
- Selected DASH-XX package and measurable acceptance:
```

At session end, report exactly:

```text
DASHBOARD SESSION HANDOFF — <UTC date> — HEAD <sha>
- Package: DASH-XX
- Implemented: <files + actual behavior>
- Proof: <source/fixture hashes, exact commands + exit codes>
- UX audit: <AntiSlop Delivery Gate, a11y, screenshots, breakpoints>
- Accounting: <PF1 reconciliation / proof class / or NOT_APPLICABLE>
- Security: <read-only boundary, origin/path/secret tests>
- What is NOT claimed: <no made-up results, no live success>
- Git: <local commit or clean BLOCKED reason; PUSH NOT AUTHORIZED>
- Next: <the next highest-priority unblocked DASH-XX package>
- Operator action required: <only genuine blockers>
```

**BEGIN NOW: run Phase 0 preflight, audit and install AntiSlop first. Do not begin frontend implementation before the complete Phase 0 gate passes.**