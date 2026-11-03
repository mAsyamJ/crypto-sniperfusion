---
name: pump-protocol-forensics
description: Verify current Pump.fun and PumpSwap instructions, IDLs, accounts, curve/PumpSwap fee economics, V3 synthetic migration and real local fork behavior; use for protocol compatibility work.
---
# Pump.fun and PumpSwap Protocol Forensics

Read `AGENTS.md`, `docs/PROTOCOL_COMPATIBILITY.md`, `docs/evidence/M3c.md`, and `crates/sniper-protocols/src/{curve,amm,pda,builders,events}.rs` before modifying protocol code.

## Workflow
1. Pin the official `pump-fun/pump-public-docs` commit, deployed program IDs, relevant program upgrade/version, and fee_config account. Report all hashes in the evidence record.
2. Enumerate creation, buy_v2/v3, sell_v2/v3, crossing buy, completion, migration and PumpSwap sell paths, including Token-2022 ATA, Mayhem, cashback and SOL/USDC/pump-coin quote variants.
3. Derive account and PDA metas from official docs; independently confirm writable/signers and key ordering with observed transactions. Distinguish logs, decoded event amounts, wallet deltas and claimable accumulator balances.
4. Run official-IDL differential tests then local Surfpool real-program tests using throwaway keys only, including failing accounts and exact balance conservation.
5. Document `VERIFIED_SOURCE`, `STRUCTURAL_TEST`, `FORK_EXECUTED`, and `NOT_TESTED` distinctly.

## Required deliverable
`docs/research/protocol-truth-table.md`: version, discriminators, accounts, fee route, quote units, source links, on-chain receipts, local commands, and gaps.

Never invent a viable sell path or infer rent/fees from old tutorials. No mainnet broadcasting.
