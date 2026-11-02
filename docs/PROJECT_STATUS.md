# Project Status

Living document; update at every milestone close. Format: state, evidence pointer, next.

## Milestones

| Milestone | State | Evidence |
|---|---|---|
| M0 — Audit, baseline, dependency selection, protocol verification | **COMPLETE** (2026-11-02) | `docs/evidence/M0.md` |
| M1 — Functional Rust core, config, risk authz, safe replay runtime | NOT STARTED (next) | — |
| M2 — Pump.fun event decoding, streaming interfaces, fixture parity | PLANNED | — |
| M3 — V2/V3 + PumpSwap lifecycle on local SVM | PLANNED | — |
| M4 — Chainstack parity scope closure | PLANNED | — |
| M5 — Token intelligence + DevFlowMonitor | PLANNED | — |
| M6 — Multi-DEX + copy trading | PLANNED | — |
| M7 — Execution routes + reconciliation + fault injection | PLANNED | — |
| M8 — Replay/backtest/benchmarks | PLANNED | — |
| M9 — Hardening + release readiness | PLANNED | — |

## M0 results (summary — verbatim detail in evidence file)

1. **Security audit**: no funded credentials in workspace; upstream install surfaces safe; m8s ships apparent live credentials (flagged, unused, uncopied).
2. **Chainstack baseline**: 38 verifier scripts discovered, **38 executed, 38 passed, runner exit 0**, isolated `uv sync --frozen` env.
3. **Dependency spike**: Gen-2 (solana-sdk 2.3.1 + litesvm 0.7 + spl-token-2022 9) **GREEN end-to-end**; Gen-4 **RED** (MSRV wall + litesvm 0.18.1 broken vs agave-4.4 lockstep). Decision: Gen-2 (ADR-0003). Toolchain updated 1.98.0 → 1.99.1 to give Gen-4 a fair trial (kept).
4. **Protocol audit** (`pump-docs` @ 639b87f): Chainstack's vendored IDLs lack the official **V3 curve trades** and **PumpSwap V2 trades** (17-account families) + multi-hop — Rust scope therefore exceeds Chainstack parity on these (G-1/G-2). Fee schedules are on-chain (`fee_config`) — no hardcoded rates. Synthetic migration + negative i128 `virtual_quote_reserves` captured with formulas and PDAs.

## Open items / blockers

- None blocking M1. Watch item: host RAM (~2–3.6 Gi free) — build jobs capped at 2.
- Deferred: `pump-rust-client` 0.5.0 evaluation (M3); Yellowstone version bump headroom (16→17) harmless (SDK-decoupled).
- Gen-4 revisit trigger documented in ADR-0003 (litesvm ≥0.19 or mollusk maturity).

## Next actions (M1)

1. Cargo workspace + 4 crates + pins + `rust-toolchain.toml` + jobs=2 config.
2. sniper-core: failing tests first — dual event identity/dedup, integer newtypes, config safety defaults, RiskAuthorization unforgeability; then implement.
3. sniper-runtime: priority bus (CRITICAL/POSITION/SPECULATIVE, shed-speculative-only), observe/replay/paper runners (no broadcaster capability), WAL interface round-trip test.
4. sniper-protocols: venue/program-ID registry from `docs/PROTOCOL_COMPATIBILITY.md` tables.
5. sniper-app: CLI modes + 4-gate live stub; tracing+redaction; CI workflow.
6. Gates: fmt/clippy/test green; evidence M1.md; commit.
