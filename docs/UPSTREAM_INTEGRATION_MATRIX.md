# SniperFusion upstream integration map — 2026-11-04

This is a **capability map and licensing checklist**, not authorization to execute upstream bots or copy unlicensed code. Local pinned revisions are in `docs/UPSTREAM_MANIFEST.md`. Exact mainnet program/SDK versions must be checked against official docs/on-chain state at implementation time.

| Source | Feature of interest | Current SniperFusion | Recommended method | Priority |
|---|---|---|---|---|
| Chainstack `pumpfun-bonkfun-bot` (Apache-2.0) | Pump.fun/LetsBonk/StonkFun listeners, trade logic, migration-aware selling, exits | Rust Pump.fun/PumpSwap builders, fork tests, runtime and paper broker already exist | differential tests only for correct current behavior; close remaining M4 parity | P2 |
| Yuto Kazuma `solana-sniper-bot` (no standard license verified) | Rust Yellowstone, Raydium/Meteora, Jupiter/ZeroSlot, copy trading | Rust ingestion exists; broader DEX route and wallet-copy signal not implemented | clean-room from official Solana/Jupiter/DEX APIs; do not reuse unlicensed code | P3 |
| m8s-lab `solana-sniping-bot` (missing license grant) | create/migrate signals, creator initial buy, social metadata | H6 creation decoder exists; creator-linked balances and DevFlowMonitor do not | independently implement from on-chain instructions; social links ≠ rug detection; hostile metadata | P2 |
| bigmacman `solana-sniper-trading-mev-bot` (MS-PL) | Jito submission, circuit breaker, holder filter and TP/SL | journal + safe runtime exist; real Jito routing not implemented | reimplement against Jito official examples and current status APIs; do not import Ms-PL code blindly | P3 |
| Official `pump-fun/pump-public-docs` | V3/V2/IDLs and current fee routing | Pinned protocol reference but H7 Engine B still uses synthetic fallback fees in unobserved situations | fetch exact on-chain fee_config + account-state snapshots with as-of timestamps; protocol migration receipts | **P0** |
| Solana Foundation `solana-dev-skill` (MIT) | Runtime, SPL/token, SDK version compatibility, SVM, common errors | Local specialist playbooks exist but official reference not installed as managed dependency | optional reviewed/pinned operator installation; cite docs, validate generation-2 compatibility | P1 |
| Surfpool (Apache-2.0) | Local mainnet-state program execution | M3c real-program fork evidence | retain as protocol-correctness CI fixture; NOT real MEV/landed mainnet economics | P1 |
| Carbon by sevenlabs (MIT) | Rust ingestion/decoder processors | Custom Rust Geyser/WSS and event normalization exist | benchmark as optional shadow-only adapter; merge only if coverage/latency improves | P3 |
| Jito official searcher examples (Apache-2.0) | Bundle/send and status semantics | Not a live route | version-pinned reference, transactional result reconciliation and failover chaos tests before adding route | P3 |
| Raydium/Meteora official SDK/IDL | Pool state, swap quote and graduation semantics | Limited scope so far | one venue at a time; buy AND sell SVM fixtures and quote/math parity, license review | P3 |

## Integration priority is not feature count

**P0** Unblock G0 block-verifiable real launch-cohort data and point-in-time fee/reserve snapshots. `getSignaturesForAddress` cannot prove completeness. A provider that supports getBlock + versioned transactions is a prerequisite; do not buy a paid plan without operator approval.

**P1** Unify H6 collector and Rust CoverageLedger schemas, implement checkpointed bounded provider sweeps, then real-data Engine A/B side-by-side in a sealed manifest. Real fork tests validate instruction *mechanics*, not historical price time ordering or arrival latency.

**P2** Implement DevFlowMonitor from actual account/token deltas (creator buys separated from rent and tips; linked wallets marked probabilistic); safety checks for Token-2022 extensions, freeze/transfer controls, sell feasibility and actual executable liquidity. M4 can be closed selectively.

**P3** Compare multiple provider feeds and only then optimize Jito, Carbon or add Raydium/Meteora. Copy trading follows independently verified wallet P&L and survivorship-bias tests.

## Strategy research queue

- S00 no-trade (required control).
- Fixed delay vs fresh launch, conditioning on creator net flows and real buyer breadth.
- Momentum confirmation vs reverse momentum, with point-in-time features.
- Near-graduation and post-migration with real executable sell-route checks.
- Anti-rug abstention: creator-linked sale pressure, same-slot bundles, holder concentration.
- Selective wallet-follow: compare against random eligible-wallet baseline.
- Full-exit liquidity-aware positions and timeouts vs static percentages.
- Retry/fees/tips/latency execution-cost optimization *after* signal edge is evidenced.

For each, preregister data available by decision time, costs, expected direction, falsifier, negative controls and OOS creator/time holdout. No strategy has been shown to beat S00 on a real complete cohort.
