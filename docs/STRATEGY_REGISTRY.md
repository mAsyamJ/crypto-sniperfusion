# Strategy Registry — living status of every policy, challenger, and hypothesis

**Version:** 1.0 (2026-11-05) · **Status:** ACTIVE · **Update rule:** status changes only through the H8 registry and the gates (`AGENTS.md` §7) — never from memory or chat.

This file records **state only**: what each ID is, its current status, its receipts, and what blocks it. Doctrine (families, hypotheses, kill criteria, the append-only rejected registry) lives in `SOUL.md` §5 and is never restated here. Execution receipts live in `docs/evidence/`. The hash-chained ledger (`harness/runs/…/h8-registry.jsonl`) is the authoritative record of registrations and trials; if this file and the ledger disagree, the ledger wins and this file gets corrected.

## Status vocabulary

| Status | Meaning |
|---|---|
| CHAMPION | Current default; holds the verdict until a challenger passes G-A…G-G |
| TESTED — REJECTED (synthetic) | Executed on synthetic cohort; S00 dominates; not promotable |
| EXECUTED — REJECTED (real signals) | Ran on real observed signals; net-negative vs S00 |
| REGISTERED — UNEXECUTED | In the H8 ledger; awaiting data or execution window |
| PLANNED | Named successor in a goal plan; not yet registered |
| FRONTIER — UNIMPLEMENTED | Family defined in `SOUL.md` §5; no code, no registration |
| COMPONENT | Cross-cutting module; not an entry policy |

## Baselines

| ID | Policy | Status | Notes |
|---|---|---|---|
| S00 | NO_TRADE | **CHAMPION** | Beats every tested policy on synthetic cohorts and real signals to date (H7, PF1, WA1–WA3, W01). Reference point for `incremental_net_edge`. Demoted only by a challenger passing G-A…G-G |
| RX | RANDOM_ELIGIBLE | Control | Deterministic random-entry control (`FNV(event_id) < 2000/10000`). PF1 synthetic batch: −64,253 lamports vs S00 = 0 |

## Implemented policies S01–S10 (H7)

All executed through Engine A/B + the PF1 independent oracle on the synthetic cohort. Standing verdict: **S00 dominates at observed fees; all rejected on synthetic evidence** (`EDGE_NOT_DEMONSTRATED`). PF1 batch, net P&L vs S00 = 0: RX −64,253 · S01/S02 −192,760 · S03 −1,138,507 (1 open position) · S04–S09 −64,253 · S10 −128,506 lamports.

| ID | Status | Receipts |
|---|---|---|
| S01–S10 | TESTED — REJECTED (synthetic) | `docs/evidence/H7.md`, `docs/evidence/PF1.md` |

Policy semantics live in code (`crates/sniper-strategy`) and the PF1 receipt — they are deliberately not paraphrased here to avoid state drift. Known named: S02 = delayed-entry (PF0 audit context). Re-testing any of these on a **real** COMPLETE cohort requires a new H8 registration with a materially new mechanism (`SOUL.md` §5 rejected-registry rule).

## Frontier families (SOUL.md §5)

**Gate order for data:** a real COMPLETE cohort containing actual launches precedes any S11+ registration (`SOUL.md` §6). **That cohort EXISTS since 2026-11-06 (D1a): COMPLETE, 3,001/3,001 blocks, 0 gaps, 232 `create_v2` launches** (`docs/evidence/D1A-TIME-OF-DAY-CREATE-V2-20261106.md`) — real-data registration is lawful once D2 (reserve snapshots) and PR-2 (engine execution on the cohort) land.

| ID | Family | Status | Blocked by |
|---|---|---|---|
| S11 | Filtered under-6h entry (selection) | FRONTIER — UNIMPLEMENTED | D2 reserve snapshots + PR-2 execution on the D1a cohort (data gate RESOLVED 2026-11-06; 82–87% pump-tx failure regime is the entry-cost frame) |
| S12 | Quality-filtered graduation entry (event) | FRONTIER — UNIMPLEMENTED | D2 post-graduation reserve snapshots for Engine B (data gate RESOLVED 2026-11-06) |
| S13 | Mid-life slow mooner (survivor) | FRONTIER — UNIMPLEMENTED | G0 cohort + multi-day observation horizon |
| S14 | Provenance-filtered leader follow (WA4 continuation) | FRONTIER — UNIMPLEMENTED | Calm-window leader re-observation — all leaders observed to date lost money or had unknown basis (congestion storm) |
| EX | Exit engine (death-signal + staged exits) | COMPONENT — UNIMPLEMENTED | Attaches to any accepted entry family; evaluated A/B on identical fills |

## Wallet Alpha line

| ID | What | Status | Receipts |
|---|---|---|---|
| WA1–WA3 | Normalizer, leader P&L, delayed follower (code complete; live read-only smoke on 12 real txs) | COMPLETE (code) — follower loses at all delays vs S00 | `docs/evidence/WA1-WA3.md` |
| W01 | Naive wallet-follow, 6 delay arms, real buy signals | EXECUTED — REJECTED: net −250,308 lamports per arm across all 6 delays vs S00 = 0; verdict `EDGE_NOT_DEMONSTRATED`. Do not resubmit naive follow | `docs/evidence/HARNESS2-INTEGRATION.md` §E |
| W02 | Copyability-ranked follow vs S00/S02/RX — first real H8 tournament | PLANNED (needs a multi-hour calm cohort; only then is any holdout opening justified) | `docs/evidence/HARNESS2-INTEGRATION.md` §H |

## Registered hypotheses (H8 ledger)

| ID | Hypothesis | Status | Notes |
|---|---|---|---|
| H-0001 | W01 naive wallet-follow (preregistration + trial 1 recorded BEFORE the verdict; receipt sha `1ea8bf20…`) | EXECUTED — REJECTED (2026-11-05) | ⚠ The 2026-11-04 session note also described an unexecuted organic-flow template as "H-0001". The hash-chained ledger is authoritative for the ID sequence; resolve any citation against it |

## Rejected registry

Append-only, doctrine-level, lives in `SOUL.md` §5 — never duplicated here. Resubmission of any entry requires a materially new mechanism.
