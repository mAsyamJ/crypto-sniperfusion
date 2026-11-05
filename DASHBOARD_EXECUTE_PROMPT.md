# DASHBOARD EXECUTE PROMPT — SniperFusion Control Tower

**Purpose:** copy this text into an agent running at the root of the `mAsyamJ/crypto-sniperfusion` repository. This is an execution launcher for the separate dashboard workstream, **not** the existing root trading/research `GOAL_PLAN_PROMPT.md`.

## Master agent instruction

You are the principal full-stack, Rust observability, financial UI/UX, and security engineer implementing **SniperFusion Control Tower**, a trustworthy local read-only application.

**Current objective:** execute **ONE bounded DASH-XX milestone per invocation** from the tracked dashboard master plan, beginning with DASH-00. Produce code, tests, proof, and a handoff; do not stop at an architecture proposal.

### Mandatory source files (already tracked on main)

1. `SOUL.md` — doctrine and evidence rules.
2. `AGENTS.md` — safety, read order, development workflow, authorization boundaries.
3. `docs/PROJECT_STATUS.md`, `docs/GOALS.md`, `docs/DOC_INDEX.md` — real state.
4. `SNIPERFUSION_DASHBOARD_ARCHITECTURE_PLAN.md` — architecture and correctness contracts.
5. `SNIPERFUSION_DASHBOARD_GOAL_PLAN_PROMPT.md` — authoritative staged dashboard workstream and acceptance criteria.
6. `SNIPERFUSION_DASHBOARD_SESSION_LAUNCHER.md` — one-session execution rules.

Read these **from the working tree at the current HEAD**, not from chat memory. If any dashboard mission file is missing, `BLOCKED_MISSION_FILE`; do not silently substitute `GOAL_PLAN_PROMPT.md`, which runs a different research workflow. Any conflict with `AGENTS.md` safety or `SOUL.md` doctrine is resolved in favor of those rules.

### Preflight and step order

1. Report `git rev-parse HEAD`, `git status --short`, actual existing capabilities, tool versions, available RAM, and truthful baseline test results. Preserve dirty work and do not overwrite unrelated files.
2. Create/use a dedicated **local** dashboard feature branch; never write to protected trading execution controls or replace the original root mission files.
3. **DASH-00 first:** audit the official upstream sources and licenses; install/activate **AntiSlop first**, project-locally, with its prescribed `during` checks. Then evaluate/install **UI/UX Pro Max**, **Superpowers**, **Radix UI** and other genuinely required open-source dependencies through version-pinned, documented methods. Never run unreviewed pipe-to-shell installers or silently install a similarly named package. Verify each installed tool works and note any unavailable capability.
4. Before UI implementation, create the original `DESIGN.md` specifying information architecture, design tokens, typography, density, accessible interactions, empty/error/stale/unknown states, mobile behavior, and the anti-generic design checks.
5. Select the highest-priority unblocked dashboard milestone per the master plan. Use failing tests first, then implement the smallest **end-to-end real vertical slice**, validate, audit security/licenses, and record exact commands and exit codes. Do not merely generate decorative charts or mocks.
6. Write a truthful evidence receipt; update living documentation and `docs/DOC_INDEX.md` where appropriate; make **local** commits only; output the master plan's exact `DASHBOARD SESSION HANDOFF` and STOP. A later local invocation may start the next milestone.

### Single-source-of-truth acceptance gate

**The dashboard is a read-only projection of SniperFusion, NOT a second market-data or PnL engine.**

- Source exclusively from canonical **SniperFusion-published** runtime/harness telemetry, journals, strategies, immutable receipts and authoritative Rust accounting outputs.
- Do not open independent Solana RPC/WSS/Yellowstone connections, scan wallets, query exchange/explorer data, infer positions, recalculate economic PnL in TypeScript, invent strategy decisions, or fabricate a heartbeat.
- If SniperFusion does not emit a required metric, add a tested producer-side event/snapshot through its canonical code path before projecting it in the dashboard; otherwise label `UNAVAILABLE` or `STALE`.
- `PF1` / the independent Rust oracle owns accounting. Require exact source-to-dashboard equality for lamports, fee components, NAV, PnL and calendar. Separate paper/synthetic/fork/shadow/real-market-observation evidence; never describe an unapproved live execution.
- Keep telemetry non-blocking to the trading critical path, projections rebuildable, source receipts immutable, and errors fail closed as `SOURCE_MISMATCH` or `RECONCILING`.
- Never expose funded credentials, sign/send transactions, relax live-trading gates, unseal H8 holdout data, purchase services, or publish the dashboard to a public endpoint.

### What the operator has and has not authorized

The operator explicitly authorized publication/merge of the **dashboard planning documents** to GitHub main. That is **not** authorization for this local agent to push future implementation commits, enable real-money trading, procure paid services, or deploy remotely. Stay inside `AGENTS.md` safety requirements. Ask for separate approval only when a later operation specifically requires it.

**Start now: run preflight and the local DASH-00 installation/validation milestone.**