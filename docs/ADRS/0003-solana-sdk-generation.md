# ADR 0003 — Solana SDK generation selection (compile-spike driven)

Status: accepted (M0) · Date: 2026-11-02 · Evidence: `docs/evidence/M0.md` §spike, `docs/SOLANA_COMPATIBILITY.md`

## Context

The amendment forbids defaulting to a Solana SDK generation without an actual compilation spike. As of 2026-11-02 the ecosystem spans solana-sdk 2.3.1 (classic facade over modular crates), 3.0.0, 4.1.0, 5.0.0, with litesvm 0.7 (Gen-2 aligned) … 0.18.1 (agave-4 aligned), spl-token-2022 9…12, yellowstone-grpc-client ≤17 (SDK-independent), and pump-rust-client 0.5.0 requiring sdk `>=2.1.21,<3`.

## Decision

Adopt **Generation 2**: `solana-sdk =2.3.1`, `litesvm =0.7.0`, `spl-token-2022 =9.0.0`, `yellowstone-grpc-client 16.0.1/proto 14.0.1`, optional `pump-rust-client 0.5.0` (evaluation), toolchain rustc 1.99.1. Exact pins live in the workspace `Cargo.toml` + committed lock.

## Evidence summary

- **Gen-2 spike: GREEN.** Full build + runtime probe (legacy & v0 tx on LiteSVM, balance assertions, program-id parse) exit 0; no MSRV conflicts.
- **Gen-4 spike: RED twice.** (a) 19 agave-4.4 crates require rustc ≥1.98.1 (host had 1.98.0); (b) after toolchain update, litesvm 0.18.1 itself fails to compile against the agave-4.4 family its `^4.3.0` requirements resolve to; downgrades blocked by `=4.4.0` lockstep pins and missing intermediate releases. Local-SVM testing is a hard requirement for M3 — no litesvm, no adoption.
- **Gen-3/5/6: single-release transitional lines** without ecosystem validation — rejected for churn risk.

## Consequences

- (+) Only generation compatible with pump-rust-client, litesvm, and yellowstone simultaneously today; matches the most-deployed line (chainstack's solana-py is 2.x-era; yuto pins 2.1.14).
- (−) 2.x facade will age; mitigated by writing core types against narrow re-exports (Pubkey/Signature/Hash/Instruction) so a future generation bump is mechanical.
- Revisit trigger: litesvm ≥0.19 compatible with agave ≥4.3 family, or mollusk reaching feature parity — then re-spike and ADR-amend.
