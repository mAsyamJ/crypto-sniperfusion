# Glossary — SniperFusion ID System and Shared Vocabulary

**Version:** 1.0 (2026-11-05) · **Status:** ACTIVE · **Update rule:** add terms in the same PR that introduces them.

One term, one meaning. When a session meets an ID it does not recognize, it looks here first, then at the authority doc. Do not invent new IDs ad hoc; mint them through the owning zone (see `docs/DOC_INDEX.md`).

## Milestone and workstream IDs

| ID | Meaning | Authority |
|---|---|---|
| M0 | Audit, baseline, dependency selection, protocol verification | `docs/evidence/M0.md` |
| M1 | Functional Rust core, config, risk authz, safe replay runtime | `docs/evidence/M1.md` |
| M2 | Pump.fun event decoding + streaming (M2a fixture differential 320/320; M2b providers) | `docs/evidence/M2.md` |
| M3 | V2/V3 + PumpSwap lifecycle, builders, paper fill engine; M3c = real-ELF execution on Surfpool fork | `docs/evidence/M3.md`, `M3c.md` |
| M4–M9 | Planned: Chainstack parity closure; token intelligence + DevFlowMonitor; multi-DEX + copy trading; execution routes + fault injection; replay/benchmarks; hardening + release | `docs/PROJECT_STATUS.md` |
| H0 | Security hardening: sealed `AuthorizedOrder`, durable journal, history-aware secret scan | `docs/evidence/H0.md` |
| H1 | Real pump.fun/PumpSwap program ELF execution on a local Surfpool mainnet fork (recorded with M3c) | `docs/evidence/M3c.md` |
| **H2** | **Paper broker** (event-sourced, forward orders, no lookahead). ⚠ Name collision: Google **Harness²** is a *different* thing — always written "Harness²", evidenced in `HARNESS2-INTEGRATION.md`, never "H2" | `docs/evidence/H2.md`, `HARNESS2-INTEGRATION.md` |
| H3 | ≥7-day prospective read-only shadow window — NOT_STARTED; historical replay never counts | `docs/PROJECT_STATUS.md` |
| H4 | Harness CLI skeleton (`snf-harness` doctor/verify/replay/fork/shadow/evaluate/compare/promote) | `docs/evidence/H4.md` |
| H5 | Devnet protocol pilot + offline research loop (draft PR #6) | `docs/DEVNET_EXECUTION_PILOT.md` |
| H6 | Canonical dataset contracts + bounded creation-cohort collector; coverage states | `docs/evidence/H6.md` |
| H7 | Strategy SDK (S00–S10, engines A/B), accounting oracle, evaluator + 12 negative controls | `docs/evidence/H7.md` |
| H8 | Research registry: hash-chained preregistration, one-use holdout, multiple-testing corrections, council | `docs/evidence/G0-H8-session.md` |
| PF0 | P&L audit + on-chain fee provenance | `docs/PF0_PNL_AUDIT.md` |
| PF1 | P&L V2: double-entry oracle, reconciled equity, deterministic receipts | `docs/evidence/PF1.md` |
| PF5 | Alias used in some plans for the H8 registry machinery | `docs/PROFIT_FIRST_GOAL_PLAN.md` |
| INFRA / INFRA-2 / INFRA-3 | Disposable wallets + RPC validation / `sniper rpc-probe` integration / mainnet-readiness watch wallet | `docs/evidence/INFRA-*.md` |
| G0 | Block-verified **real** launch cohort (the data milestone). Distinct from gates G-A…G-G | `docs/evidence/G0-H8-session.md` |
| WA0–WA-full | Wallet Alpha line: WA0 audit + offline lab; WA1 normalization; WA2 leader P&L (`sniper wallet-pnl`); WA3 delayed follower; WA-int million/handi-cat integration; WA-full owner-graph + consensus/ladder + Dorado differentials | `docs/WALLET_ALPHA_UPSTREAM_AUDIT.md`, `docs/evidence/WA1-WA3.md` |
| WA4 | Planned token-safety / provenance layer; continuation line into S14 | `docs/evidence/HARNESS2-INTEGRATION.md` §H |
| WA6 | Live-shadow stage for Wallet Alpha (alias of H3 in the WA plan) | `docs/evidence/HARNESS2-INTEGRATION.md` §G |
| W01 / W02 | Executed strategy runs on the WA line (see Strategy Registry) | `docs/STRATEGY_REGISTRY.md` |
| H-0001 | First hash-chained hypothesis registration in the H8 registry | `harness/runs/…/h8-registry.jsonl` (ledger is authoritative) |

## Strategy IDs

| ID | Meaning | Status authority |
|---|---|---|
| S00 | NO_TRADE — the champion baseline. `incremental_net_edge` is always measured against S00 | `docs/STRATEGY_REGISTRY.md` |
| S01–S10 | Implemented policies (H7), run through Engine A/B + PF1 oracle. S02 = delayed-entry | `docs/STRATEGY_REGISTRY.md` |
| RX | RANDOM_ELIGIBLE — deterministic random-entry control: `FNV(event_id) < 2000/10000` | `docs/evidence/PF1.md` |
| S11–S14 | Frontier families (filtered under-6h entry; quality-filtered graduation; mid-life slow mooner; provenance-filtered leader follow) | `SOUL.md` §5 |
| EX | Exit engine — cross-cutting death-signal/staged-exit module, not an entry policy | `SOUL.md` §5 |

## Components

| Term | Meaning |
|---|---|
| Engine A | `LegacySurrogateV1` surrogate pricing engine; byte-exact vs the frozen Python fixture |
| Engine B | Protocol-quote engine on real observed reserves/fees; refuses unobserved state and retroactive fee tiers (abstains rather than guesses) |
| PaperBroker | Event-sourced paper execution: fills, fees, rent, slippage; forward orders only, no lookahead |
| PF1 oracle | Independent double-entry accounting truth: lamport-exact reconciliation, FIFO lots, equity waterfall; the sole portfolio accountant |
| Evaluator | Full-portfolio evaluation vs S00: CVaR95, profit factor, attribution, outlier dominance, 12 negative controls |
| `research_registry.py` | H8 preregistration ledger: hash-chained specs, structural 20-trial bound, cross-process one-use holdout, BH + Bonferroni |
| `launch_cohort_capture.py` | Read-only Mainnet G0 collector (COMPLETE windows only) |
| `snf-harness` | Harness CLI: doctor / verify / replay / fork / shadow / evaluate / compare / promote (promotion is paper-only) |
| `agentctl.py` | Read-only dispatch helper: doctor / skills / commands / lint-skills / research-demo / harness2 group. Not an autonomous agent, never reads keys |
| `rpc_capability_probe.py` | ≤5 read-only calls: genesis, finalized slot, one full `getBlock` sample; sanitized write-once receipt |
| `wallet_alpha_lab.py` / `wallet_watch.py` | Offline synthetic WA lab / offline alert ledger (no network) |
| Harness² | Google research framework (`google-research/harness2`, Apache-2.0), pinned checkout under `.external/` per ADR-0012; deterministic channel only, model-backed channel BLOCKED behind the isolated worker |

## Evidence classes (two ladders — never mix them)

**Chain-execution ladder** (what a *market claim* is worth — `SOUL.md` §3, `AGENTS.md` §11):

`SYNTHETIC` → `FORK_MECHANICS` → `SHADOW_ESTIMATE` → `LIVE_CONFIRMED`

**Implementation-proof ladder** (what an *engineering claim* is worth — `docs/AGENT_OPERATIONS.md`, `docs/evidence/`):

`VERIFIED_CODE`, `SYNTHETIC_TESTED`, `SURFPOOL_REAL_PROGRAM_FORK_EXECUTED`, `DEVNET_PROGRAM_TRANSACTION`, `MAINNET_READ_ONLY_OBSERVED`, `BLOCKED`

Rules: a claim is labeled with the lowest class its evidence actually supports; synthetic outputs are permanently `promotable=false`; passing CI proves correctness, never market edge.

## Coverage and data states

| Term | Meaning |
|---|---|
| COMPLETE / INCOMPLETE / CENSORED / UNKNOWN | Coverage states derived from block evidence only (H6); never from optimism |
| Zero-create window | A COMPLETE window with zero successful launches — a market fact (failure storm), not a capture failure |
| Failure storm | Regime where most pump-program transactions fail (observed 87% during the 2026-11-04 capture; 270/1000 successful in the calmer 11-05 window) |
| BLOCKED_RPC_CAPABILITY | The honest classification when a provider cannot serve the evidence a run needs — it is never a green run |

## Screening, risk, and safety states

| Term | Meaning |
|---|---|
| ELIGIBLE / WATCH / BLOCK | Token-intelligence screen states (M5). ELIGIBLE means "criteria met" — never "safe" or "profitable"; no score overrides a BLOCK |
| RiskAuthorization | Typed authorization object consumed by execution; construction is possible only after risk checks pass — bypass is unrepresentable |
| Four live gates | `live` cargo feature ∧ operator authorization token ∧ validated budget config ∧ kill-switch check — all four, or no broadcasting |
| Kill switch | Gates RiskAuthorization *issuance*, not just submission |

## Verdicts and markers

| Term | Meaning |
|---|---|
| EDGE_NOT_DEMONSTRATED | The standing verdict: no tested policy beats S00. A result, not a failure |
| `promotable=false` | Permanent label on synthetic-derived outputs; feeding them to `promote` is prohibited |
| S00 dominates | The measured outcome pattern: S00 net P&L ≥ every challenger on every cohort tested to date |
| FULL_BLOCK_SAMPLE_AVAILABLE | `rpc_capability_probe` success marker — provider compatibility only; proves nothing about G0 coverage or profitability |
| MAINNET_READ_ONLY_OBSERVED | Facts observed via read-only mainnet calls; no broadcast implied |

## Gates

| Term | Meaning |
|---|---|
| G-A…G-G | Research promotion gates in order: Sample (≥30 trials / ≥100 paper trades, ≥3 clustered cohorts) → Effect (incremental_net_edge > 0, clustered bootstrap CI excludes 0) → Risk (CVaR95 + drawdown bounds, no outlier carrying the result) → Stress (+100% fees, +2 slots, 50% no-fill) → Holdout (one-use, direction confirms) → Shadow (≥7 prospective days) → Human (owner review; funded trading is a separate human-signed decision) |

## Statistical protocol terms

| Term | Meaning |
|---|---|
| Preregistration | Hypothesis, cohort, features, parameters, exit policy, kill criteria, and trial budget recorded hash-chained BEFORE evaluation (`SOUL.md` Law 7) |
| incremental_net_edge | `net_liquidation_pnl(S) − net_liquidation_pnl(S00)` — the only number that can promote a challenger |
| Creator/day-clustered bootstrap | Resampling that preserves creator and day clusters; protects against n-of-1 creators inflating significance |
| CVaR95 | Conditional tail expectation at 95%; declared 20-trade minimum |
| BH / Bonferroni | Multiple-comparison corrections applied by the H8 registry |
| One-use holdout | Sealed label partition that may be opened once, cross-process enforced (flock + O_EXCL + persistent counter) |
| Negative controls | The 12 mandated checks that must fail when the machinery is broken |
| Adverse stress | +100% fees, +2-slot latency, 50% no-fill — a challenger must survive without flipping sign |

## Protocol and market terms (as used in this repo)

| Term | Meaning |
|---|---|
| Bonding curve | Pump.fun launch pricing curve; graduation happens when the curve fills |
| Graduation / migration | Bonding-curve → PumpSwap pool transition. A filter (≈1-in-58) that is also gamed — thin/fake graduations exist; "filter the filter" |
| Curve fill regime | Organic slow fill (6–24h) vs coordinated fast fill (<30min) — an S11 feature |
| Bundler | Actor bundling create+buy in one atomic bundle at launch; counted per token at block 0 (S11 feature) |
| Dev dump | Creator-linked net selling — an immediate exit trigger (EX; T6) |
| Diamond-hands ratio | Share of first-hour buyers still holding; >40% is the S13 aliveness signal |
| Insider concentration | Creator-linked share of holdings; <3% is the S11 safety line |
| Failure storm | See Coverage states above |
| Fee truth | On-chain-observed fees only: curve 95+30 = 125 bps; PumpSwap tiered 125→30 bps. The 25 bps figure is a reference fixture, never a live assumption |
| Lamport | 10⁻⁹ SOL; the integer accounting unit (checked math, no floats) |
| bps | Basis points (1/100 of a percent) |
| Surfpool | Local mainnet-fork execution tool, pinned v1.7.0 (`harness/docs/surfpool-pin.md`) |
| LiteSVM | In-process Solana VM used for program-execution tests (M3) |
| Jito / Jupiter | Future execution routes (M7, `docs/ARCHITECTURE.md`); no code exists today |
