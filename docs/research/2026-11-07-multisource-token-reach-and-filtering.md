# Multi-source Token Reach, Mandatory Filtering, 24/7 Paper Trading — Research Review

**Date:** 2026-11-07 · **Status:** POINT-IN-TIME RESEARCH, not implementation · **Baseline:** main at 314399959ab5f7573a25df77eed25991d9cae907.  
**Operator mission:** `docs/NEXT_GOAL_24H_FILTERED_PAPER_RESEARCH.md`; root `GOAL_PLAN_PROMPT.md`. Public API documentation was checked on 2026-11-07. Confirm terms, prices, and rate limits at the time of integration.

## Verified repository facts and core gap

- The corrected G0/D1a record in `docs/evidence/D1A-TIME-OF-DAY-CREATE-V2-20261106.md` reports 3,001/3,001 block coverage, zero gaps, **232 create_v2 launches** over roughly 10.7 minutes of chain time: 181 SOL-quote, 51 alternative-quote. An earlier zero-launch claim was a decoder blind spot. This is not a trade or a profitable result.
- `crates/sniper-strategy/src/engine/protocol.rs` has an as-of decision-time MarketBook and fee-aware Engine B, but rejects non-SOL quotes; snapshot collection from those real launches and actual Engine B execution remain open. `crates/sniper-runtime/src/runner.rs` defaults to ObserveOnlyHook; its screening enum alone is not a full filter architecture.
- `crates/sniper-runtime/src/ingest/provider.rs` and `ingest/supervisor.rs` implement ingestion abstractions, reconnect, and gaps, NOT proof of a production 24/7 system. `crates/sniper-runtime/src/paper.rs` implements simulated fills but has not demonstrated net-positive execution on the corrected cohort.
- `crates/sniper-core/src/authz.rs` and `crates/sniper-runtime/src/capability.rs` enforce execution boundaries. The dashboard remains a read-only projection; it must never accidentally gain signer/command capabilities.
- Genuine pinned Google Harness² at `integrations/harness2/` works on deterministic tasks. Model-backed worker remains operator-BLOCKED. H8 ledger and PF1 independent net P&L, not Harness² self-report, decide any paper strategy claims.
- **Standing verdict: EDGE_NOT_DEMONSTRATED; S00 NO_TRADE is champion.** A single 11-minute launch cohort cannot establish profitable trading or a ≥7-day prospective shadow.

## API/source inventory — official documentation and exact utility

A documented public API does NOT necessarily mean free, keyless, unrestricted production use or real-time. Onboard with measured latency, permission, fee/quota, data completeness and validated chain ID. Third-party summaries cannot substitute for on-chain reserve/fee/transfer state.

| Provider | Verified documentation | Reach and use | Role / caveat |
|---|---|---|---|
| Solana RPC/WSS and provider Yellowstone | https://solana.com/docs/rpc ; https://solana.com/docs/rpc/websocket/logssubscribe | Raw instructions, creates, account state, slots, tx results, commitment; main canonical truth. | Primary; `logsSubscribe` mentions one pubkey per subscription; gap backfill mandatory. |
| Official Pump.fun / PumpSwap IDLs | https://github.com/pump-fun/pump-public-docs | Creation v2, curve, graduation, fee accounts, Token-2022 specifics. | Primary protocol decode, pin IDL/revision; do not assume an official hosted market API. |
| Raydium v3 and LaunchLab | https://github.com/raydium-io/raydium-docs-v1/blob/main/sdk-api/rest-api.mdx | Pools, liquidity, LaunchLab mint discovery, token indexes, trades and history. | Cross-venue candidate source; reported indexed pool state can be minutes stale. |
| Meteora DBC / DAMM v2 | https://github.com/MeteoraAg/docs/blob/main/developer-guides/dbc/index.mdx | DBC pool creation, bonding curves, fee/config, migration and transfer-hook accounts. | Add first-party decode before relying on an aggregator's listing. |
| Orca Whirlpools | https://orca-so.github.io/whirlpools/ts/ | Concentrated-liquidity pool state, tick quotes and SDK. | Secondary liquidity reach, not early-launch firehose. |
| DEX Screener REST | https://docs.dexscreener.com/api/reference | Profiles, latest boosts, community takeovers; search, pairs, tokens and token pools. | Public documented 60 req/min for promo endpoints, 300 req/min pair endpoints; cached/vendor context, not landed executable price. |
| DEX Screener WebSockets | https://docs.dexscreener.com/api/websockets | Six documented feeds: latest/recent profiles, takeovers, ads, latest/top boosts. | Promotion/attention discovery; NO documented trade-by-trade WebSocket in this reference. |
| Jupiter Tokens V2 | https://github.com/jup-ag/docs/blob/main/tokens/index.mdx | Recent tokens, search, trending/top, token program, organic scores, holders/liquidity/audit flags. | Discovery and risk hints; verified/organic is not an on-chain security guarantee. |
| Jupiter swaps | https://dev.jup.ag/docs/swap | Quotes and routes for modeling possible trades and sell paths. | Quote is not a fill; no transaction broadcasts authorized. |
| Birdeye Data | https://birdeye.so/data-api/product/streaming | WebSocket new pairs, live trades, prices, wallet and whale activity. | Strong candidate source, plan-gated; operator must authorize paid use. |
| Bitquery GraphQL/gRPC | https://docs.bitquery.io/docs/blockchain/Solana/ ; https://docs.bitquery.io/docs/grpc/solana/topics/dextrades/ | DEXPools/new pools, Pump.fun events, trades, wallets, creator and token balances. | Requires credential/plan; realtime and archive are different coverage products. |
| CoinGecko / GeckoTerminal | https://docs.coingecko.com/websocket/wss-onchain-trade ; https://docs.coingecko.com/websocket/wssonchainohlcv | Per-pool trade and OHLCV streams; new pool/pair discovery from REST as available. | WSS may be subscription-gated; null values and lag require explicit abstention. |
| Shyft Solana DeFi | https://docs.shyft.to/solana-defi-apis/defi-apis | Indexed pools across Raydium, PumpSwap, Orca, Meteora and LaunchLab. | Cross-check/secondary discovery; not canonical reserve evidence. |
| GoPlus Solana | https://docs.gopluslabs.io/reference/solanatokensecurityusingget | Solana token security and risk API (beta). | Defense-in-depth; UNKNOWN if no report, not proof of no honeypot. |
| RugCheck | https://api.rugcheck.xyz/swagger/index.html ; https://fluxrpc.com/docs/rugcheck/getTokenReportSummary | Token report, mint/freeze, liquidity and risks. | Swagger found; full official endpoints/terms not independently verified here. Third-party docs are context, require dedicated probe. |
| Helius LaserStream | https://www.helius.dev/laserstream | Yellowstone-compatible streams/replay, alternative chain-data transport. | Paid service; marketing pages currently differ about 24h vs 48h replay, verify provider contract. |

Further candidates: verified venue-native on-chain SDK/IDL support before adding scraping or undocumented browser endpoints. Measure **incremental** coverage rather than counting copies of the same event across correlated APIs.

## Reach measurement and data causality

Split discovery by lifecycle: **launch curve** (Pump.fun CreateV2 / Raydium LaunchLab / Meteora DBC) → **graduation** (PumpSwap/Raydium/Meteora DAMM) → **secondary pools** (Orca/Raydium/etc.) → **attention and smart-money** (screener boosts, Jupiter trend, Birdeye/Bitquery wallet feeds). A pool-based screener may miss launches BEFORE their first DEX pool or swap; first-party on-chain is the recall denominator.

Normalize each candidate to (genesis hash, mint, venue, pool) with provider ID, schema version, raw event hash, chain slot + commitment, event chain time, API received time, our known-at time, TTL, gap state and evidence class. Account/fee state is looked up as of decision slot; later historical API responses cannot masquerade as earlier knowledge. Preserve raw snapshots in bounded journals; expose hash-sealed receipts and source-specific exclusion reasons.

Metrics: unique on-chain-verified candidates/hour, verified recall vs complete block census, provider novelty, first-seen lag distribution, false positives, security veto rates, source outage and gaps, cost per useful candidate, unpriceable or unexitable rate. Count filtered-out candidates to prevent survivorship bias.

## Universal deterministic filter contract

Filter output is versioned ALLOW_PAPER, WATCH, BLOCK plus reason codes, evidence hashes, expiration and policy hash. A pure function on sealed inputs, never a network call or a model answer. Mandatory data unavailable = WATCH/BLOCK; only optional enrichment may be missing with explicit lower confidence. Disagreement between vendors requires chain verification. Existing positions retain separate authorized risk-reducing exit path when new entries are halted.

| Gate | Purpose | Mandatory checks and failure |
|---|---|---|
| G1 | Source and causality | Genesis, mint owner, program ID, timestamp known at decision, schema, payload size, signature, commitment, stale/poisoned feed |
| G2 | Token permissions | SPL/Token-2022: mint/freeze, TransferHook + external hook program, PermanentDelegate, transfer fee, default frozen, non-transferable, unknown extensions |
| G3 | Actor and provenance | Creator launch patterns, funding-linked holders, concentrated insiders, wash/bot buying, unknown cost basis |
| G4 | Liquidity and exit | As-of reserves, real vault ownership, liquidity floor, buy AND sell feasibility, fee+slippage-adjusted exit, unsupported quote |
| G5 | Genuine market demand | Organic unique wallets, bot-adjusted trade flow, multi-pool consistency; boosts/ads never sole entry evidence |
| G6 | Real execution economics | Observed fee tiers, Token-2022 fee, priority fee, failed-tx burn, slippage, latency/no-fill; stress survivability |
| G7 | Portfolio/risk authorization | RiskEngine approved order, caps by mint/creator/wallet, exposures, drawdown, cooldown, duplicates, kill switch |

**Universal application:** source admission before ingestion; token gate before paper signal; wallet funding/basis gate before leader alpha; venue adapter fixture/IDL gate before deployment; research dataset causal gate before H8; strategy H8 preregistration/critic gate before promotion; Harness² worker sandbox and proposed-code gate; dashboard artifact hash gate. The model cannot bypass a BLOCK.

Security threat matrix: Token-2022 hooks may execute custom logic, delegates can move/burn balances, transfer fees can alter settlement, unknown restrictions can block sells; developer/wallet drain, fake pools, manipulated volume, metadata injection/SSRF, malicious transaction accounts, MEV/sandwich/failure storms, reorg, RPC provider equivocation, corrupted checkpoint, partial disk, unsanitized secrets and unsafe agent tooling. Prove negative fixtures for each. Official sources: https://solana.com/docs/tokens/extensions/transfer-hook ; https://solana.com/docs/tokens/extensions/permanent-delegate ; https://solana.com/docs/tokens/extensions/transfer-fees ; https://solana.com/docs/tokens/extensions/non-transferrable-tokens .

## 24/7 paper architecture

A bounded supervised Rust service runs independent of all AI APIs: provider adapters → schema/provenance admission → durable append-only journal → canonical event/market book → G1–G7 → paper capability → paper broker → independent PF1 oracle → immutable receipts → read-only dashboard. Use states BOOT, CATCHUP, OBSERVE, PAPER_READY, DEGRADED_NO_NEW_ENTRY, RECONCILE, STOP. Restart must be idempotent; gap backfills, signature/slot dedup and chain commit upgrades are deterministic. Stale fees, missing reserves or critical gap must halt new entries. Existing paper positions still get safe evaluation/exits where valid data remains.

A 24-hour fault-injected no-AI soak is an OPERATIONS goal, not evidence of the ≥7-day truly prospective SHADOW research gate. systemd or equivalent requires least privilege, limits, watchdog, disk quotas, killed-process recovery, replay hash equality and no signer imports.

## Google Harness² and safe self-improvement

Real pinned Google Harness²: https://github.com/google-research/harness2/blob/main/docs/EXPERIMENTS.md . It proposes/probes/composes improvements in **research workflow**. The existing deterministic channel is functional; model-backed execution remains blocked pending separate truly isolated keyless worker per ADR-0012. Never put LLM calls into runtime risk/decision path; never give a model RPC or wallet secrets. H8 preregistration/one-use holdout, PF1 independently computed NAV and G-A–G-G alone decide if a strategy wins.

If investigating reinforcement learning, use offline, constrained contextual bandit or batch RL only on causally valid, action/propensity-labelled data with held-out creators/days. Reward = risk-adjusted, actually realizable after-cost net P&L; actions include ABSTAIN and bounded PAPER EXIT. Reject optimistic off-policy extrapolation where there is no action support. No autonomous funded exploration, no unsupervised production policy updates or “run until green” benchmark peeking.

## Strategy thesis and critical path

Prioritize D2 as-of reserve/fee snapshots on the 232 launches → PR-2 historical real-cohort Engine A/B/PF1 independent NAV (including all rejected/unexitable fills) → G1–G7 security filters → multi-source/venue reach → deterministic 24/7 paper → S11 organic selection + EX safe exit → S12 graduation, S13 multi-day survivor and S14 provenance follower. S11 cannot use eventual 6–24h curve-fill as a feature at time t < 6h. W01 naive follower remains rejected. All variants count for H8, S00 remains bar, all fees and failed burns included, clustered CI/CVaR95/stress/one-use holdout/≥7-day shadow/human paper promotion required.

## Evidence limits

This is a verified documentation and repo audit, NOT live API probe, measured throughput, new implementation, real trade, 24/7 soak or profit result. Source plans, rate limits and APIs change. Do not claim any third-party API is currently reachable or free without a measured probe.
