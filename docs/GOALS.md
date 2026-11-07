# Goals — the SniperFusion goal ladder

**Version:** 1.0 (2026-11-05) · **Status:** ACTIVE (LIVING) · **Owner:** repository owner · **Compiled from:** `SOUL.md`, `AGENTS.md`, `docs/PROJECT_STATUS.md`, `docs/NEXT_GOAL_H8_G0.md`, `docs/PROFIT_FIRST_GOAL_PLAN.md`, `docs/WALLET_ALPHA_GOAL_PLAN.md` at `f51ab4f`.

**Update rule:** this file tracks *what remains, in dependency order* — it is the checklist view, not the state authority. Strategy states → `docs/STRATEGY_REGISTRY.md`; milestone history + session records → `docs/PROJECT_STATUS.md`; doctrine (winning, laws, gates) → `SOUL.md`. Flip a box only when the owning system says so (H8 registry verdict, gate passage, merged PR with evidence receipt) — never from memory or chat.

**The mission (`SOUL.md` §2):** a strategy with positive `incremental_net_edge` vs S00 NO_TRADE that replicates out-of-sample on `coverage_state=COMPLETE` real cohorts, passes gates G-A…G-G, and survives adverse stress. Until then **S00 NO_TRADE is the champion** and every item below is a hypothesis with a preregistered budget.

## 1. Done — verified with receipts (compact)

- [x] M0–M3c — audit; Rust core/protocols/runtime; 320/320 decode differential; **real pump.fun ELF execution on Surfpool fork**
- [x] H0, H2, H4 — security hardening; event-sourced PaperBroker; harness CLI (promotion structurally requires shadow)
- [x] INFRA 1–3 — wallets; `sniper rpc-probe`; mainnet read-only readiness
- [x] H6 — dataset contracts, coverage states, G0 collector (code)
- [x] H7 — strategy SDK S00–S10, Engine A/B, independent oracle, evaluator + 12 negative controls
- [x] PF0/PF1 — fee truth on-chain-observed (curve 95+30 bps; PumpSwap tiered 125→30); P&L V2 lamport-exact
- [x] H8 registry machinery — hash-chained preregistration, 20-trial bound, cross-process one-use holdout
- [x] WA0→WA-full — wallet normalization, leader P&L, delayed follower, owner-graph/consensus/ladder, Dorado differentials
- [x] W01 naive follow — executed and **REJECTED** (all 6 delay arms −250,308 lamports vs S00); a completed trial, negative result
- [x] Harness² deterministic channel (ADR-0012); documentation layer (`c35e6b3`, `f51ab4f`)

Standing verdict everywhere: **`EDGE_NOT_DEMONSTRATED` — S00 dominates every tested policy.**

## 2. Now — the critical path (data before hypotheses)

`SOUL.md` §6: a real COMPLETE cohort containing actual launches precedes any S11+ registration. ~~One blocker gates the entire frontier~~ **BLOCKER RESOLVED (2026-11-06, D1a): the "zero launches" market fact was a decoder blind spot, not market structure.** The identical window `[465625744, 465628744]` re-captured with the D0-fixed collector is COMPLETE with **232 create events / 232 unique mints** (all `create_v2`; 181 SOL-quote / 51 alt-quote; ≈1.3k launches/hour; launch-time reserves decoded in every record) — the first real COMPLETE cohort containing actual launches in repo history. Receipt: `docs/evidence/D1A-TIME-OF-DAY-CREATE-V2-20261106.md`. Frontier gate is now D2 reserve snapshots → PR-2 on this cohort → S11+ preregistration.

- [x] **G0 at scale** — COMPLETE windows containing real launches. **DONE at both coverage and content level (2026-11-06, D1a)**: 3,001/3,001 blocks, 0 gaps, **232 launches** (`manifest-20261106T140626-7720-*`). Time-of-day probes: creation NOT hour-gated (6 vs 7 ok per 60 sampled blocks, quiet vs US-evening). Historical receipts superseded on this point: `docs/evidence/G0-WINDOW-COMPLETE-20261105.md` + `docs/evidence/D0-LAUNCH-ATTRIBUTION-AUDIT-20261105.md`.
- [ ] **Reserve/fee snapshots as-of decision** — the next collector capability; Engine B refuses unobserved reserves (abstains, never guesses)
- [ ] **PR-2** — Engine A/B + independent NAV on the identical real cohort (`docs/NEXT_GOAL_H8_G0.md`)
- [ ] **W02 tournament** — copyability-ranked follow vs S00/S02/RX on a multi-hour real cohort; only then is any holdout opening justified

## 2A. Owner priority — filtered, real-data PAPER profitability (2026-11-07)

**Contract:** `docs/NEXT_GOAL_24H_FILTERED_PAPER_RESEARCH.md`; **research:** `docs/research/2026-11-07-multisource-token-reach-and-filtering.md`; **launcher:** root `GOAL_PLAN_PROMPT.md`. These are **UNIMPLEMENTED** until evidence/owner system says otherwise. The 232-launch D1a COMPLETE census is done, but NO profitable real-cohort strategy or 24/7 paper execution has been demonstrated.

- [ ] **D2A/D2B** — verify locally available 232-launch raw cohort + immutable hash; build as-of-decision reserve/fee/config snapshots (181 SOL quote; 51 unsupported alternate quotes require explicit abstention).
- [ ] **PR2** — run Engines A/B and independent PF1 NAV on the identical real cohort; report realistic modeled fills, failed fees, exit/unexitable exposure and S00.
- [ ] **F0/F1** — mandatory universal G1–G7 deterministic filtering for provider, token, wallet, dataset, Harness² proposal, execution and dashboard boundaries; Token-2022/MEV/drainer/honeypot negative controls.
- [ ] **API0** — documented DexScreener REST/WebSocket and Jupiter Tokens V2 adapters, provenance/causal timestamps, bounded rate limits, fixtures and quote-only use.
- [ ] **API1** — first-party Raydium LaunchLab and Meteora DBC on-chain launch/venue adapters; measure verified marginal token reach.
- [ ] **API2** — conditional Birdeye, Bitquery, CoinGecko/GeckoTerminal, Shyft, GoPlus/RugCheck, Helius and Orca additions only after explicit auth/quota/terms/cost and source-value probes.
- [ ] **OPS24** — 24h no-AI, no-signer paper daemon fault-injected soak with restart/replay and gap receipts; not the ≥7-day prospective strategy shadow.
- [ ] **S11EX + STRAT** — real-data H8 preregistered filtered entry/EX exit and S12/S13/S14 follow-on with no lookahead, independent critic, honest S00 comparisons and stress.
- [ ] **H2SAFE** — genuine Harness² deterministic channel expanded with safe benchmark negative-controls; model-backed worker stays operator-blocked; offline RL only, never direct live policy change.
- [ ] **SHADOW7** — ≥7-day genuine forward read-only shadow, one-use holdout, human paper-only promotion under G-A…G-G.

**Definition of profitable:** positive, risk-bounded, after-cost realizable incremental NAV versus S00 on unbiased creator/day OOS and shadow; not a guaranteed goal and not authorization for funded trades.

## 3. Strategy frontier (gated on §2; doctrine in `SOUL.md` §5)

- [ ] **S11** — filtered under-6h entry: holder band 1k–5k, organic curve-fill 6–24h, bundler tier 11–50, dev-history score, insider <3%
- [ ] **S12** — quality-filtered graduation entry: filter the filter (thin/fake graduations); post-graduation reserve snapshots for Engine B
- [ ] **S13** — mid-life slow mooner: 24h–14d, diamond-hands >40%, dev-holdings decay, liquidity floor — aliveness proven, not assumed
- [ ] **S14** — provenance-filtered leader follow (WA4 continuation): verified-real leader P&L, win-rate persistence OOS, early delay arms, follower-cost haircut
- [ ] **EX** — exit engine (cross-cutting): volume death, one-sided selling, dev dump, liquidity drain, price-collapse confirmation + staged partial/time exits; A/B on identical entry fills (`SOUL.md` Law 2: entry-only challengers are refused)

Rejected registry (`SOUL.md` §5, append-only): naive block-0/latency entry · naive wallet-follow any delay · unfiltered migration snipe · leaderboard copy without provenance. No resubmission without a materially new mechanism.

## 4. Verification ladder — how a challenger becomes champion

In order (`SOUL.md` §7); no step may be skipped, compressed, or simulated:

- [ ] **G-A** Sample — ≥30 preregistered trials (or ≥100 paper trades) across ≥3 creator/day-clustered COMPLETE cohorts
- [ ] **G-B** Effect — `incremental_net_edge > 0`, clustered bootstrap CI excluding zero
- [ ] **G-C** Risk — CVaR95 + max drawdown inside pre-declared bounds; no outlier carrying the result
- [ ] **G-D** Stress — +100% fees, +2-slot latency, 50% no-fill without sign flip
- [ ] **G-E** Holdout — sealed one-use holdout confirms direction
- [ ] **G-F** Shadow — ≥7 consecutive days prospective read-only shadow (H3/WA6; **NOT_STARTED** — cannot be replayed, only lived)
- [ ] **G-G** Human — PF7 owner review; paper-only promotion at first

## 5. Profit-first ladder (PF2→PF7; `docs/PROFIT_FIRST_GOAL_PLAN.md`)

- [ ] **PF2** — real G0 data with as-of account/reserve/fee/config state and gap receipts
- [ ] **PF3** — real Engine B: paired S00–S10 on same G0 data, conservative fill uncertainty, independent NAV
- [ ] **PF4** — profit tournament: one hypothesis per run, same-cohort paired P&L, adverse conditions, all variants counted
- [ ] **PF5** — H8 governance on **real** trials (machinery done; zero real trials executed)
- [ ] **PF6** — ≥7-day prospective shadow with coverage audit and modeled attainable prices
- [ ] **PF7** — human review & release: capacity, worst cases, kill switch, downgrade/rollback

## 6. Wallet Alpha ladder (WA4→WA7; `docs/WALLET_ALPHA_GOAL_PLAN.md`)

- [ ] **WA4** — creator & alpha features: creator first buys, dump risk, buyer breadth, smart-wallet consensus, narrative features; negative controls + ablations vs S00/S02/RX/naive-follow
- [ ] **WA5** — H8 tournament on wallet signals: preregistered, critic-separated, multiple-testing corrected
- [ ] **WA6** — forward shadow/ops: ≥7 days, coverage ledger, latency histograms, recurring P&L dashboard
- [ ] **WA7** — speed/DEX: benchmark Yellowstone/Carbon/Jito/Jupiter *simulated* routing; integrate only if lower p99 **and** better after-cost executable P&L

## 7. Engineering milestones (M4→M9; `docs/PROJECT_STATUS.md`)

- [ ] **M4** — Chainstack parity scope closure (rows C2–C24; LetsBonk/StonkFun adapter decision; `CONFIG_MIGRATION.md`)
- [ ] **M5** — token intelligence (ELIGIBLE/WATCH/BLOCK screen) + DevFlowMonitor (threat-model T3–T6 become live)
- [ ] **M6** — multi-DEX + copy trading surfaces
- [ ] **M7** — execution routes (DirectRpc/Jito/Jupiter), reconciliation, fault injection
- [ ] **M8** — replay/backtest/benchmarks
- [ ] **M9** — hardening + release readiness

## 8. Waiting on the operator (cannot be done by agents)

- [ ] Provide/confirm a block-serving mainnet RPC endpoint (env-only, never in chat/logs) and authorize a long calm-window G0 sweep
- [ ] Review `integrations/harness2/worker/OPERATOR_CHECKLIST.md` → provision the isolated worker to unblock model-backed Harness²
- [ ] Authorize + execute the Devnet pilot network run (H5-A preflight, then H5-B bounded buy→sell; code present, NOT EXECUTED)
- [ ] Decide the repository LICENSE (currently no general redistribution license; owner legal choice)

## 9. The far end — funded trading

- [ ] Four independent gates (cargo `live` feature ∧ operator authorization token ∧ validated budget config ∧ kill-switch check) **plus** a separate human-signed owner decision. No agent can authorize it; paper-only promotion comes first (`SOUL.md` §8).

## 10. Always-on invariants (checked every session, never "done")

- [ ] Mainnet read-only; no funded keys, no broadcasting, no agent-controlled signing
- [ ] S00 in every comparison — a missing baseline voids the receipt
- [ ] The 12 negative controls stay green; synthetic outputs permanently `promotable=false`
- [ ] Secrets never in prompts, logs, fixtures, or history; clean-room licensing (ADR-0002)
- [ ] Every number labeled with its evidence class; an honest `BLOCKED` beats a fabricated green
