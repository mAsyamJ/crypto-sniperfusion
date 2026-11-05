# Engineering Design Review: Building a High-Performance Solana Memecoin Sniper

**Status:** POINT-IN-TIME (2026-11-02) — original design review; moved from root `research.md` on 2026-11-05 (see `docs/DOC_INDEX.md`). Current state lives in `docs/PROJECT_STATUS.md`; this file is history, not a capability claim.

Research date: November 2, 2026 · Architecture: Solana / Pump.fun / PumpSwap / Raydium / Meteora · Python + Rust

## Executive engineering decision

Keep Chainstack Labs as the baseline, but build a dedicated Rust execution engine around its proven Solana protocol implementation. Do not blindly merge all four repositories.

After inspecting the repository structures and implementation files, I've identified several important differences between what these projects advertise and what their code actually implements.

Proposed architecture: Chainstack Sniper Fusion

A working name for the proposed system, not an existing product.

Baseline

Chainstack Python

Hot path

Rust / Tokio

Market detection

Multi-DEX Geyser

Execution

Direct + Jito

Token analysis

On-chain risk engine

Position management

Event-driven exits

The priority is not just making transactions faster. It is maximizing the probability of a profitable, correctly executed trade while rejecting dangerous tokens.

A fast bot buying an obvious developer dump is simply a faster way to lose capital.

## 1. What the four repositories actually contribute

| Repository                                                                                                     | Engineering contribution                                                               | Integration decision                     |
| -------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- | ---------------------------------------- |
| [Chainstack Labs](https://github.com/chainstacklabs/pumpfun-bonkfun-bot?utm_source=chatgpt.com)                | Protocol decoding, bonding-curve trading, listeners, exits, regression tests           | Keep as reference baseline               |
| [Yuto Kazuma](https://github.com/yuto-kazuma/solana-sniper-bot?utm_source=chatgpt.com)                         | Rust/Tokio, multi-DEX adapters, ZeroSlot, Jupiter, copy trading, exit strategies       | Adapt selected designs                   |
| [m8s-lab](https://github.com/m8s-lab/solana-sniping-bot?utm_source=chatgpt.com)                                | Launch-event parsing, creator-buy estimation, metadata/social checks, migration events | Reimplement with stronger validation     |
| [Solana Sniper MEV Bot](https://github.com/bigmacman1129/solana-sniper-trading-mev-bot?utm_source=chatgpt.com) | Jito execution, holder filters, position locks, circuit breakers                       | Extract patterns, not the entire runtime |

Sources: inspected repository trees and selected source files, including Chainstack's `universal_trader.py`, Yuto's `sniper_bot.rs` and `zeroslot.rs`, m8s-lab's `check-metadata.ts` and event handlers, and bigmacman's transaction executors and circuit breakers.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+3



### Critical discoveries from the source code

1\. m8s-lab does not implement the comprehensive honeypot detector you're expecting.

Its `src/checker/check-metadata.ts` primarily retrieves JSON metadata, extracts X/Twitter and website links, and checks social handles against a blacklist. It does not establish whether a token is safe to sell, whether developers control multiple wallets, or whether liquidity will disappear.

Its `handle-create.ts` also contains a commented-out buy section, while `handle-migrate.ts` contains an unfinished sell section. This is a useful research reference but not a complete production-grade automated trading engine.

2\. Yuto's Rust implementation is useful, but shouldn't become your foundation unchanged.

It contains actual Rust modules for transaction processing, DEX adapters, Jupiter, ZeroSlot, and selling strategies. However, one risk-management module defaults to checking copied wallets' token balances every ten minutes. That is not an adequate real-time defense against a memecoin developer dumping immediately after launch.

Its Jupiter implementation also references an older Swap API endpoint. Any migration must be validated against current Jupiter integration requirements.

3\. The MEV bot contains useful protections, but its circuit breaker is limited.

The inspected `risk/circuit-breaker.ts` tracks consecutive failures and temporarily pauses trading. That protects against repeated operational failures, but it does not itself enforce portfolio drawdown limits, abnormal developer selling, stale market data, or unsafe token authorities.

Its Jito implementation submits a tip transaction and a swap transaction as a bundle. An accepted submission is not proof of landing, so the new system must reconcile transaction signatures and actual token balances before modifying positions.

4\. Chainstack already has a highly optimized buying mode.

Its `extreme_fast_mode` can submit a buy with no additional RPC read after a sufficiently detailed launch event. However, this sacrifices price discovery and can increase financial risk.

Chainstack also explicitly labels the project educational rather than production-ready.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub



Conclusion: We should preserve Chainstack's protocol knowledge, not its entire execution architecture. The new system needs its own risk gates, event model, transaction state machine and benchmarks.

## 2. Major protocol compatibility changes — November 2026

Before implementing Rust acceleration, we must update the protocol layer. This is more urgent than the language migration.

Pump.fun V3 instructions are now available

Pump.fun documents `buy_v3`, `buy_exact_quote_in_v3`, and `sell_v3`, which use 17 accounts rather than the larger V2 account lists. They maintain the same trade pricing and fees but retain most fees on the curve until settlement.

V3 does not support legacy cashback coins, so the adapter must retain a V2 fallback.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub



Jito's original ShredStream service has been retired

Jito's documentation announced a September 5, 2026 shutdown and recommends migration to DoubleZero Edge.

Chainstack's `UniversalShredsListener`, however, uses a Geyser `SubscribeDeshred` interface. That is a distinct integration path whose availability must be checked with the actual RPC provider, rather than assuming all shred-based detection has disappeared.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://docs.jito.wtf\&sz=32)

High Performance Solana Infrastructure



Jupiter's older integrations require migration

Jupiter now recommends Swap API V2 at `api.jup.ag/swap/v2`, using `/order` for managed execution or `/build` when assembling your own transactions. Its legacy Metis Swap V1 is deprecated, and the old `lite-api.jup.ag` host was scheduled to shut down October 30, 2026.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+1



Existing MEV bot assumptions are outdated

The bigmacman repository states that Pump.fun graduation moves into Raydium and assumes a 1% protocol fee. Neither is a safe basis for today's Pump.fun architecture: canonical migrations use PumpSwap, and Pump.fun's published bonding-curve total fee is 1.25%.

Its README also claims an MIT license, while the actual `LICENSE.md` contains the Microsoft Public License. Do not copy this repository wholesale without resolving the inconsistency.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+2



One additional distinction: ZeroSlot is a transaction-landing service, not zero-slippage execution. No relay can guarantee you enter a volatile memecoin at the observed price. Slippage tolerance is a risk limit, not a speed feature.

## 3. The target architecture

Target architecture. This is a proposed design, not a deployed or benchmarked system.

### Split the system into two planes

Data and control plane: configuration, historical analysis, wallet intelligence, backtesting, Telegram alerts, reporting, dashboards, research jobs, and strategy experiments. Python is perfectly suitable here.

Latency-critical trading plane: Solana event decoding, in-memory state, deterministic risk checks, pricing math, transaction construction, key signing, submission, and fill reconciliation. Implement these in Rust once the behavior is validated.

This separation means Python jobs or external APIs cannot block the live trading loop.

The transaction engine should have no runtime dependence on MongoDB, Postgres, social-media requests, or an LLM. If those services become unavailable, active positions must remain manageable.

## 4. Token Intelligence Engine — the most important subsystem

Your central requirement is to know whether a token is sniperable before spending money.

That is not a binary on-chain fact. We can reliably detect certain technical hazards, estimate manipulation risk, and classify trading conditions. We cannot conclusively predict whether a developer will dump or whether a price will rise.

I would implement three distinct verdicts.

ELIGIBLE

All mandatory safety checks passed, adequate market data exists, and the strategy currently has a valid trading signal.

WATCH

No definitive malicious condition, but insufficient evidence, stale state, unusual holder activity, or poor risk-adjusted execution.

BLOCK

Critical technical risk, unsupported trade, excessive concentration, unacceptable execution cost, or portfolio risk limit breached.

### The screening pipeline

| Check                    | What to inspect                                                              | Response                                 |
| ------------------------ | ---------------------------------------------------------------------------- | ---------------------------------------- |
| Program verification     | Actual executable program IDs, mint program, pool and account ownership      | Block unexpected programs                |
| Mint / freeze authority  | Ability to mint more supply or freeze accounts                               | Block unsafe, unexpected authority       |
| Token-2022 extensions    | Permanent delegate, transfer hook, transfer fees, pause/control capabilities | Deny unsafe or unsupported extensions    |
| Creator initial buy      | Creator purchase in the launch transaction                                   | Measure; don't assume zero               |
| Developer holdings       | Creator and suspected associated wallet balances, excluding pool custody     | Block extreme concentration              |
| Top-holder concentration | Top 10/20 beneficial holders, excluding known protocol vaults                | Risk-adjusted threshold                  |
| Developer history        | Prior launches, rapid sells, repeated funding relationships                  | Strong negative signal                   |
| Curve / liquidity        | Available reserves, effective quote reserves, exit depth, price impact       | Reject insufficient exit liquidity       |
| Launch trading activity  | Unique buyers, repeated bundled buyers, concentration of order flow          | Detect suspicious demand                 |
| Metadata                 | URI, social references, name, symbol, image, duplication                     | Reputation signal only                   |
| Sell feasibility         | Required sell instruction, token transfer permissions, current pool state    | Block if not demonstrably executable     |
| Transaction economics    | Estimated entry/exit costs, fees, slippage and tip                           | Reject unprofitable execution conditions |

Solana's Token-2022 extensions can introduce powerful permissions, including permanent delegates that may transfer or burn user tokens, and transfer hooks that execute additional program logic during transfers. These need to be decoded from on-chain state, not inferred from token names.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://solana.com\&sz=32)

solana.com+2



For canonical Pump.fun-created tokens, the checks must be protocol-aware. A legitimate program-owned mint authority or liquidity vault should not automatically be mistaken for a malicious developer wallet.

### How to measure the developer's initial purchase

This is the feature I would adapt from m8s-lab, but implement more carefully.

When a new coin is created:

1. Extract the creator identity, mint address, bonding-curve address and transaction signature from the validated launch event.
2. Inspect the creation transaction's buy instructions, including relevant inner instructions.
3. Calculate the amount of quote tokens actually spent on the creator's purchase, separating trading fees, rent and unrelated SOL transfers.
4. Calculate the resulting token ownership and compare it against supply outside protocol-controlled custody.
5. Track further buys, transfers and sells from the creator and suspected related wallets.

The m8s-lab source attempts to estimate an initial buy using differences in transaction balances and a fixed deduction. That can be useful as a prototype heuristic but is not precise enough for production.

Example: If the developer initially purchases 8% of independently held tokens, then transfers most of them to four other wallets, a simple top-holder check may incorrectly report that no single wallet is dangerous.

We therefore need wallet clustering, but the bot must distinguish verified connections from heuristic guesses.

### Detecting developer dumps

I would create an ongoing `DevFlowMonitor`, not only a launch-time filter.

It would track creator-linked holdings, transfers, and estimated net selling pressure, then trigger warnings or protective exits when the risk increases.

One important detail: a creator selling 0.5 SOL is not inherently suspicious. Selling 0.5 SOL into a pool with very little available liquidity can be devastating. The engine should measure selling pressure relative to executable liquidity, not simply SOL volume.

### Honeypot detection limitations

Ethereum-style honeypot checks are not directly transferable to Solana. On Solana, a token may fail to sell because of token permissions, malicious transfer-hook behavior, an unsupported token program, missing accounts, invalid routes or insufficient liquidity.

Even a successful simulated sell only establishes that the transaction works against the simulated state. It cannot guarantee a future sell after the creator changes market conditions.

For secondary verification, integrate [GoPlus Solana Token Security](https://docs.gopluslabs.io/reference/solanatokensecurityusingget?utm_source=chatgpt.com) or RugCheck as asynchronous enrichment—not as a mandatory external HTTP request on every microsecond-sensitive entry. Solana's documentation specifically recognizes RugCheck's analysis of authorities, holder distributions and liquidity.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://docs.gopluslabs.io\&sz=32)

docs.gopluslabs.io+1



### Candidate score design

Prototype Risk Evaluator

Illustrative scoring logic — not trained or calibrated

ELIGIBLE\*

Risk-adjusted score

# 76 / 100

Higher is better

Creator-linked holdings

8%

Top-holder concentration

20%

Creator history

CleanUnknownRug history

Token authorities / extensions

Expected

Unsafe

Executable exit liquidity

Adequate

Low liquidity

No valid sell route

Screening passes in this simplified example, subject to all other mandatory checks.

\*Eligible does not mean safe or profitable. The score excludes market signals, price impact and transaction confirmation, and these weights are placeholders. Real thresholds require historical calibration.

The actual production scorer should output the raw facts, evidence slots, confidence, and rejection reasons. A score of 80 alone is not meaningful without a calibrated historical relationship to trading outcomes.

## 5. Rust execution engine: the technical specification

I would design the Rust service as five independently testable modules, running within one process initially to avoid unnecessary network hops.

| Rust module    | Responsibilities                                                         | Source of ideas             |
| -------------- | ------------------------------------------------------------------------ | --------------------------- |
| `ingest`       | Yellowstone streams, wallet monitoring, event decoding and deduplication | Chainstack + Yuto           |
| `intelligence` | Program verification, risk checks, developer-flow tracking               | m8s + custom                |
| `strategy`     | Launch entry, migration entry, copy-trading decisions                    | Yuto + custom               |
| `execution`    | Quotes, instruction building, signing, Jito/direct submission            | Chainstack + Yuto + MEV bot |
| `positions`    | Transaction reconciliation, trailing exits, TP/SL, position limits       | Chainstack + Yuto + MEV bot |

### Recommended Rust stack

| Layer               | Choice                                                                        |
| ------------------- | ----------------------------------------------------------------------------- |
| Language            | Stable Rust                                                                   |
| Runtime             | Tokio                                                                         |
| Streaming           | Yellowstone gRPC with Tonic-compatible clients                                |
| Solana transactions | Version-compatible Solana Rust SDK                                            |
| Pump.fun protocol   | `pump-rust-client` as a candidate, audited against canonical IDLs             |
| Concurrency         | Tokio tasks, bounded channels, sharded in-memory maps                         |
| HTTP client         | Reqwest with pooled connections                                               |
| State persistence   | PostgreSQL for audit/positions, optional Redis or Valkey for supporting state |
| Observability       | `tracing`, Prometheus and Grafana                                             |
| Secrets             | Dedicated restricted signer / encrypted key storage                           |

Pump.fun's current documentation references `pump-rust-client` 0.4.1 alongside its newer Pump SDKs. It includes V3 bonding-curve builders and PumpSwap support, making it a better candidate than translating every Python instruction builder manually.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+1



### A. Make market detection event-driven

The bot should subscribe to:

- Pump.fun token creation and bonding-curve trade activity.
- PumpSwap migration and pool creation.
- Raydium LaunchLab and supported AMM pool events.
- Meteora Dynamic Bonding Curve creation and migration.
- Transactions and balance changes from selected copy-trading wallets.

Each venue needs its own versioned protocol adapter, all producing the same internal event structure.

```

pub enum MarketEvent {
    TokenCreated {
        venue: Venue,
        mint: Pubkey,
        creator: Pubkey,
        slot: u64,
        signature: Signature,
    },
    TradeObserved {
        venue: Venue,
        mint: Pubkey,
        trader: Pubkey,
        slot: u64,
    },
    PoolMigrated {
        mint: Pubkey,
        source: Venue,
        destination: Venue,
        slot: u64,
    },
    WalletActivity {
        wallet: Pubkey,
        mint: Pubkey,
        slot: u64,
    },
}

```

This is an illustrative contract; a production event also needs amounts, event indexes, source identifiers, commitment level and enough transaction context for deterministic deduplication.

A key design decision is using one canonical event identity. The same token creation may arrive through multiple providers. Key events by chain, transaction signature and instruction/event index, not mint alone.

An event seen at `processed` commitment is preliminary. Handle reversions and confirm or finalize the recorded state afterward.

### B. Low-latency execution without unnecessary RPC calls

The execution path should be:

Receive event → decode → risk gate → quote → sign → submit

Everything needed for a fast entry should already be cached or carried by the event.

Specifically:

1. Cache recent blockhashes and static protocol configuration.
2. Precompute immutable program IDs and commonly used PDAs.
3. Keep persistent RPC, gRPC and relay connections.
4. Decode directly from event payloads when sufficient and verified.
5. Use exact integer arithmetic for token amounts, fees and thresholds.
6. Avoid synchronous database writes and external metadata lookups before submission.
7. Bound transaction spend, priority fees, tips and slippage in the signed transaction.

The new PumpSwap protocol has additional important mathematics: its virtual quote reserves may be negative, and Pump.fun supports synthetic migration as the bonding curve is exhausted. Both require correct signed-integer arithmetic and special migration-boundary tests.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub



Do not implement your own curve formulas by copying a 2024 or 2025 example. Compare against the latest supported SDK, current program state and known on-chain transaction fixtures.

### C. Execution router: direct RPC, Jito or Jupiter

The router should decide where an instruction goes based on its requirements.

| Route                                    | When to use                                                   | Trade-off                                                                    |
| ---------------------------------------- | ------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Direct RPC / optimized transaction relay | Standard Pump.fun or AMM swap                                 | Simple and flexible, but exposed to ordinary transaction landing uncertainty |
| Jito `sendTransaction`                   | Low-latency submission with Jito-specific protection          | Requires separate status reconciliation                                      |
| Jito `sendBundle`                        | Atomic ordered groups of transactions                         | Tip auction, possible non-landing, leader-dependent                          |
| Jupiter Swap V2                          | Cross-DEX exits, migration recovery or best available routing | Additional quote/build latency; not ideal for earliest bonding-curve snipes  |

Jito supports atomic bundles of up to five transactions; the bundle must be tracked through its status and ultimately verified against on-chain execution. A bundle ID is not a completed trade.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub



For Jito-routed trades, I would also evaluate its `DontFront` mitigation, which can constrain how transactions are ordered inside Jito bundles. It does not protect against all MEV and is not a general-purpose slippage guarantee.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://solana.com\&sz=32)

Solana



One operational rule is essential: never blindly send a second independent buy because the first submission timed out.

A timeout does not mean a transaction failed. The original transaction may still land.

Use a transaction lifecycle such as:

PLANNED

SIGNED

SUBMITTED

LANDED

Update fill

FAILED

Review retry

UNKNOWN

Reconcile

When a transaction's status is unknown, reconcile its signature, expiry and wallet balances before rebuilding an order. Re-broadcasting the exact same signed transaction is a different operation from creating a fresh transaction that can cause a duplicate buy.

### D. Exit strategies must be more sophisticated

I would implement the following exit capabilities, each configurable independently:

| Exit mechanism      | Trigger                                                   |
| ------------------- | --------------------------------------------------------- |
| Fixed take-profit   | Net expected return exceeds threshold                     |
| Hard stop-loss      | Marked exit value falls below threshold                   |
| Trailing stop       | Price falls from the highest tracked executable valuation |
| Partial exits       | Sell portions of the position at defined targets          |
| Time-based exit     | Position exceeds maximum holding time                     |
| Developer-dump exit | Material creator-linked selling detected                  |
| Liquidity-risk exit | Executable exit depth deteriorates                        |
| Emergency exit      | Critical event or operator command                        |

An exit engine should evaluate the expected sale proceeds, not merely the last displayed token price. In bonding-curve markets, selling the entire position changes the price.

Maintain separate states for `OPEN`, `SELL_PENDING`, `PARTIALLY_EXITED`, `CLOSED` and `RECONCILIATION_REQUIRED`.

The sell path needs the same reliability engineering as the buy path. A bot that buys quickly but cannot exit across a Pump.fun → PumpSwap migration is not ready for real capital.

## 6. Three separate trading strategies

Do not combine all entry conditions into a single score or assume all strategies benefit from identical settings.

Strategy A — Fresh-launch sniper

Detect a newly created coin and purchase shortly after launch.

Highest sensitivity to infrastructure speed, but least historical information about the token, developer and genuine demand.

Build priority: Second. Test initially without automatic spending.

Strategy B — Momentum-qualified / migration sniper

Track creator behavior, buyers, holder concentration, trading volume and bonding-curve progression. Enter only if the token meets a defined momentum and risk profile.

Less dependent on winning the very first transaction race and provides more data to assess developer behavior.

Build priority: First. My recommended initial live strategy.

Strategy C — Selective wallet copy trading

Observe vetted wallets, identify genuine buys, determine your achievable entry price, and decide independently whether copying the transaction still has positive expected value.

Never buy solely because a monitored wallet bought. The wallet may be part of a coordinated launch or preparing to sell.

Build priority: Third, after wallet-profit attribution and position-risk controls.

These should have separate performance reports. Otherwise a successful migration strategy could conceal a systematically losing fresh-launch strategy.

## 7. Migration plan: Python baseline to Rust-native trading

I would implement this through six engineering milestones. Each milestone has a verifiable acceptance gate before moving to the next.

P0 — Fork and establish a reproducible baseline

Fork Chainstack, pin the exact upstream commit, lock dependencies, run its regression tests, and document existing behavior.

Capture real Solana market events without placing trades. Build a reusable event-fixture corpus.

Acceptance: baseline tests pass; all source revisions and protocol versions recorded; no live-trading credentials required.

P1 — Upgrade protocol support and build risk screening

Verify current Pump.fun and PumpSwap instructions, build token-risk checks, measure creator purchases, add holder-concentration analysis, and produce structured trading verdicts.

Keep Chainstack's existing trader disabled by default.

Acceptance: historical fixtures validate calculations and verdicts; unsupported tokens cannot enter the trading path.

P2 — Implement Rust event ingestion

Create a Tokio service with Yellowstone gRPC, typed events, deduplication, reconnection, and reference parity against Chainstack.

Run the Rust listener in shadow mode alongside Python.

Acceptance: equivalent event detection on captured fixtures, correct handling of duplicate and out-of-order events, and reliable stream recovery.

P3 — Migrate execution to Rust

Implement transaction construction, cached state, quote validation, signing, RPC/Jito broadcasting, and signature reconciliation.

Add a persistent order journal and ensure a crash cannot generate duplicate positions.

Acceptance: transaction parity on test fixtures, no duplicate purchases after retries, and correct account balances after every simulated trade.

P4 — Multi-DEX and copy trading

Add Raydium and Meteora adapters individually. Implement Jupiter V2 for supported secondary routes, wallet-following signals and additional exit strategies.

Do not enable an adapter until its pool math and sell path are validated.

Acceptance: migration, buy and sell fixtures for each supported venue, with independent risk caps.

P5 — Production hardening and controlled capital testing

Add a kill switch, persistent position recovery, observability, security review, fault injection and bounded live experiments.

Release strategies individually behind feature flags.

Acceptance: reliable fills and exits, reconciled P&L, acceptable drawdowns, and demonstrated positive net expectancy before increasing capital.

### Proposed repository structure

```

solana-sniper-fusion/
├── upstream/
│   └── chainstack/          # Reference implementation
├── crates/
│   ├── sniper-core/         # Events, types, config
│   ├── sniper-ingest/       # Yellowstone, wallet streams
│   ├── sniper-protocols/    # Pump, Raydium, Meteora
│   ├── sniper-risk/         # Token and developer analysis
│   ├── sniper-strategy/     # Entry decision engines
│   ├── sniper-execution/    # Quotes, signer, relays
│   ├── sniper-positions/    # Exits, reconciliation
│   └── sniper-app/          # Runtime binary
├── research/
│   ├── backtesting/         # Python analytics
│   └── creator-clustering/
├── fixtures/
│   ├── pumpfun/
│   ├── pumpswap/
│   ├── raydium/
│   └── meteora/
├── config/
│   ├── research.toml
│   └── production.toml
├── dashboards/
├── migrations/
└── Cargo.toml

```

The `upstream/` tree is a pinned reference, not a dependency that the final Rust binary needs at runtime.

Also account for repository licensing: Chainstack has an Apache 2.0 license; Yuto's README permits personal and educational use rather than offering a conventional open-source grant; m8s-lab did not expose a license in the inspected tree; and bigmacman's README conflicts with its actual Microsoft Public License file. Reimplement designs where necessary rather than indiscriminately copying code.

## 8. Performance engineering: prove that Rust is faster

Rust can improve CPU-bound decoding, allocation behavior, concurrent state management and transaction assembly. But the largest delays in a Solana sniper frequently come from upstream event delivery, geographic network latency, transaction scheduling and inclusion.

I would measure these timestamps for every candidate token:

| Stage      | Measurement                                         |
| ---------- | --------------------------------------------------- |
| Detection  | Event receipt relative to observed chain event      |
| Decode     | Event receipt → normalized token                    |
| Screening  | Normalized token → final decision                   |
| Build      | Decision → signed transaction                       |
| Submission | Signed transaction → provider acknowledgment        |
| Landing    | Submission → first valid on-chain observation       |
| Completion | Landing → confirmed fill and balance reconciliation |

The meaningful system metric is event-to-landed-trade latency, not merely how many microseconds the parser takes.

### Performance acceptance targets

These are proposed engineering objectives—not measured performance claims.

| Metric                                    | Initial engineering objective                |
| ----------------------------------------- | -------------------------------------------- |
| Cached event decoding, p99                | Below 10 ms                                  |
| Local deterministic risk evaluation, p99  | Below 10 ms                                  |
| Cached decision → signed transaction, p99 | Below 50 ms                                  |
| Duplicate buys caused by timeouts         | Zero in fault-injection tests                |
| Unreconciled transactions                 | Zero at the end of the reconciliation window |
| Unintended buys after kill switch         | Zero                                         |

Network delivery and inclusion require separately measured baselines. A p99 local-processing objective is meaningless if the provider delivers an event hundreds of milliseconds late.

I would benchmark multiple infrastructure regions rather than automatically placing your VPS in Singapore or Europe. The optimal region depends on actual provider ingress, relay paths and leader connectivity.

A faster submission that lands less often or pays excessive tips is worse than a slower, profitable execution.

## 9. Infrastructure costs and capital efficiency

Chainstack's currently published Yellowstone Geyser pricing starts at $49/month for two streams, but it requires a Growth plan or higher. Its Growth plan starts at $49/month, so the listed entry-level paid combination is approximately $98/month before VPS, trading fees and other services.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://chainstack.com\&sz=32)

From $49/mo for 1 stream+2



| Setup                                        | Indicative monthly infrastructure |
| -------------------------------------------- | --------------------------------- |
| Python research using free-tier RPC          | $0 plus existing compute          |
| Rust shadow listener on free-tier interfaces | $0 plus existing compute          |
| Chainstack Growth + Yellowstone              | \~$98                             |
| Paid streaming + dedicated VPS               | \~$98 plus VPS                    |
| Additional premium execution providers       | Variable                          |

At $100 of trading capital, spending $98 monthly on streaming infrastructure is economically difficult to justify. First establish that the strategy has a positive expected return.

### What profitability must overcome

Pump.fun's October 17, 2026 fee schedule lists a 1.25% bonding-curve trading fee, while canonical PumpSwap fees vary with market capitalization. Direct on-chain trading does not incur the separate Pump.fun web/mobile platform fee.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://pump.fun\&sz=32)

Pump



A rough round trip on the bonding curve may therefore begin with approximately 2.5% in combined percentage trading fees, before execution costs, slippage and price movement.

Consider a hypothetical $20 trade:

| Cost assumption                        | Approximate cost |
| -------------------------------------- | ---------------- |
| Entry and exit platform trading fees   | $0.50            |
| 1% adverse execution movement each way | $0.40            |
| Priority fees and tips combined        | $0.20            |
| Total                                  | $1.10            |

The token would need to appreciate roughly 5.5% relative to the entry reference price just to offset these illustrative costs, with the exact break-even depending on fee application and realized execution.

This is why optimizing for raw sniping volume is dangerous.

### Strategy expectancy

For a simple model:

\\[ E = pW - (1-p)L - C \\]

Where \\(p\\) is win probability, \\(W\\) is average gross win, \\(L\\) is average gross loss, and \\(C\\) is average trading cost expressed as a percentage of the position.

Suppose the bot wins 35% of trades with a 40% gross gain, loses 65% with a 20% gross loss, and incurs 5% average round-trip costs.

\\[ E = 0.35(40\\%) - 0.65(20\\%) - 5\\% = -4\\% \\]

Despite impressive individual winners, the bot loses an expected 4% of traded notional per trade under those assumptions.

We need to optimize risk-adjusted net expectancy, not speed in isolation.

## 10. Required production tests

Before real-money release, I would require automated tests covering these scenarios:

- A normal Pump.fun V2 and V3 buy/sell, including cashback incompatibility and Token-2022 handling.
- A malicious or unsupported transfer authority causing a mandatory entry rejection.
- Creator buying, transferring to related wallets, and subsequently dumping.
- Bonding-curve graduation while an open position is being sold.
- Negative virtual quote reserves and synthetic migration boundary behavior.
- Yellowstone disconnection, duplicated events, missed slots and reordered events.
- A transaction timing out locally but later landing on-chain.
- Two concurrent sell triggers attempting to exit the same position.
- Jito acknowledging a bundle that never actually lands.
- A process restart with unresolved positions, followed by safe state reconstruction.

For operational tests, add injected RPC delays, corrupted event messages, exhausted rate limits and sudden fee spikes.

A proper backtest also needs point-in-time data. Using holder information or creator history that only became known after the simulated trade would introduce look-ahead bias and produce misleadingly high returns.

## 11. My final architecture recommendation

## Recommended implementation

| Foundation     | Chainstack protocol reference and regression fixtures           |
| -------------- | --------------------------------------------------------------- |
| Main engine    | Rust / Tokio with event-driven architecture                     |
| Detection      | Yellowstone gRPC with supported fallbacks                       |
| Protection     | Hard safety checks, developer flow and adaptive position limits |
| Execution      | Direct Pump instructions, Jito and Jupiter V2 fallback          |
| First strategy | Momentum-qualified launch / migration sniper                    |
| Research       | Python backtesting, creator analysis and P&L attribution        |

I would not rewrite Chainstack into Rust in one operation. First verify its protocol behavior against current Solana state, add the necessary screening and fixtures, and then replace the hot path behind stable interfaces. That minimizes the risk of making the system fast but incorrect.

Your original target—combining Chainstack, Yuto's Rust implementation, m8s-lab's launch intelligence and bigmacman's Jito/risk controls—is technically achievable. But it is a new trading system assembled from partially overlapping reference implementations, not a straightforward repository merge.

The first concrete engineering deliverable should be a read-only, multi-DEX market intelligence engine that records and scores every candidate launch. It should produce replayable decisions with exact event timing, developer-buy estimates, holder concentration and simulated net P&L.

Only after those records demonstrate a repeatable advantage should we enable automatic orders and spend money optimizing transaction landing.