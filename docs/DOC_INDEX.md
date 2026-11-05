# Documentation Index — SniperFusion

**Version:** 1.0 (2026-11-05) · **Status:** ACTIVE · **Update rule:** every PR that adds, moves, or retires a document updates this index in the same PR.

This is the single map of every document in the repository. If a file is not listed here it is either new (add it here) or should not exist (remove it or archive it). State claims in any doc are verified against `docs/PROJECT_STATUS.md` and actual code at HEAD.

## How to read this repository

Read order and precedence are defined in the `AGENTS.md` header. Summary:

1. `SOUL.md` — doctrine: why we exist, what winning is.
2. `AGENTS.md` — operations: how every session works.
3. `docs/PROJECT_STATUS.md` — state: what exists today.
4. This file — where everything lives.
5. Everything else — loaded on demand per the tables below.

## Document status vocabulary

| Status | Meaning | Update rule |
|---|---|---|
| LIVING | Describes current state; expected to track HEAD | Update at every milestone close |
| POINT-IN-TIME | Plan / review / mission written against a baseline SHA | Never rewritten; superseded by newer docs or evidence |
| HISTORICAL | Superseded; kept for provenance | Read for history only; conflicts resolve against LIVING docs |
| RECEIPT | Write-once evidence | Immutable; corrections in new files (see `docs/evidence/README.md`) |

## Root

| File | Role | Status |
|---|---|---|
| `README.md` | Public entry point; honest capability statement; agent quickstart; tool table | LIVING |
| `AGENTS.md` | Operating rules for every agent session; router to all zones below | LIVING |
| `SOUL.md` | Doctrine: winning definition, Ten Laws, strategy frontier, gates G-A…G-G | LIVING (deliberate amendment only; append-only anchors) |
| `SECURITY.md` | Security policy: key hygiene, reporting, CI enforcement, trading-safety invariants | LIVING |
| `CONTRIBUTING.md` | Human-facing mirror of the operating rules | LIVING |

## docs/ — engineering and governance

| File | Role | Status |
|---|---|---|
| `PROJECT_STATUS.md` | Living state ledger: milestone table, session records, blockers | LIVING |
| `AGENT_OPERATIONS.md` | Agent Operating System: roles, three loops, safe commands, G0 protocol, handoff protocol | LIVING |
| `ARCHITECTURE.md` | Crates, event architecture, priority bus, tx lifecycle, RiskAuthorization, non-goals | LIVING — needs refresh after WA/Harness² landings |
| `CAPABILITY_MATRIX.md` | Chainstack parity rows C1–C24 + per-upstream verdicts | LIVING (per milestone) |
| `THREAT_MODEL.md` | Adversaries A1–A5, threats T1–T16, safety invariants, test mapping | LIVING |
| `TEST_PLAN.md` | Test surfaces and threat-to-test mapping | LIVING |
| `PROTOCOL_COMPATIBILITY.md` | Pump.fun/PumpSwap protocol audit facts (M0) | REFERENCE |
| `SOLANA_COMPATIBILITY.md` | Solana SDK generation spike evidence (M0) | POINT-IN-TIME |
| `UPSTREAM_MANIFEST.md` | Pinned upstream repos, licenses, attribution, amendments | LIVING (update on any pin change) |
| `UPSTREAM_INTEGRATION_MATRIX.md` | What each upstream contributes and how | LIVING |
| `NEXT_GOAL_H8_G0.md` | Active mission: unblock G0, build H8 council (PR-1…PR-4) | POINT-IN-TIME plan (active) |
| `PROFIT_FIRST_GOAL_PLAN.md` | H8 / P&L V2 goal plan | POINT-IN-TIME plan |
| `WALLET_ALPHA_GOAL_PLAN.md` | Wallet Alpha implementation contract | POINT-IN-TIME plan |
| `WALLET_ALPHA_UPSTREAM_AUDIT.md` | Upstream audit + amendments (WA0) | REFERENCE |
| `WALLET_ALPHA_MILLION_FEATURES.md` | million feature register: adopt / adapt / reject | REFERENCE |
| `PF0_PNL_AUDIT.md` | PF0 P&L V2 gap audit findings | POINT-IN-TIME |
| `DEVNET_EXECUTION_PILOT.md` | Devnet pilot runbook + hard safety rails | LIVING (while pilot active) |
| `GLOSSARY.md` | ID system and shared vocabulary (M/H/PF/INFRA/WA/G, S/RX/W, evidence classes, gates, stats, market terms) | LIVING |
| `GOALS.md` | The goal ladder: dependency-ordered living checklist (critical path, frontier, ladders, operator-pending) | LIVING |
| `STRATEGY_REGISTRY.md` | Living status of every policy, challenger, and registered hypothesis | LIVING (state only; doctrine stays in `SOUL.md` §5) |
| `RPC_PROVIDERS.md` | Provider capability and data-availability matrix; G0 critical-path tracker | LIVING |
| `DOC_INDEX.md` | This file | LIVING |

## docs/history/ — archived superseded documents

Superseded root-level documents moved out of the tree top on 2026-11-05. Content unchanged; status marked in-file.

| File | Role | Status |
|---|---|---|
| `AGENT_MASTER_PROMPT.md` | Original master build prompt (2026-11-02) | HISTORICAL — superseded where conflicting |

## docs/research/ — dated research memos

| File | Role | Status |
|---|---|---|
| `2026-11-02-initial-engineering-design-review.md` | Original 733-line engineering design review (moved from root `research.md`) | POINT-IN-TIME |

## docs/ADRS/ — architecture decision records

| File | Role | Status |
|---|---|---|
| `README.md` | ADR index, numbering status, process | LIVING |
| `_template.md` | Template for new ADRs | REFERENCE |
| `0001-rust-first.md` | Rust-native hot path, Python frozen oracle | ACCEPTED |
| `0002-clean-room-licensing.md` | Clean-room licensing policy for upstream references | ACCEPTED |
| `0003-solana-sdk-generation.md` | Solana SDK generation selection (Gen-2) | ACCEPTED |
| `0012-harness2-integration.md` | Google Harness² integration method | ACCEPTED |

Numbers 0004–0011 are unused (reserved by an interim plan, never written). Do not renumber 0012; the next ADR takes 0013.

## docs/evidence/ — write-once execution receipts

19 immutable receipts (M0…M3c, H0, H2, H4, H6, H7, PF0, PF1, INFRA ×3, G0-H8-session, WA1-WA3, HARNESS2-INTEGRATION, G0-SWEEP-20261105). Format contract and index: `docs/evidence/README.md`. Corrections land in new files, never edits.

## harness/docs/ — research-harness working documents

⚠ `AGENTS.md` §12 warning applies: several files below predate H6/H7 and carry stale implementation-state strings. On any conflict, `AGENTS.md`, `SOUL.md`, `docs/PROJECT_STATUS.md`, and `docs/evidence/*` outrank everything in this zone.

| File | Role | Status |
|---|---|---|
| `RESEARCH_LOOP_RUNBOOK.md` | Command-level research loop operations | LIVING (verify against `AGENTS.md` §7) |
| `RESEARCH_CYCLE_V1.md` | V1 research cycle contract | POINT-IN-TIME (V2 planned via PR-3) |
| `RAW_CAPTURE_V0.md` | Raw capture V0 spec | POINT-IN-TIME |
| `HARNESS_WORKFLOW.md` | Harness objective and workflow (V0-era) | POINT-IN-TIME |
| `AI_STRATEGY_HARNESS_V1.md` | Harness architecture V1 | POINT-IN-TIME |
| `H5_H8_GOAL_PLAN.md` | H5–H8 plan + acceptance matrix | POINT-IN-TIME |
| `MASTER_AGENT_MISSION_H5_H8.md` | Mission document for H5–H8 | POINT-IN-TIME |
| `AGENT_NEXT_GOAL.md` | Earlier next-goal brief | HISTORICAL |
| `NEXT_AGENT_RESEARCH_PROMPT.md` | Earlier master research prompt | HISTORICAL |
| `COPY_PASTE_AGENT_LAUNCH_PROMPT.md` | Mission-director launch prompt | POINT-IN-TIME |
| `PROFIT_FIRST_AGENT_MISSION.md` | Profit-first mission | POINT-IN-TIME |
| `WALLET_ALPHA_AGENT_MISSION.md` | Wallet Alpha mission | POINT-IN-TIME |
| `MEMECOIN_RESEARCH_QUESTIONS.md` | Research question bank | REFERENCE (living question bank) |
| `DEEP_RESEARCH_IMPLEMENTATION_REVIEW.md` | Deep-research review → engineering decisions (2026-11-03) | POINT-IN-TIME |
| `ENGINEERING_AUDIT.md` | Codebase audit (2026-11-03) | POINT-IN-TIME |
| `surfpool-pin.md` | Surfpool v1.7.0 pin + sha256 | REFERENCE |

## integrations/harness2/

| File | Role | Status |
|---|---|---|
| `README.md` | Harness² channel and install surface | LIVING |
| `worker/runner.spec.md` | Isolated worker spec (model-backed channel stays BLOCKED) | POINT-IN-TIME spec |
| `worker/OPERATOR_CHECKLIST.md` | Operator checklist for enabling the worker | LIVING (while blocked) |

## .agents/skills/ — 19 specialist playbooks

Routed per `AGENTS.md` §8; validated by `python3 harness/agentctl.py lint-skills`. These are documentation, not installed capabilities. Adding a skill = new `.agents/skills/{name}/SKILL.md` + routing line in `AGENTS.md` §8 + index entry here.

## .github/

| File | Role |
|---|---|
| `workflows/ci.yml` | Rust gates: fmt, clippy ×2, tests ×2, cargo-deny, history-aware secret scan |
| `workflows/harness-validation.yml` | Python harness tests, skill lint, synthetic smoke |
| `secret-allowlist.txt` | Exact-substring allowlist for the secret scan; every entry needs a justification comment |
| `PULL_REQUEST_TEMPLATE.md` | PR checklist enforcing the engineering loop |
| `ISSUE_TEMPLATE/research-hypothesis.md` | H8-aligned hypothesis intake |
| `ISSUE_TEMPLATE/bug.md` | Bug report contract (no secrets, no endpoints) |

## Where new documents go

| You want to write… | It goes to… | Rule |
|---|---|---|
| A permanent operating rule | `AGENTS.md` section | Function-organized, never dated log entries |
| Doctrine (winning, laws, gates, frontier) | `SOUL.md` | Deliberate, owner-visible amendment; append-only anchors |
| Milestone state | `docs/PROJECT_STATUS.md` | Evidence pointer required |
| An execution receipt | `docs/evidence/{ID}.md` | Write-once; contract in `docs/evidence/README.md` |
| An architectural decision | `docs/ADRS/00NN-slug.md` | Process in `docs/ADRS/README.md`; next free number 0013 |
| A new mission plan | `docs/NEXT_*.md` | Point-in-time; baseline SHA in header |
| A research memo / literature review | `docs/research/YYYY-MM-DD-slug.md` | Dated; never edited after close |
| A superseded/historical document | `docs/history/` | `git mv` + update every reference + index in the same PR |
| A strategy status change | `docs/STRATEGY_REGISTRY.md` | Via H8 registry + gates only |
| A specialist playbook | `.agents/skills/{name}/SKILL.md` | Must pass `lint-skills`; routed from `AGENTS.md` §8 |
| A provider / data-availability fact | `docs/RPC_PROVIDERS.md` | With receipt or probe output reference |

## Anti-sprawl rules

1. **One job per document.** If an existing doc already does the job, extend it instead of adding a near-duplicate.
2. **No new master prompts.** The `*_MISSION*.md` / `*_PROMPT*.md` files in this repo are point-in-time mission artifacts. New missions are dated goal plans under `docs/`, not new "master" files.
3. **Point-in-time docs carry a baseline SHA and status header** and are never silently rewritten.
4. **Every new doc lands in this index in the same PR.** A doc missing from this index is a bug.
5. **Deprecation is explicit.** Add `Status: HISTORICAL (superseded by <file>)` under the title; do not delete history.
6. **Generated artifacts are not documents.** Run receipts, manifests, and captures live under `harness/runs/` (gitignored) or are promoted to `docs/evidence/` as receipts — never committed as loose markdown.
