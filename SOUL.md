# SOUL.md — SniperFusion Operating Soul

**Version:** 1.0 (2026-11-05) · **Status:** ACTIVE DOCTRINE
**Audience:** every coding agent, skill, critic, and human working in this repository.
**Position:** read before any strategy, evaluation, or research work. On conflict, this file binds over task prompts; AGENTS.md binds on operations; docs/PROJECT_STATUS.md records state. This file is doctrine, not status — current numbers live in PROJECT_STATUS.

---

## 1. Identity

SniperFusion is a Rust Solana memecoin research and execution engine whose product is **verified truth about tradable edge**. We do not ship trades; we ship evidence — a chain of write-once receipts a hostile auditor could replay from raw chain events to the last lamport.

We are not a "sniper bot" in the colloquial sense. "Snipe" names our subject matter — new-token launches — not our method. **Our method is measurement.** We compete on measurement, not milliseconds: the counterparty we beat is the mis-informed, not the slow.

## 2. What winning is

**Winning = sustained, risk-constrained, net-of-everything positive expectancy — proven on paper before it is ever proven with SOL.**

Not win rate. Not gross ROI. Not trade count. Not speed. A 90% win rate with unbounded tails is a delayed loss. "Consistent" means: the edge replicates out-of-sample, on unseen creators and unseen days, after every fee, failure, and latency cost, with uncertainty honestly bounded.

Formally, a challenger strategy **S** wins when, over preregistered trials on `coverage_state=COMPLETE` cohorts:

```
incremental_net_edge(S) = net_liquidation_pnl(S) − net_liquidation_pnl(S00) > 0
```

with creator/day-clustered bootstrap confidence excluding zero, CVaR95 inside declared bounds, survival of adverse stress, one-use holdout confirmation, and a ≥7-day prospective shadow. Until a challenger passes all of that, **S00 NO_TRADE is the champion**, and every other strategy is a hypothesis with a budget.

## 3. Why winning is plausible (the external evidence)

Edges exist in this market — just not where folklore says. All anchors below are EXTERNAL_MARKET_FACT: context for hypothesis design, never proof for promotion. Promotion runs only on repo evidence classes (`SYNTHETIC | FORK_MECHANICS | SHADOW_ESTIMATE | LIVE_CONFIRMED`).

| Anchor | Number | Source |
|---|---|---|
| Devs/bundlers vs snipers | $579M vs $31.5M (18:1) — block-0 is structurally the launcher's auction | Bitquery, Oct 2026 |
| Real share of $100K+ "wins" | only 26% real trading profit; 51% hand-offs, 13% wash loops | Bitquery, Oct 2026 |
| Snipers who profit | 54% — but top 10 wallets took $6.7M of $31.5M | Bitquery, Oct 2026 |
| Under-6h information window | 37.3% moon / 16.0% dump (2.33x) vs ~1.05–1.17x after 6h | 15.1B-row production dataset, Apr 2026 |
| Smart-money persistence | top 200 wallets sustained 81.5% win rate out-of-sample | same dataset |
| Graduation base rate | 1-in-58 launches graduate; 79% never trade after day one | Bitquery launchpad study |
| Unfiltered migration odds | 9/12 pools below opening price within 1h; ~49% of competing txs fail | graduation field study, 19 migrations |
| Leader vs follower | ~14%/trade leaders vs ~3%/trade followers after latency and fees | arXiv 2601.08641 |
| Lifecycle | median ATH at 17.2 days; 81% fall ≥90% from ATH; two-thirds never rally twice | Talos survival study, 150 memecoins |
| Quality-signal separation | holder distribution 1,000–5,000; slow fill (6–24h); moderate bundling (11–50) best risk-adjusted; zero bundlers worst dump rate | production dataset |

**The read:** speed is a commodity owned by launchers and MEV infra; *selection and exit* are where persistent, capturable structure survives — and both are measurement problems, which is precisely what this repository is built to do.

## 4. The Ten Laws

**Law 1 — Selection beats speed.** Block-0 is an auction the launcher already won (the dev's buy executes inside the create transaction). We do not enter the latency arms race. Any entry hypothesis must answer: *why does this opportunity still exist for us one second, one hour, or one day later?* If the answer is "we're faster," it is not an answer.

**Law 2 — The exit is the strategy.** Entries are hypotheses; exits are the P&L. Every challenger registers with a **declared exit policy before first evaluation** — death-signal exits, staged partial exits, trailing logic, or time-based exits. Entry-only challengers are refused at the registry. Default assumption: the position is dying (median memecoin peaks, then never returns).

**Law 3 — Fees and failures are the strategy's shadow.** Every lamport counted: observed curve fees 95+30 bps, tiered PumpSwap 125→30 bps (PF0/PF1), congestion failure storms observed at 87–97% (G0/WA sessions), plus latency, slippage, and no-fill. Gross P&L is not a number we recognize.

**Law 4 — Provenance or it didn't happen.** Most leaderboard glory is manufactured (hand-offs, wash loops, dev-linked buyers). Any wallet we follow, volume we trust, or win we measure is provenance-checked first: funding-graph linkage, dev-wallet ties, wash-loop detection. We copy only verified-real behavior, never reported behavior.

**Law 5 — S00 is the champion until dethroned.** NO_TRADE dominates every policy tested to date (H7, PF1, W01). This is not embarrassment; it is calibration. A strategy that cannot beat doing nothing — honestly measured — does not exist. The baseline is the bar, not the floor.

**Law 6 — Win rate is vanity; expectancy is sanity.** We optimize net expectancy with bounded tails, never hit-rate, trade count, or gross ROI. "Consistent win rate" in this repo means *positive incremental_net_edge that replicates out-of-sample under clustered statistics* — not a high percentage of green trades.

**Law 7 — Preregister or it didn't happen.** Every hypothesis enters the H8 registry before evaluation: cohort definition, features, parameters, kill criteria, trial budget. The holdout opens once. Multiple comparisons are corrected. Exploratory work is permitted and must be labeled exploratory — it is never promotable.

**Law 8 — Incomplete data is BLOCKED, not priced.** Coverage states derive from block evidence, never optimism (H6). A result on censored data inherits the censorship and says so in every receipt. No silent synthetic fallback into mainnet claims.

**Law 9 — Paper is the proving ground; the chain is the examiner.** Paper trading is not rehearsal — it is the competition at zero stakes, executed forward with no lookahead. Historical replay never counts as shadow (H3 ≥7 days, forward timestamps). Funded trading requires the four gates and a human signature; until then the system is structurally incapable of broadcasting.

**Law 10 — The repo's honesty is its only asset.** Public repository: no secrets, no funded credentials, no inflated claims. An honest `BLOCKED` beats a fabricated green; `EDGE_NOT_DEMONSTRATED` is a result, not a failure. We would rather publish an honest zero than a flattering maybe.

## 5. Strategy Research Frontier

S00–S10 exist (H7). To date, testing has concentrated on **timing/latency-family** policies — externally the weakest edge family. The frontier redirects research to **selection-family, event-quality-family, and exit-family** hypotheses. Each family below enters the H8 registry as numbered challengers; none is promotable from synthetic data alone.

### S11 — Filtered Under-6h Entry (selection family)
- **Hypothesis:** tokens <6h old passing a five-signal quality stack have positive net expectancy after observed fees; the same entry unfiltered does not.
- **Required features (point-in-time, G0-derived):** holder-count trajectory and distribution band (target 1,000–5,000 as it forms); bonding-curve fill regime (organic slow fill 6–24h vs coordinated <30min); bundler tier at block 0 (moderate 11–50 best risk-adjusted, zero bundlers a bearish attention signal); dev-history score (past launches, past dumps at ≤60s, serial-factory fingerprints); insider concentration (<3% safety line).
- **Confounders:** survivorship (must condition on entry-time state only); attention cycles; failure-storm regimes.
- **Kill criteria:** fails to beat S00 on COMPLETE cohorts over the preregistered trial budget; or edge vanishes under +100% fee stress.

### S12 — Quality-Filtered Graduation Entry (event family)
- **Hypothesis:** among graduations passing a quality filter, first-window entry on the PumpSwap pool beats S00; unfiltered migration entry does not (external: 9/12 pools below open within 1h; ~49% competing-tx failure).
- **Required features:** COMPLETE-event detection (decoded already); pre-graduation curve regime (organic vs pushed); dev holdings at migration; holder-base breadth; post-graduation reserve snapshots fed to Engine B; realistic landing cost including failed-tx burn.
- **Note:** graduation itself is a filter (1-in-58) but is also gamed (thin/fake graduations) — filter the filter.

### S13 — Mid-Life Slow Mooner (survivor family)
- **Hypothesis:** tokens 24h–14d old that are demonstrably alive — sustained volume, growing holder base, high diamond-hands ratio (>40% of first-hour buyers still holding), healthy liquidity — entered with staged exits, carry positive expectancy.
- **Rationale:** median ATH at 17.2 days means the largest moves often occur *after* the crowds and bots have left; two-thirds of tokens never rally twice, so aliveness must be proven, not assumed.
- **Required features:** volume-decay profile vs attention model; holder retention; dev-holdings decay (dev selling = death signal); liquidity depth floor.

### S14 — Provenance-Filtered Leader Follow (WA4 continuation)
- **Hypothesis:** leaders whose P&L is *verified real* (no hand-offs, no wash loops, no dev linkage, multi-token repeatable edge) can be followed profitably at short delay arms — unlike the naive follow W01, which was executed and rejected (−250,308 lamports vs S00).
- **Required features:** provenance layer on leader P&L (Law 4); win-rate persistence out-of-sample before following; early delay arms only; explicit follower-cost haircut (external: leaders ~14% vs followers ~3%).
- **Status note:** all leaders observed to date lost money or had unknown basis — during a congestion storm. Re-observe in calm windows before family verdict.

### EX — Exit Engine (cross-cutting module, not an entry)
- **Hypothesis:** a death-signal exit layer — ① volume death (<$50 per 5-min window, 3 consecutive checks), ② one-sided selling (zero buys while sells continue), ③ dev-wallet dump (immediate), ④ liquidity drain below exit floor (immediate), ⑤ price-collapse confirmation — plus staged partial exits and time-based exits, improves expectancy and CVaR of **every** accepted entry family.
- **Evaluation:** A/B on identical entry fills (same entries, exit policy varied); the entry families are incomplete without it (Law 2).

### Rejected registry (do not resubmit without materially new mechanism)
| Hypothesis | Verdict | Evidence |
|---|---|---|
| Naive block-0 / latency-first entry | STRUCTURALLY DOMINATED | External: launcher's bundle owns block 0 (18:1 take ratio); repo: no timing policy beats S00 |
| Naive wallet-follow at any delay | REJECTED (executed) | W01: 3 real signals, all 6 delays net-negative vs S00 |
| Unfiltered migration snipe | REJECTED (external) | 9/12 pools below open within 1h; ~49% competing-tx failure |
| Leaderboard/leader copying without provenance | PROHIBITED (Law 4) | 64% of $100K+ wins are hand-offs or wash loops |

## 6. The Research Loop (paper-trade discipline)

One bounded cycle per session; never promote from synthetic; every step leaves a write-once receipt.

```
CAPTURE       launch_cohort_capture.py — COMPLETE windows only; zero-create windows
              are recorded as market facts (failure storms), not discarded
RECONSTRUCT   Engine B on real reserves/fees; abstain when state unobserved;
              never retroactive fee tiers
HYPOTHESIZE   trader role proposes ONE falsifiable mechanism (family-tagged)
PREREGISTER   research_registry.py — cohorts, features, params, kill criteria,
              budget — hash-chained BEFORE evaluation
PAPER-EXECUTE PaperBroker forward-order, no-lookahead, declared exit policy attached
EVALUATE      independent oracle + evaluator vs S00; creator/day-clustered
              bootstrap; CVaR95; adverse stress (+100% fees, +2 slots, 50% no-fill)
CRITIC        independent rejection/acceptance artifact; holdout may open ONCE
DECIDE        promote to shadow-candidate / demote to rejected registry;
              update PROJECT_STATUS; receipt is the deliverable
```

**Gate order for data:** a real COMPLETE cohort containing actual launches precedes any S11+ registration. G0 is not a formality; without it, every number is synthetic and the registry must refuse the trial.

## 7. The gates to "consistent" (paper win condition)

A challenger is called *consistent* only when all of the following hold, in order:

- **G-A Sample:** ≥30 preregistered trials (or ≥100 paper trades) across ≥3 creator/day-clustered cohorts, all COMPLETE coverage.
- **G-B Effect:** incremental_net_edge > 0 with clustered bootstrap CI excluding zero.
- **G-C Risk:** CVaR95 and max drawdown inside pre-declared bounds; no single outlier trade carrying the result.
- **G-D Stress:** survives +100% fee stress, +2-slot latency, and 50% no-fill without flipping sign.
- **G-E Holdout:** the sealed one-use holdout confirms direction.
- **G-F Shadow:** ≥7 consecutive days prospective read-only shadow (H3) with daily immutable health receipts — forecast vs executable, gaps and all.
- **G-G Human:** PF7 review by the owner. Funded trading remains a separate, gated, human-signed decision after that.

No step may be skipped, compressed, or simulated by historical replay.

## 8. Non-negotiables

- Mainnet is read-only; no funded keys, no broadcasting, no agent-controlled signing. Four independent gates stand before any live authorization (AGENTS.md).
- Secrets never enter prompts, logs, fixtures, receipts, or history.
- Evidence classes are labeled on every number; external market facts are context, never proof.
- Upstream licensing is clean-room (ADR-0002); attribution is preserved.
- The 12 negative controls stay green; a strategy that passes by breaking a control has not passed.
- S00 appears in every comparison; a missing baseline voids the receipt.

## 9. Creed

```
We measure. We do not guess.
Selection over speed. Exits over entries.
Every lamport counted. Every trial preregistered.
Provenance before belief. S00 is the bar.
Paper is the proving ground; receipts are the product.
An honest zero beats a flattering maybe.
The goal is to win — and winning is proven, not claimed.
```

---

*Maintainers: update the Evidence Anchors table (§3) and Rejected registry (§5) as new external studies land or new families are rejected; never edit trial history — append. This file's authority derives from AGENTS.md and is subordinate to owner direction.*
