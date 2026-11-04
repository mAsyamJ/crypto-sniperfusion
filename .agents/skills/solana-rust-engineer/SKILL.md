---
name: solana-rust-engineer
description: Engineer SniperFusion's Rust Solana program decoders, transaction economics, event ingestion, local SVM tests, latency profiling and version-compatible dependencies using official Solana and Pump.fun references.
---
# Rust Solana Systems Engineer

Consult `pump-protocol-forensics`, `solana-execution-reliability`, `docs/SOLANA_COMPATIBILITY.md` and `docs/PROTOCOL_COMPATIBILITY.md` before Rust changes.

Official references: https://solana.com/docs ; https://solana.com/docs/intro/coding-with-agents ; https://github.com/solana-foundation/solana-dev-skill ; https://github.com/pump-fun/pump-public-docs ; https://github.com/solana-foundation/surfpool .

## Workflow

1. Pin the relevant official program/IDL revision and on-chain deployment. Compare docs with actual observed event/account fixtures; label uncertain claims.
2. Respect current proven generation-2 dependency graph; upgrade only with a compiling compatibility spike and full regression matrix. Treat unofficial bot code as research, not production import.
3. Write typed checked-unit math, exact signers/accounts, Program IDs, PDAs, quote-chain and fee snapshots; guard all Token-2022 extension cases and unexpected program upgrades.
4. Unit + property + official-IDL differential + LiteSVM/Surfpool fork execution tests with ephemeral local signer only. Verify account deltas, CU consumption, log outputs and failure paths.
5. Separate source-event receive latency, deterministic decision time, signing time and observed landing; optimize p99 only after measuring real ingestion and economic costs.
6. Never connect AI to funded signer or real mainnet broadcaster. Produce evidence receipts and source hashes before claiming parity.

## Deliverable

A versioned protocol/SDK compatibility table, failing regression and fix, fork fixture provenance, actual exit codes, unresolved program states and optional measured throughput/latency.
