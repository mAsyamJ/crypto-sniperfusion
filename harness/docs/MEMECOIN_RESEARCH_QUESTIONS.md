# SniperFusion — Advanced Memecoin Lifecycle & Sniping Research Questions
Date: 2026-11-03 | Research brief | All empirical findings require reproducible citations, cohort definitions, dates and code-level evidence.

## Instructions for answering
For every substantive question, return (a) source/IDL or data URL and dated as-of version, (b) verified fact vs inference vs hypothesis, (c) how to measure it with as-of events, (d) likely counterarguments/selection biases, (e) impact on SniperFusion's architecture/strategy and (f) proposed falsification test. Use creator- and day-clustered uncertainty intervals for performance questions. **Never describe a statistically profitable strategy without verified after-cost, out-of-sample receipts.** Do not recommend manipulative strategies.

## A. Mint, creation and creator behavior (R01–R10)
1. What exactly occurs in Pump.fun `create` and `create_v2` transactions: account creation, Token-2022 vs SPL mint, authorities, fee payers, curve state and metadata?
2. What variations exist for SOL, USDC and pump-token quote paths, including nested quote-coin dependencies and multi-hop resolution?
3. How do standard, Mayhem, legacy cashback and other launch modes differ in initial inventory, fees, trade instruction support and graduation semantics?
4. What is the real slot/order relation between creation and first trade; what does a bundled atomic create+buy imply about possible entry times?
5. How to derive creator buy amounts precisely from instructions, inner CPIs and balance deltas without confusing rent, fees and transfers?
6. How often are creator/beneficiary funds traceable across associated wallets, and how reliable is clustering? What is unverifiable?
7. What mint/freeze/transfer authorities and Token-2022 extensions are relevant to sellability or malicious interference?
8. How many created mints see zero successful trades, and what proportion ever permits an economically viable resale after fees?
9. How do program upgrades, updated IDLs and new launch modes shift data decoding and signal validity through time?
10. Which creation-time attributes retain predictive power after excluding obvious creator-backed instant graduations and after transaction costs?

## B. Bonding curve, microstructure and order flow (R11–R20)
11. What are exact buy/sell quote formulas, rounding, virtual vs real reserves and signed virtual_quote_reserves under current program versions?
12. When are protocol/creator/buyback fees charged, where are they stored, and how does on-chain fee_config vary by venue/time/coin?
13. Which facts are visible at processed vs confirmed/finalized commitments; what can roll back, and what is known to a bot at decision time?
14. How to reconstruct initial 1s/5s/15s/30s/60s flows from raw successful transactions, with transaction-ordinal ordering inside slots?
15. What fraction of apparent unique buyers are funded by a common entity or visibly coordinated bundle? What false positives arise?
16. Can net buy pressure, first-sell intensity, creator holdings, wallet provenance and price acceleration predict executable next-horizon net returns?
17. What are the price/impact differences for 0.005/0.01/0.05/0.1 SOL sized orders at different reserve states?
18. How can post-launch order flow be forecast without using future knowledge, off-chain delayed timestamps or post-facto labels?
19. How frequently do pools become economically unexitable despite a theoretically valid sell instruction, and how should costs be modeled?
20. Which minute-of-day, chain congestion, launch rate and broader risk regimes change base rates enough to require separate models?

## C. Curve completion, synthetic migration and PumpSwap (R21–R30)
21. What is the exact `buy_v3`/ `buy_exact_quote_in_v3` crossing-buy behavior and event sequence (TradeEvent, CompleteEvent, PostCompleteBuyEvent)?
22. How are synthetic migration legs priced, accounted for, and distinguished from real post-migration PumpSwap fills in a strategy backtest?
23. How long is the real gap between curve completion and availability of a tradeable PumpSwap pool? What exit risk exists during the gap?
24. Which actions fail with BondingCurveComplete, NotEnoughTokensToBuy, or CashbackCoinNotSupported, and what version-aware fallbacks are safe?
25. What pool creation, canonical ownership, LP burn and liquidity-lock assumptions are actually verifiable, and what remains attackable?
26. How do PumpSwap v1/v2 AMM reserves, signed effective quote reserves and variable fee tiers change quoting at graduation?
27. What percent of graduations are creator-funded or near-instant bundles rather than organic trading? How does this differ by time and coin type?
28. Do entries before, during or after graduation generate better risk-adjusted net returns under the **same risk budget**, allowing for observed latency?
29. What percentage of post-graduation liquidity retains a viable exit after 1m/5m/1h/24h, excluding collector gaps and survivorship bias?
30. How does multi-hop quote exposure propagate: if a quote pump coin crashes, how does that change the target's SOL-denominated returns?

## D. Industry tactics, searcher competition and market integrity (R31–R40)
31. Which categories of profitable-appearing traders are observable on-chain: creators, early funded wallets, fast searchers, momentum traders, migration traders, selective copiers, and liquidity takers? Do not presume all are profitable.
32. How should a complete wallet-level P&L account for all trades, inventory, transfers, gas, tips, fees, unrealized holdings and multiple wallets?
33. Are apparent 'smart wallets' simply creators/beneficiaries or curated survivor accounts? How can independent prior performance be verified?
34. How do faster feeds (RPC logs, blocks, Geyser, deshred) compare on real observed latency, completeness, total cost and false-early signals?
35. How much are priority fees, transaction CU estimates, routing choices, block-engine tips and retry failures actually costing small-budget bots?
36. How does Jito bundle reception differ from landed execution, and what is the exposure to uncled-block re-broadcast and partial bundle effects?
37. How should detect-to-decision latency and send-to-land latency be measured from synchronized host timestamps rather than assumed 70ms fills?
38. Which alleged profitable techniques depend on manipulative wash trading, fabricated attention, privileged creator coordination or sandwiching, and must be treated as threats rather than target strategies?
39. Can organic wallet cohorts, independent funding, sustained exit liquidity and creator abstention distinguish repeatable demand from coordination?
40. Does a geographically closer RPC/leader/relay route produce enough additional *after-cost* edge to justify a paid provider before strategy edge is demonstrated?

## E. Security, smart contract and financial accounting (R41–R50)
41. How to derive every PDA, curve vault, ATA, quote account, volume accumulator and fee recipient for each supported version, and prove with current on-chain ground truth?
42. Which Pump.fun V3 and PumpSwap V2 builder variants have actually executed against deployed program ELF; which remain only IDL/shape-tested?
43. What exactly explains M3c's user_volume_accumulator SOL routing and sentinel TradeEvent values? Can actual balances reconcile every successful buy/sell/claim?
44. Does the required Token-2022 ATA or accumulator rent change break-even for repeated tiny buys and exits? Which deposits can be recovered?
45. How can the bot independently prove a valid sell route including transfer hooks/permanent delegate, frozen accounts, transfer fees and sufficient quote liquidity?
46. What can a malicious provider forge, delay, omit or reorder; what provider quorum/gap/backfill proof is required for entry authorization?
47. How do network partitions, timeouts, unconfirmed transactions, expired blockhashes, chain reorganizations and crashes affect duplicate-buy prevention?
48. Which source-code invariants guarantee observer, replay, fork and AI research processes cannot sign or broadcast funded mainnet transactions?
49. How to make risk limits composable across 10 simultaneous strategies sharing one wallet (cash, max exposure, max pending, per-mint/creator concentration)?
50. How can AI-generated code, token metadata and external research text introduce prompt injection, SSRF, malware or secrets leakage into the harness?

## F. Dataset validity, AI science and evaluation (R51–R62)
51. What is the actual denominator: all created tokens, traded tokens, coins with liquidity, coins reaching a threshold, completed curves, or surviving pools?
52. How to handle left truncation, right censoring, warm-up windows, missing RPC slots and mints never observed until after creation?
53. Which public Pump.fun graduation datasets have corrections/retractions? Reproduce headline rates before using as benchmark targets.
54. Which point-in-time features remain available within each proposed decision horizon after measured provider and computation latency?
55. How to split by launch date while purging overlapping future labels and creator/connected wallet clusters across training and holdout?
56. Which baseline strategies (no-trade, randomized eligible entry, immediate, fixed delay, simple flow threshold) best test a purported edge?
57. Which fill-model assumptions are falsified by fork account deltas and actual shadow-to-chain latency observations?
58. How to attribute performance across market regime, venue, launch type, token cohort, strategy version, position size, fee model and execution path?
59. What is the minimum independent sample of mints/days to estimate tail risk and beat a champion after multiplicity corrections?
60. How do we prevent repeated AI prompting, hyperparameter search and human inspection from leaking holdout information?
61. Which discovery approaches (rule search, Bayesian optimization, tree models, contextual bandits with safe exploration) outperform a simple deterministic ruleset after costs and complexity?
62. How can agent-generated hypotheses be falsified by negative controls and timestamp-shuffle tests before writing any executable code?

## G. Harness, system design and engineering operations (R63–R74)
63. How should `ReplayRunner` preserve chain event order while retaining priority shedding and emergency response semantics?
64. How should `PaperBroker` match multi-mint entries to exits, partial exits, cost basis, rent and independent strategy attribution?
65. What schema, hash chain, immutable manifest and toolchain/IDL/program pin are sufficient to reproduce one historical run byte-for-byte?
66. How should a batch orchestrator isolate 10 strategies yet support one shared-capital portfolio competition fairly?
67. How should the evaluator calculate open exposure, mark-to-exit equity, drawdown, missed opportunities, failure-adjusted P&L and latency quantiles?
68. What fault-injection cases are required across streams, replay, fork SVM, broadcaster and journal recovery?
69. What is the cheapest credible infrastructure plan using a single local machine, Parquet/DuckDB/SQLite, public read-only data and optional paid Geyser?
70. Which research steps can run asynchronously in the **user's own infrastructure** while the low-latency Rust service remains isolated?
71. How should AI proposals be serialized, schema-validated, sandboxed, tested and subjected to an adversarial independent reviewer?
72. Which quantitative gates permit paper-only champion promotion, and what evidence is required for operator-authorized funded trading?
73. How should model drift, producer lag, fee drift, program upgrades, regime changes and broken assumptions trigger emergency downgrade to no-trade?
74. What are the exact engineering acceptance tests and milestone exit criteria for a first reproducible ten-strategy cohort experiment?

## Required research outputs
1. **Protocol truth table**: exact version / venue / ix / accounts / state / fee / proof.
2. **Lifecycle event schema and state diagram**, including negative controls and censor cases.
3. **Annotated market-cohort dataset audit**: coverage, denominator, graduation and survival estimates with uncertainty.
4. **Searcher/tactic taxonomy**: observable evidence, hypothesized causal mechanism, threats, legal/ethical constraints, economics.
5. **Strategy hypothesis register**: ten initial reproducible candidate configs, phase eligibility, falsifiers and abstention logic.
6. **Security/financial red-team report**: reproducible failures, risk gates and code pointers.
7. **Harness contracts + phased plan**: proposed APIs, schemas, tests, infrastructure cost and 7-day shadow protocol.
8. **Ranked next 3 engineering PRs**: file-scoped changes, tests and expected output; no unreviewed live trading code.

## Reference anchors
Official: https://github.com/pump-fun/pump-public-docs
Solana: https://solana.com/docs/rpc and https://solana.com/docs/tools/transaction-landing
Jito: https://docs.jito.wtf/lowlatencytxnsend/
Market cohort: https://hiddenhand.trade/research/pump-fun-graduation-rate (2026-11-02)
Academic manipulation: https://discovery.ucl.ac.uk/id/eprint/10220651/
Research dataset: https://github.com/git-disl/MELT
Lookahead checks: https://www.freqtrade.io/en/stable/lookahead-analysis/
Beware corrected rates: https://doi.org/10.2139/ssrn.6915560 (6-minute collector window caveat).
