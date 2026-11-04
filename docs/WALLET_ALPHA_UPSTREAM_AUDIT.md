# SniperFusion — audited upstream and wallet tracker integration map

**Reviewed:** 2026-11-04; SniperFusion base `main@3dd5693697dfa630d291074ec3de28bccc8276ca`. This is a **code-source audit and implementation plan**. We inspected public pinned snapshots by GitHub API, not the user's current, private working-tree contents. Check local `git -C upstream/<name> rev-parse HEAD` before assuming exact parity.

## Clarifying the five local folders

The user calls the second repository "Mapbot"; the project's existing upstream manifest names `upstream/mev-bot`. Do not silently treat any other "mapbot" as that same source. The five actual entries in the committed manifest are **chainstack**, **mev-bot**, **yuto**, **m8s**, and **pump-docs**. `pump-docs` is official program documentation/IDL, **not a trading bot**.

| Source (pinned SHA) | Verified source behavior | SniperFusion gap / use | Caveats |
| --- | --- | --- | --- |
| [Chainstack Labs Pump.fun/BONK bot](https://github.com/chainstacklabs/pumpfun-bonkfun-bot) `9e22464e` | Python logs/blocks/Yellowstone/SubscribeDeshred listeners; Pump.fun/PumpSwap and LaunchLab/LetsBonk/StonkFun; quote handling, fee manager, TP/SL/time exits and offline regression | Keep as differential behavioral baseline, reference for cross-venue lifecycle and cleanup. Benchmark listeners; migrate missing useful behaviors into current Rust interfaces | **Apache-2.0**; retain attribution; codebase is not a profitability model; old V2/IDL assumptions must not override official newer V3 |
| [Yuto Kazuma Rust sniper](https://github.com/yuto-kazuma/solana-sniper-bot) `b1af7e10` | Rust/Tokio, wallet follow, Yellowstone, multiple DEX source files, Jupiter/ZeroSlot, trailing exits; only three DEX adapters actually exported in `src/dex/mod.rs` | Independently design tracked-wallet subscription, leader-buy/sell signal, latency/exit control; venue-specific adapters only after verified parity | **No explicit standard license**; source-file count is not functional coverage. `risk_management.rs` polls target wallet balances every **10 minutes** by default, inadequate for second-scale risk |
| [m8s-lab Solana sniping](https://github.com/m8s-lab/solana-sniping-bot) `2a0e3bf5` | New-launch and migration event handlers; metadata links, holder and vault snapshots, rudimentary social metrics | Add creator net-buy, concentration, migration and coverage-independent risk features to Rust, all as-of decision time | **No explicit license text** despite package.json MIT claim. `check-metadata.ts` is primarily URL/X blacklist, **not honeypot detection**; `handle-create.ts` delays 15s and contains a TODO for actual buy |
| [bigmacman MEV bot](https://github.com/bigmacman1129/solana-sniper-trading-mev-bot) `829f251d` | TS Jito/RPC/Warp transmit adapters, holder/burn/mutable filters, in-memory positions, simple consecutive-failure circuit breaker | Independent multi-route status and transaction recovery, entry throttling/exit prioritization, once routing actually needed | **Microsoft Public License (Ms-PL)**, conflicting README claims MIT. Simple circuit breaker does not address independent position/gap/reorg failures; do not import mechanically |
| [Pump.fun public docs/IDLs](https://github.com/pump-fun/pump-public-docs) `639b87fb` | Current official schema reference: curve V2/V3, PumpSwap V2, fee_config, CPI, Token-2022 variants and migrations | Mandatory oracle for program version, PDA, accounts, fees, sell feasibility and fork state | Documentation may lag actual deployments. Record observed fee config/program hashes at each relevant slot and provenance. Not an open-source sniper to run |

**Current result**: SniperFusion already has Rust builders, Surfpool M3c evidence, fee-config reads, P&L V2, H8 trial registry and G0 limited COMPLETE windows. A second wholesale migration would be counterproductive; integrate capabilities through typed adapters and independently measured tests.

## Open-source wallet trackers: popularity is not audit quality

Snapshot data from GitHub API on 2026-11-04; stars/forks are **public engagement**, not active users, live reliability, profitability or code safety. This is a candidate comparison, **not proof of "most used"**.

| Repository | Signals / code actually inspected | Popularity at review | License at review | Decision |
| --- | --- | --- | --- | --- |
| [Handi Cat wallet tracker](https://github.com/DracoR22/handi-cat_wallet-tracker) `f154c670` | TS Telegram add/delete wallet, onLogs subscriptions, Raydium/Jupiter/Pump events, token price and alerts, Prisma persistence | **207 stars / 79 forks**, most starred wallet *alert tracker among the candidates examined* | **No LICENSE located** | Observe UI/alert features **only**; do not copy code. Alerts aren't full-accounting proof or subsecond profitable trades |
| [Kelows/million](https://github.com/Kelows/million) `c89ad07c` | MIT Node/NestJS discovery token→buyer→wallet graph, bot/venue exclusion, wallet analysis, token screener, paper/shadow, backtester, **copyability.service.ts** | **12 stars / 1 fork**; 308 commits reported by GitHub web snapshot | **MIT** | **Primary licensed wallet-alpha reference**; evaluate selected modules/fixtures and reimplement Rust hot path. NOT a drop-in millisecond engine |
| [DoradoDevs wallet P&L tracker](https://github.com/DoradoDevs/solana-wallet-pnl-tracker) `1aaeda86` | MIT balance-delta DEX parser, paginated Helius history, FIFO cost basis, checkpointed live monitoring; Jupiter/Raydium/Pump/Meteora/Orca support in source | **1 star / 0 forks** | **MIT** | Independent cross-DEX accounting/parsing reference and negative-control oracle. Estimates need reconciliation |
| [justFiveDev Rust Copy-Trading-Bot](https://github.com/justFiveDev/Solana-Copy-Trading-Bot) `d7335e5` | Rust Helius-style transactionSubscribe, Raydium and Pump.fun wallet-follow, Jito; Jupiter stub | **52 stars / 105 forks** | **No LICENSE found** | Architecture design reference only; Solana 1.16 dependency assumptions and funded auto-trading are inappropriate for SniperFusion research runtime |
| [Solsonar multi-DEX decoder](https://github.com/solsonar/solana-multi-dex-decoder) `8ae4c238` | MIT zero-RPC account / swap decoder for Raydium, Pump AMM, Meteora DLMM and Orca; includes limitations for Raydium AMM direction | **0 stars / 0 forks** | **MIT** | Good low-level differential fixtures for Rust balance-delta attribution; not a wallet P&L tracker |
| [anchoys1337 wallet behavior analysis](https://github.com/anchoys1337/solana-wallet-behavior-analysis) `3e65c87e` | MIT public-chain trading reconstruction, follower simulation and risk/freshness tests | **3 stars / 1 fork** | **MIT** | Research methodology/control sample; audit all assumptions before reuse |

**Recommendation:** combine **million's wallet/token discovery and copyability research**, **Dorado's cross-venue transaction/balance normalization**, and **existing SniperFusion's Rust engine and independent P&L oracle**. Handi Cat is more visible in this small public sample but its missing license and Telegram focus make it an inferior source-code foundation.

### Specific source-level pitfalls

- `million/apps/api/src/copyability/copyability.service.ts`: uses **one fixed 60-second delayed minute candle** and only **six recent closed trades per wallet** as its bounded measurement; this is not a millisecond follower backtest, and sample truncation creates selection risk. Replace with actual observed transaction and reserve updates, multiple delays and all holdings.
- `million/apps/api/src/discovery/discovery.service.ts`: excludes pool/router infrastructure buyers and warns that recent-token discovery is recent-window biased. Borrow the **idea** but identify traders from account ownership, balance deltas and repeated activity; record missing-pool attribution.
- `million/apps/api/src/screener/token-check.service.ts`: combines authority verification, holder distribution, Rugcheck and Jupiter sell simulations; useful for enrichments, but remote scoring cannot replace deterministic Solana/Token-2022 checks.
- `DoradoDevs/apps/server/src/indexer/parsers/balance-delta.ts`: use independent balance-delta reconciliation with router/aggregator exclusions; some swaps are multi-hop and should be one net economic trade despite many pool legs.
- No public tracker can **discover a reliably profitable trader in milliseconds from scratch**. Discovery/label validation/backfill takes minutes or longer. A **pre-indexed monitored wallet roster** can trigger subsecond alerts with Yellowstone; measure event-to-observation and candidate-evaluation p50/p95/p99 on actual provider feeds.
- Hype/influencer attribution is inherently probabilistic: a claimed "Murad wallet" is not proof of his personal control, promotion or current holdings. Transfers are not buys. Treat KOL labels as opt-in/publicly sourced tags with source, confidence and timestamp, not as authorization or direct buy triggers.

## Token quality model: distinguish safety, opportunity and exit liquidity

**Technical policy (hard BLOCK/WATCH/ELIGIBLE):** mint/freeze authority; correct SPL/Token-2022 owner; permanent delegate; transfer hook; transfer fees; pausable/non-transferable extensions; protocol/quote asset and IDL version; complete observation; ability to simulate a sell; max order/tip/priority and stale data. An UNKNOWN risk is not safe.

**Creator/cluster risk (reasoned uncertainty):** developer initial buys net of rent, taxes/fees and vault movements; creator sell pressure relative to *executable exit depth*; top holder effective concentration excluding protocol vaults; linked-wallet/funding graph; same-slot trades (not all same-slot activity is collusion); seller withdrawal behavior. Cluster and reputation indicators should have uncertainty/provenance, not deterministic guilt.

**Opportunity quality (never overrides hard policy):** buyer breadth of independently controlled wallets, repeated organic buys, real volume/flow, momentum after executable cost, migration survival, whale signal relevance, risk-adjusted expected return vs no trade. Narrative/engagement as slowly varying research features only; no unverified social scraping in hot path.

**Economic hurdle:** per-position expected follower gains must exceed **both entry/exit protocol+creator+LP fees** (possibly market-cap tiered), signature+priority/tips, slippage, adverse selection, no-fill costs, and future sell impact. Feed on-chain observed fee configuration and reserve-state timestamps from PF0/PF1; fail closed if unobserved. A wallet with 100 SOL *leader* P&L may have **negative follower P&L**.

## References and provenance

GitHub snapshot SHAs above; verify source/commit/LICENCE before implementing. Official live references:
- https://solana.com/docs/payments/accept-payments/indexing
- https://solana.com/docs/tokens/extensions/permanent-delegate
- https://solana.com/docs/tokens/extensions/transfer-hook
- https://github.com/pump-fun/pump-public-docs
- https://github.com/sevenlabs-hq/carbon
- https://github.com/solana-foundation/surfpool

No third-party repo was cloned into the user's local working directory or executed as part of this audit. Respect the root `AGENTS.md` clean-room restrictions.