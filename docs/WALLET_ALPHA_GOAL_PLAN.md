# SniperFusion Wallet Alpha — Implementation Goal and H8 Profit Plan

**Baseline:** `main@3dd5693697dfa630d291074ec3de28bccc8276ca` at 2026-11-04. **This document is a future implementation contract, not a claim that the wallet tracker is already running.**

## Business goal / success definition

Build a sniper / selective-follow engine whose **full-portfolio, after-cost net liquidation P&L** exceeds doing nothing (S00), simple time-delay (S02), randomized eligible trades, and naive whale-following on **identical, out-of-sample market opportunities** without unacceptable tail loss.

The system must understand *why it makes or loses money*. A 99% win rate with one catastrophic unsellable token is not success. A headline whale P&L cannot be replicated automatically. No public signal guarantees "always profitable" wallets.

Existing PF0/PF1 assets are **not** to be rewritten: `crates/sniper-strategy/{fees,oracle,eval,receipt}.rs`, `sniper-runtime` paper broker, read-only H6/G0 collector, H8 registry, Surfpool M3c protocol tests. Evidence: `docs/evidence/PF0.md`, `PF1.md`, `G0-H8-session.md`. Synthetic PF1 cohort: 0.1 SOL virtual start; S00=0; best trader −64,253 lamports; all trading policies negative, `EDGE_NOT_DEMONSTRATED`.

## Architecture: two-timescale wallet signal system

```text
DISCOVERY / SLOW RESEARCH (minutes–days; NO mainnet sends)
  Pump.fun token cohorts + tracked pools + historical wallet trades
       │
       ▼
  Venue/balance-delta parser, transfer-vs-swap validation, wallet-owner resolver
       │
       ▼
  Coverage-aware wallet P&L via Rust oracle + creator/insider/cross-DEX labels
       │
       ▼
  Copyability backtest over multiple follower delays, candidate whitelist + scores
       │
       └── immutable watchlist version, source labels, expiry, score/uncertainty
                                     │
                                     ▼
FAST RUST STREAM (preindexed roster only; event-driven)
  Yellowstone transaction filter / WSS fallback + slot/fork supervision
       → canonical WalletTradeEvent (signature, venue, owner, delta, observed_at)
       → typed safety / creator-risk gate
       → follower decision within bounded latency
       → READ-ONLY shadow or PaperBroker; NO funded signer
       → persisted wallet positions and P&L V2 / H8 evidence
```

**Do not promise instantaneous wallet discovery.** Detection of a *known, preindexed wallet's new trade* can be low-latency if a measured stream supports it. Finding an unknown trader, reconstructing its full wallet history, scoring it and determining identity cannot be reduced to milliseconds by naming a library.

### Production Rust contracts (target, not yet shipped)

`WalletTradeEvent`:
- `chain_genesis`, `signature`, `outer_ix`, `inner_ix`, `event_ordinal`, `slot`, `commitment`, `program_version`;
- `wallet_owner`, `authority_provenance`, `mint`, `venue`, `buy_sell_transfer_unknown`, `raw_token_delta`, `raw_quote_delta`, `network_fee`;
- `chain_observed_at`, `provider_seen_at`, `agent_decision_at`, `quote_available_at`, `source_provider`, `coverage_status`, `classification_confidence`;
- no routing keys, no signing material, no unverified influencer label in the economic identity.

`WalletCandidate`: wallet, roster version, first/last seen, eligible observable history coverage, historical full-liquidation P&L + independent oracle, realized/unrealized amounts, open/unexitable inventory, owner cluster, creator exposures, fee/capital/holding metrics, confidence/unknown count, label attribution source+age, **ranking_as_of** before follower signal.

`FollowerDecision`: signal identity, decision-time knowledge cut-off, candidate score snapshot, token safety verdict and reasons, venue-specific executable quotes, entry/exit budget, slippage/tip/fee caps, paper order or explicit NO_TRADE reason.

`FollowerPnLReceipt`: separate leader vs follower portfolio accounts, quote freshness, follower observation delay, entry/exit adverse prices, fee waterfall, realized/unrealized and conservative liquidation NAV, missed/excluded opportunities, risk flags, paired S00/naive/delayed comparisons, dataset and strategy hashes, proof class.

### Correct wallet trade attribution

- Resolve the **wallet owner**, token accounts, fee payer, vaults and router authorities, not merely "any account touched by a transaction."
- Use signed, integer pre/post SOL/WSOL/USDC/token balance changes, wrap/unwrap/rent and routed inner swaps; do not misclassify LP deposit, transfer, airdrop, token authority move, arbitrage multi-hop or failed tx as a buy.
- Dedup provider duplicates and commitment upgrades without erasing orphan fork history; mark unknown owner/mint/venue as UNKNOWN, fail closed for wallet scoring.
- Treat historical leader performance based on *all positions and cash flows*, not only winners/closed positions; unknown opening balances mean INCOMPLETE.
- Price follower entry and exit **at or after signal observation** from as-of executable reserves/order paths. Never use leader's earlier price or candle close after the decision.
- Limited historical coverage is a labeled uncertainty, not a high confidence wallet score.

### Token / degen / narrative intelligence

Compose 3 independent components:
1. **Hard RiskPolicy**: token program, mint/freeze/Token-2022 delegation/transfer-hook/fee flags, sell simulation, liquidity, slippage and gas/tip caps. Missing required state -> BLOCK/WATCH, not eligible.
2. **Creator & FlowModel**: net creator buys/sells (exclude fees/rent), related funding clusters with uncertainty, distinct-owner buyer breadth, holder concentration, same-slot bundled activity, migration route validity, developer dumps. Cluster suspicion is a *signal*, not proof of wrongdoing.
3. **OpportunityModel**: price impact-adjusted flows, wallet-copyability, independent confirmation, market regime and bounded narratives. Viral Twitter/KOL prominence must be optional enrichment and may be delayed/unverifiable; label public sources and do not turn influencer names into buy permissions.

### Wallet ranking: prioritize follower economics

Use **copyability-adjusted expected net edge** and uncertainty, not follower count, star ratings, leader's historical win rate or wallet name. Compare at least 0.5, 1, 3, 10, 30, 60 second observed follower delays when live data supports it. Stratify by hold-time, venue, token liquidity, creator identity and signal crowding.

`follower_net_pnl = exit_quote_received_after_pool_fees_and_impact - entry_quote_debited_inclusive_of_pool_fees_and_impact - independent_network_fees - paid_priority_fees - tips - any additional charged costs`.

Fees embedded in quote amounts must be disclosed but **never subtracted again**. Compare shared-capital NAV using the existing Rust Oracle; a stand-alone roundtrip batch is *not* a capital-constrained portfolio P&L result.

`incremental_edge = follower_net_liquidation_pnl - matched_baseline_net_liquidation_pnl`. Report SOL and separately SOL/USD exposure change. Negative or uncertain means NO_TRADE. Include all operator/provider costs before claiming an operating business profit.

## Delivery phases (small independently gated PRs)

| Phase | Change | Acceptance gate |
| --- | --- | --- |
| **WA0 — this PR** | Upstream code/license audit, wallet tracker comparison, full research architecture, an isolated **offline paired-signal evaluator spike** and agent skill/mission | All Python harness tests green, honest synthetic result, no new runtime broadcaster or secrets |
| **WA1 — wallet normalization** | Rust read-only `wallet_observer`, owner/ATA resolver, token/SOL balance-delta DEX event parser, signed int math and covered backfill | Fixtures for Pump.fun, PumpSwap, Jupiter multi-hop, Raydium, Meteora, transfers, LPs, token-2022 and failed tx; cross-check independently with Dorado/Solsonar on known txs, >=0 duplicates on replay |
| **WA2 — real roster & accounting** | Token→wallet and wallet→token discovery; exclusion of pool/router/LP/creator misattribution; full historical wallet state to independent oracle; copyability rank | Correct withdrawal-adjusted NAV, no closed-winner-only bias, explicit missing-history flags, no future reputation leak; repeatable roster snapshot |
| **WA3 — paper follower** | Rust `wallet_follow` policy, realistic quote-available-at delays, dynamic no-fill/fee/impact scenarios, leader vs follower portfolios, exited positions | Paired results for same signals; 500ms..60s delays; slippage stress; no-funded execution; invalid data refuses trade |
| **WA4 — creator & alpha** | Derive creator first buys, dump risks, buyer owner breadth, smart-wallet consensus, migration and social narrative features | Separate hard vs heuristic checks, backtest using only pre-decision features; negative controls and ablations vs S00, S02, RX, naive follow |
| **WA5 — H8 tournament** | Preregister hypotheses, independent critic, creator/day temporal holdout protected across OS principals, multiple testing correction, cost and sensitivity experiments | Enough independent wallets/trades to estimate uncertainty; no signal promoted from tiny or missing data; all trials counted, no access to holdout labels by proposers |
| **WA6 — forward shadow/ops** | Read-only mainnet provider shadow for ≥7 consecutive days with coverage ledger, latency histograms, recurring P&L V2 dashboard | Actual event-to-decision p50/p95/p99 and observed quote ages; complete daily P&L with no-lookahead, provider outage/failure and false-positive receipts |
| **WA7 — speed/DEX** | Benchmark Yellowstone, Carbon, current Rust ingester, Jito/Jupiter *simulated* routing and venue adapters | Only integrate when lower p99 **and** better after-cost executable P&L/robustness; no live mainnet trades without separate explicit approval |

### First strategy tournament (pre-registered hypotheses)

- **W01**: naive whale follow (negative-control baseline).
- **W02**: copyability-ranked wallets only (past confirmed full NAV; exclude abnormal creator/pool owners).
- **W03**: independent multi-wallet consensus (cluster-collapsed, distinct owners).
- **W04**: creator net-selling veto of W02/W03.
- **W05**: whale-selling absorption (verified depth and subsequent independent buyers, not impulse sell-to-buy).
- **W06**: migration/post-graduation momentum with verified sell routing.
- **W07**: narrative + flow (narrative alone never overrides risk or grants a buy).
- **W08**: volatility/liquidity-adaptive exit vs fixed TP/SL.
- **Controls**: S00, S02, RX and random watched-wallet selection.

All are hypotheses, **not** claimed profitable. One experiment per preregistered card, bounded total budget, full record of misses. Reject strategies that only outperform on the same data used to select wallets.

### Performance and provenance targets

**Initial engineering targets (not measured):** event receipt→decoded p99 under 10ms (local); decision→authorized paper intent p99 under 20ms at warm cache; 0 duplicate economic intent on timeout, 0 unlogged critical gaps. Event→landed is endpoint/regional/relay dependent and cannot be asserted without actual measurements. Stop discovery from blocking sell/reconciliation hot paths.

Track `wallets_discovered`, `wallets_verified`, `wallet_rank_staleness`, `signal_age_ms`, `false_wallet_attribution`, `data_coverage`, `leader_pnl`, `follower_pnl`, `copy_penalty`, `fees`, `rejected_signals`, `max_drawdown`, `equity/NAV`, `narrative_source_age`, `p99_lag`.

### Security and licensing

Existing RPC key remains unchanged and secret. Separate operator read-only provider from agent research (sanitized fixture/receipt access). Do not copy or execute Yuto/M8S/MEV/HandiCat/justFive source without explicit rights. MIT sources may be used under notice obligations but are never automatically granted network/key execution permissions. Any new third-party service (Helius, Rugcheck, GMGN, Birdeye, DexScreener) requires terms, key handling and rate-limit review; no scraping private API endpoints or paying for infrastructure without owner approval.

### Required handoff

Every PR reports: source SHAs/licenses, data origin and proof class, files changed, exact test commands/exit codes, signed/integer economic invariant checks, cross-provider discrepancies, S00 and matched follower P&L, no-trade fraction, CI status, remaining blocker and next falsifiable test. If only synthetic, mark `SYNTHETIC_TESTED`, `promotable=false`; never call it live profit.