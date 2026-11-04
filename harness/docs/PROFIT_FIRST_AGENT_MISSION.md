# SniperFusion — Profit-First P&L and H8 Master Agent Mission

**Root:** `~/project/crypto-bot`. **Repo:** `mAsyamJ/crypto-sniperfusion`.
Read `AGENTS.md`, `README.md`, `docs/PROFIT_FIRST_GOAL_PLAN.md`, `docs/evidence/H6.md`, `docs/evidence/H7.md`, `docs/PROJECT_STATUS.md` and `.agents/skills/sniper-pnl-architect/SKILL.md` before changes.

## Role

Act as SniperFusion principal maintainer, Solana/Rust engineer, trader, independent quantitative accountant and harness architect, with **independent** security and statistical criticism. Roles may be procedural if no multi-agent runner is available; do not invent LLM dispatch evidence.

## Mission

The owner's business objective is to **make net money with a trading bot**. Build the strongest possible evidence-backed system for determining whether, how and why any strategy actually earns *withdrawable* after-cost P&L, and improve strategies through bounded falsifiable experiments. Do not promise profit, maximize only win rate, or declare a synthetic backtest viable.

H8 means propose → measure → independently reconcile net P&L → compare → falsify → retain/reject → only after evidence, paper-promote. Mainnet remains strictly read-only.

## First: audit what is actually implemented

Do not rewrite `sniper-strategy`. Inspect H7's `engine::legacy`, `engine::protocol`, `fees`, `oracle`, `eval` and H2 paper broker; map gaps against `PnlRunReceiptV2`. Inspect current HEAD, branches, untracked files and CI; preserve operator work. Consult current official Pump.fun fee schedules, IDLs and Solana priority-fee documentation without copying old constants.

## Implement in dependency order (not documentation-only)

**PF0/1 — build the actual P&L control plane now.** Define account/valuation schema and real data provenance. Add immutable run receipts, detailed per-trade/strategy performance, gross-to-net fee waterfall, cashflow-normalized start/end NAV, uncertain/unexitable inventory handling, full-portfolio drawdown, paired S00 and delayed-entry comparisons, independent cash/account delta reconciliation and failure audits. If source data is synthetic, label `SYNTHETIC` and make `promotable=false`.

Required tests: deposits are not profits; partial exits conserve cost basis; failed tx charges its fee only once; tips/priority fees have correct units; ATA rent is cash locked not free money; SOL/USD changes are not mistaken for sniper alpha; unexitable inventory is not counted as withdrawable; unknown tx cannot silently become realized; same inputs yield identical receipt hash.

**PF2 — unblock block-verified real creation cohorts** using the existing owner-managed Mainnet RPC (read-only). First run an unsensitive capability probe. Do not rotate, reveal, log, persist or expose credentials to LLM workers. Distinguish rate limits/unsupported versions/slow endpoint; add bounded resumable capture with slot-level proof, creator/day/time as-of evidence and fee/reserve states. If no complete coverage, declare BLOCKED and continue independent P&L work offline.

**PF3 — run real Engine B** on verified as-of state and independently reconstruct ending NAV from raw records; Engine A is a diagnostic surrogate, not a performance estimate. No synthetic fallback may silently price a real trade.

**PF4 — conduct a matched profit tournament** with S00 and delayed-entry reference, strategy-specific trade counts, paired after-cost P&L, worst cases, uncertainty, time/creator split, false-positive cases, no-fill and price-impact stress. Each strategy must specify why it expects a causal edge before seeing test results.

**PF5 — implement H8 research council**, ≤20 hypothesis trials, independent critic, real sealed one-use creator/time holdout (not fake toy split), multiplicity accounting. The council must have zero path to signer, paid APIs or live config.

**PF6 — forward read-only shadow**, at least seven consecutive days, tracking event age/quote freshness/provider availability and robust fill uncertainty. Provide daily P&L estimates and abstention logs. Genuine live profits can only be known after separately operator-authorized, funded, confirmed real trades; this mission does not authorize those.

## Engineering loop

Inspect → design acceptance → failing test → implementation → `cargo fmt` / `clippy` / Rust + Python tests → local SVM/fork for protocol changes → chaos / no-lookahead tests → independent NAV check → paired net-P&L comparison → evidence receipt → update project status → local commit and PR.

Require exact exit codes and receipt hashes. Never count skipped tests as passed, never overwrite holdout after viewing results, never claim profit where evidence supports only synthetic/fork/shadow simulation. Never automatically push or merge without explicit owner permission.

## Immediate deliverable and output

Start coding PF0/PF1 immediately on a feature branch; do not stop after writing a plan. Produce a CLI-readable **Net P&L V2** sample report from the existing synthetic cohort that honestly shows S00 NO_TRADE dominating and its complete gross-to-net fee breakdown, then independent unit/regression tests and a P&L V2 schema. Hand off PF2 separately if full-block RPC unavailable.

Every session report: HEAD, code changes, actual test/CI status, start/end equity, net P&L, max drawdown, costs, uncertainty, no-trade comparison, blockers, evidence class and next measurable financial hypothesis.

**Success = a trustworthy, reproducible machine for measuring and improving net after-cost trading returns—not an unverifiable guarantee of profitability.**
