# Project Status

**Repository: `mAsyamJ/crypto-sniperfusion` (PUBLIC — verified 2026-11-03, owner decision: keep public).** Project name: **SniperFusion**. Chain-agnostic naming on purpose: Solana is the only implemented chain today; every doc claim about capabilities remains Solana-scoped until multi-chain code lands behind its own milestone gates.
**Operational consequence of PUBLIC:** strategy logic, risk thresholds, and evaluation results are world-readable. Enforced invariants that make this acceptable: no keys/secrets/`.env` in history (H0.4 scans enforce), no funded credentials, no infrastructure endpoints in docs. Revisit visibility before any live-trading authorization — see ADR-0002.

Living document; update at every milestone close. Format: state, evidence pointer, next.

## Milestones

| Milestone | State | Evidence |
|---|---|---|
| M0 — Audit, baseline, dependency selection, protocol verification | **COMPLETE** (2026-11-02) | `docs/evidence/M0.md` |
| M1 — Functional Rust core, config, risk authz, safe replay runtime | **COMPLETE** (2026-11-02) | `docs/evidence/M1.md` |
| M2 — Pump.fun event decoding, streaming interfaces, fixture parity | **COMPLETE** (M2a differential 320/320; M2b providers+supervision+cache) | `docs/evidence/M2.md` |
| M3 — V2/V3 + PumpSwap lifecycle | **COMPLETE incl. M3c** (real local execution on Surfpool fork, 2026-11-02) | `docs/evidence/M3.md`, `docs/evidence/M3c.md` |
| H2 — Real paper broker (event-sourced) | **COMPLETE** (2026-11-02) | `docs/evidence/H2.md` |
| H4 — Improvement harness CLI skeleton | **COMPLETE** (2026-11-02; H3 shadow window intentionally not started) | `docs/evidence/H4.md` |
| INFRA — Solana disposable wallets + operator RPC validation | **COMPLETE** (2026-11-04; docs-only, no code change) | `docs/evidence/INFRA-solana-rpc-wallets.md` |
| INFRA-2 — Operator RPCs integrated: `sniper rpc-probe` + endpoint resolution | **COMPLETE** (2026-11-04; full engineering loop, 179/179 tests ×2) | `docs/evidence/INFRA-2-rpc-integration.md` |
| INFRA-3 — Mainnet readiness: unfunded watch wallet + `rpc-probe --address` | **COMPLETE** (2026-11-04; read-only only, 249/249 tests ×2 on merged tree) | `docs/evidence/INFRA-3-mainnet-readiness.md` |
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

## M1 results (summary — verbatim detail in evidence file)

Workspace: 4 crates (core/protocols/runtime/app), solana-sdk =2.3.1 per ADR-0003, lockfile committed, rustc 1.99.1 pinned, build jobs=2. Gates all green: fmt, clippy×2 (`-D warnings`), tests×2 (8 suites each, 0 failures). CLI demonstrates offline replay/paper end-to-end (7 events → 2 dupes deduped → 5 decisions) with honest refusals for observe-streaming (M2) and live (4-gate + M7). Dual event identity, 3-tier shedding bus, crash-tolerant journal, base58 fixture serde landed.

## Open items / blockers

- None blocking M2. Watch item: host RAM — build jobs capped at 2 (held throughout M1).
- Deferred: `pump-rust-client` 0.5.0 evaluation (M3); Yellowstone bump headroom (16→17) harmless.
- Gen-4 revisit trigger documented in ADR-0003 (litesvm ≥0.19 or mollusk maturity).

## M2a results (summary — verbatim detail in evidence file)

Event decoding + log attribution + differential harness complete: 320 synthetic fixtures (8 kinds incl. old-format truncated), oracle = chainstack IDLParser; comparator result ALL MATCH (4792/4792 comparable fields, exit 0). Two oracle limitations discovered, documented, and excluded with rationale: D-1 vec<defined> unsupported with silent offset misalignment (oracle post-shareholders fields are garbage), D-2 EOF artifact synthesis. Rust decoders are prefix-tolerant per-field (decoded_fields counts), garbage-proof (property test), and sha256-verified against both IDLs. All gates green (fmt, clippy ×2, tests ×2).

## Open items / blockers

- M2b (live provider adapters: WSS logs/blocks, Yellowstone gRPC, SubscribeDeshred detection, reconnect/gap/reorder against real streams, blockhash cache) — NOT STARTED; requires opt-in read-only provider credentials. First task next session.
- PostCompleteBuyEvent layout (synthetic migration pool leg) — pending M3 with SYNTHETIC_MIGRATION.md quoting work.
- Host RAM watch item unchanged (jobs=2 held).

## M3 results (summary — verbatim detail in evidence file)

PDA module (doc-constants verified), exact integer math (curve constant-product, i128 signed effective reserves, bounded-output invariant), six instruction builders (curve v3×3 @17 accounts, v2 buy 27 / sell 26, PumpSwap sell_v2 @17) with doc-table structural tests, order lifecycle machine with journal write-ahead + full crash-recovery test matrix (UNKNOWN mapping, duplicate-submission rejection, entry gating), paper fill engine with fee-in/out round-trip economics. Gates all green (fmt, clippy×2, tests×2: 89 tests across workspace).

## Open items / blockers

- **M3c (next session, first task)**: LiteSVM execution vs real pump.fun/PumpSwap program binaries — needs opt-in program-ELF fetch or committed fixture; PostCompleteBuyEvent layout from synthetic-migration fixtures.
- M4 then proceeds: LetsBonk/LaunchLab parity scope, exit-strategy parity, CONFIG_MIGRATION.md, runtime-Python-independence proof.

## Next actions (M3c → M4)

1. M3c: opt-in program-ELF fixture fetch script (read-only RPC), LiteSVM harness executing builders end-to-end, PostCompleteBuyEvent decode.
2. M4: Chainstack parity closure per CAPABILITY_MATRIX rows C2–C24; `docs/CONFIG_MIGRATION.md`;LetsBonk/StonkFun adapter scope decision.
3. Carry: `pump-rust-client` 0.5.0 build-helper evaluation.

## H-mission results (2026-11-02, branch codex/harness-m3c-security)

- **H0** (`bd5c3b0`): sealed `AuthorizedOrder` trust boundary, durable
  checksummed single-writer journal with crash-tail repair, backfill-proof
  EntryGate, history-aware secret scan + cargo-deny CI, docs visibility fix.
- **H1/M3c** (`b305106`): Surfpool v1.7.0 pinned (sha256 recorded); real
  local `buy_v3`/`sell_v3` + negative control against the deployed pump.fun
  ELF on a loopback mainnet fork; official-IDL conformance fixes
  (`partial_fill` arg, account mutability, ATA prerequisite); live-PDA
  ground-truth tests; deployed-program economics documented (volume
  accumulator SOL routing, quote-era event semantics open).
- **H2** (`f38f1cc`): event-sourced PaperBroker (fills/fees/rent/slippage,
  no-lookahead, migration, ledger crash-recovery, deterministic export);
  CLI paper mode rewired with reconciled P&L receipt accepted by the
  independent evaluator (broker realized 2,080,057 vs evaluator net
  2,068,057 = exactly the separately-booked 12,000 network+tip).
- **H4** (harness skeleton): `harness/snf-harness` — doctor/verify/replay/
  fork/shadow/evaluate/compare/promote with write-once run manifests;
  promotion requires the full gate chain INCLUDING the ≥7-day shadow window
  and never enables live trading. Tests: 167/167 both feature sets.
- **H3**: NOT STARTED (needs ≥7 days read-only observation; the harness
  `shadow` command refuses to fabricate it).

## Research Harness V0 implementation branch — 2026-11-03

Branch `codex/research-harness-agent-skills-20261103` (requires PR review and CI confirmation): fixes identified P0 replay priority reordering and global-mint trade attribution in the paper broker, adds regression tests, a strict offline 11-policy **synthetic surrogate** research batch CLI, immutable-output rules, eight repository-local Solana/quant/security/critic skills, and an operational runbook. See `harness/docs/RESEARCH_LOOP_RUNBOOK.md` for exact scope and commands.

**Remaining gates (NOT COMPLETE):** genuine all-launch historical dataset, on-chain point-in-time feature reconstruction, shared-capital Rust backtester, independent full-portfolio economic reconciler, actual latency/fee calibration, statistical holdouts, automated AI orchestration, complete fork protocol matrix, and >=7-day read-only shadow. The V0 surrogate has `promotable=false` and cannot activate mainnet trading. Do not claim a positive strategy edge from it.

## INFRA results (2026-11-04, operator-authorized)

Disposable devnet/testnet wallets + operator-provided Infura RPC (mainnet+devnet) validated.
Genesis hashes cross-checked against public clusters (MATCH ×2; mainnet strictly read-only, no
keypair ever combined with it). Live devnet smoke through the private endpoint: `requestAirdrop`
1 SOL confirmed, then signed 0.001 SOL transfer finalized (slot 518298780, fee 5000 lamports,
150 CU) with exact balance deltas and full `getTransaction` receipt. Testnet faucet BLOCKED
(rate-limited, 2 bounded attempts). RPC URLs live only in a 0600 env file outside the repo;
Infura key redacted from all docs. Details: `docs/evidence/INFRA-solana-rpc-wallets.md`.

## INFRA-2 results (2026-11-04, operator-authorized)

Operator RPCs are now integrated in the Rust system: `sniper rpc-probe --cluster
mainnet|devnet|testnet` resolves endpoints (`SNF_{MAINNET,DEVNET,TESTNET}_RPC_URL` env
override → pinned public fallback; invalid env = hard error, never silent fallback),
verifies cluster identity against pinned genesis hashes, and prints a credential-safe JSON
report (`redact_url` strips path/query — Infura keys are unprintable by construction; live
leak grep = 0). Structurally read-only: four getter calls, no signer types, no send path.
Live-verified against both operator endpoints + public testnet. Gates: fmt, clippy ×2,
tests 179/179 ×2 (6 new contract tests incl. offline mock JSON-RPC happy + genesis-mismatch
paths). Faucet status: devnet wallet funded (0.998995 SOL); RPC faucets CLOSED cluster-wide
(HTTP 410, devnet-via-Infura and testnet-public) — testnet funding is operator-manual via
https://faucet.solana.com (address in evidence). `SNF_SOURCE_RPC` for Surfpool forks can now
be fed from `SNF_MAINNET_RPC_URL` (read-only). Details: `docs/evidence/INFRA-2-rpc-integration.md`.

## H5 candidate — Devnet protocol verification and offline research loop (2026-11-04)

Draft PR #6 adds a Devnet-genesis-bound Rust protocol pilot (read-only preflight; explicit transaction test), a separate read-only cluster probe, and bounded synthetic-only research cycles with immutable trial receipts. Full validation is tracked in PR #6. Real Devnet transaction signatures have not been collected by this integration. These demonstrations cannot prove profitability, and mainnet live trading remains disabled. See `docs/DEVNET_EXECUTION_PILOT.md` and `harness/docs/RESEARCH_CYCLE_V1.md`.
