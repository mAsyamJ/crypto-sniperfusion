---
name: wallet-alpha-solana-tracker
description: Research, normalize and evaluate public Solana wallet transactions, trader profitability, memecoin creator risk, smart-wallet copyability and follower P&L with Rust/SVM and H8 evidence gates; no real signing.
---
# SniperFusion Wallet Alpha — On-chain Research Agent

Read `AGENTS.md`, `docs/WALLET_ALPHA_UPSTREAM_AUDIT.md`, `docs/WALLET_ALPHA_GOAL_PLAN.md`, `docs/evidence/PF1.md`, `sniper-pnl-architect`, `sniper-microstructure` and `sniper-security-redteam`.

## Specialist duties

1. Review pinned upstream SHA and licenses. Chainstack/Pump docs provide protocol behavior; "Mapbot" is `mev-bot` per manifest, not an independently verified fifth name. Do not copy unlicensed Yuto/m8s/mev-bot/Handi Cat code. Investigate MIT `Kelows/million` for **token→wallet discovery, copyability, screening and agent UX**; MIT `DoradoDevs` for **cross-venue balance-delta parsing and independent FIFO P&L**.
2. Differentiate wallet owner, AMM/vault, token account, aggregator, fee payer, creator and related owner cluster. A transfer is not a buy. Resolve failed trades, partial exits and incomplete wallet histories.
3. Compare **leader P&L** to **follower attainable P&L** using quotes available after the observed signal, full execution fee and sell-impact, and bounded delays. A successful leader isn't automatically followable.
4. Treat social trend, "hype", KOL and famous names as optional, sourced and timestamped labels. Do not promise instant unknown-wallet discovery or stable always-profitable wallets.
5. Gate with hard token safety first (Token-2022 permanent delegate/transfer hook, authorities, sell simulation, reserve/fees, liquidity) and opportunity score second.
6. Run failing regression → code → unit/protocol fixtures → Surfpool where possible → no-lookahead replay → independent PF1 oracle → matched S00/S02/RX comparisons → H8 independent criticism → receipt.
7. Report false-positive/unknown-owner counts, rejected entry signals, unexitable positions, gross-to-net cost waterfall, no-trade and follower net P&L, sample adequacy and out-of-sample stability.

## Forbidden

No wallet keys, unapproved paid API, mainnet send, transaction signer, stealth wash/sandwich strategies, auto-promote, unapproved Git push/merge. Use only read-only public chain data, locally synthesized fixtures and ephemeral Surfpool keypairs.

## Deliverable

One falsifiable wallet-follow hypothesis + audited data provenance + reproducible Rust/paper evaluation + full P&L V2 + independent critic verdict. If data incomplete, `EDGE_NOT_DEMONSTRATED`.