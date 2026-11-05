# Evidence — write-once execution receipts

**Status:** ACTIVE · **Rule:** a receipt is immutable once written. Corrections, amendments, and reversals land in **new** files — never edits. Promote nothing from `harness/runs/` into a claim without a receipt here.

Evidence is the only currency this repository trades in (`SOUL.md` Law 10). `docs/PROJECT_STATUS.md` links to these files; the tables below are the index.

## Receipt contract

A receipt records **executed work**, not intentions. Required content:

1. **Identity:** date, branch, commit SHA(s), and what milestone/mission it closes.
2. **Reproduction:** exact commands and exit codes; fixture/data hashes where applicable.
3. **Measurements:** the numbers the claim rests on (test counts, lamports, coverage states, latencies), with units.
4. **Class labels:** every claim labeled with the lowest evidence class it supports — implementation-proof ladder (`VERIFIED_CODE`, `SYNTHETIC_TESTED`, `SURFPOOL_REAL_PROGRAM_FORK_EXECUTED`, `DEVNET_PROGRAM_TRANSACTION`, `MAINNET_READ_ONLY_OBSERVED`, `BLOCKED`) and, for market claims, the chain-execution ladder (`SYNTHETIC` → `FORK_MECHANICS` → `SHADOW_ESTIMATE` → `LIVE_CONFIRMED`). Definitions: `docs/GLOSSARY.md`.
5. **Open defects and limitations:** what is still wrong, unknown, or BLOCKED — stated, not buried.
6. **What is NOT claimed:** the honest scope line (e.g. "mechanics COMPLETE, zero launches captured").

Rules:

- No secrets, endpoint URLs with tokens, or key material — ever.
- Immutable outputs stay immutable: `harness/runs/` artifacts referenced by a receipt are write-once (0444) and never regenerated in place.
- A receipt that contradicts an earlier one does not edit it; it supersedes explicitly and says why (see `G0-H8-session.md` correcting the H6 provider conclusion).
- Synthetic-derived outputs are permanently `promotable=false`; a receipt may never present them as chain-execution evidence.

## Index

| Receipt | Closes | One line |
|---|---|---|
| `M0.md` | M0 | Audit, baseline (chainstack 38/38 oracle scripts), Gen-2 spike, protocol audit |
| `M1.md` | M1 | 4-crate workspace, config/risk-authz/replay, all gates green |
| `M2.md` | M2 | Event decoding + log attribution differential 320/320 vs chainstack oracle |
| `M3.md` | M3 | PDA module, exact integer math, builders, order lifecycle, paper fill engine |
| `M3c.md` | M3c / H1 | Real deployed pump.fun ELF execution on Surfpool mainnet fork |
| `H0.md` | H0 | Sealed `AuthorizedOrder`, crash-tolerant journal, EntryGate, secret scan |
| `H2.md` | H2 | Event-sourced PaperBroker; broker/evaluator reconcile to the lamport |
| `H4.md` | H4 | `snf-harness` CLI skeleton; promotion requires full gate chain incl. shadow |
| `H6.md` | H6 | Dataset contracts, coverage states, creation-cohort collector (coverage INCOMPLETE at the time) |
| `H7.md` | H7 | Strategy SDK S00–S10, Engine A byte-exact parity, oracle, evaluator, 12 negative controls |
| `PF0.md` | PF0 | P&L audit; fee truth on-chain-observed (curve 95+30 bps; PumpSwap tiered 125→30) |
| `PF1.md` | PF1 | P&L V2 double-entry oracle; deterministic non-promotable receipts; S00 dominates |
| `INFRA-solana-rpc-wallets.md` | INFRA | Disposable wallets + operator RPC validation; devnet airdrop/transfer receipt |
| `INFRA-2-rpc-integration.md` | INFRA-2 | `sniper rpc-probe` + endpoint resolution; credential-safe reports; 179/179 ×2 |
| `INFRA-3-mainnet-readiness.md` | INFRA-3 | Unfunded watch wallet; mainnet read-only readiness; 249/249 ×2 |
| `G0-H8-session.md` | G0 + H8 | Provider unblocked (wire-param fix), three COMPLETE windows, zero creates = failure storm, registry machinery |
| `WA1-WA3.md` | WA1–WA3 | Normalizer, `sniper wallet-pnl`, delayed follower; all delays lose vs S00 on real smoke |
| `HARNESS2-INTEGRATION.md` | Harness² + WA-full + W01 | Deterministic harness² channel, million owner-graph/consensus/ladder, Dorado differentials, W01 rejected (§E) |
| `G0-SWEEP-20261105.md` | G0 session 2 | Probe green; 4-slot COMPLETE window; 3,000-slot sweep resumable at 1,488/3,000 blocks, 0 gaps, 0 creates (partial); budget-stop KeyError regression found+fixed+tested (117/117) |
| `G0-WINDOW-COMPLETE-20261105.md` | G0 session 2 close | **First at-scale COMPLETE window: 3,001/3,001 blocks, 0 gaps, 0 launches**; sampled diagnostic: 0 create ATTEMPTS (absence, not failure), 82% program-tx failure rate; S11/S12 blocked by market structure; time-of-day + launchpad-migration next probes |
| `D0-LAUNCH-ATTRIBUTION-AUDIT-20261105.md` | D0 | Launch-attribution audit: ALT dict-flattening + create_v2 diag blind spot + log attribution fixed red-first (132/132); official IDL re-pinned (create_v2 = documented standard creation path); **downgrades "zero create attempts" to "zero LEGACY create attempts; create_v2 unmeasured"**; window re-eval BLOCKED (ckpt absent ∧ RPC unset); wallet_observer ALT defect found, deferred |
| `DASH-00-20261105.md` | DASH-00 | Control Tower tooling provisioning: baseline all green (201+312 Rust, 132 Python), upstream license/security review (anti-slop MIT @9504989, ui-ux-pro-max MIT @b7d5353, superpowers MIT @22f0b46, skills CLI 1.7.6), 14 skills project-local with double pin layer, AntiSlop `during` session-override (no global writes), 33/33 lint PASS; zero dashboard code yet (by design) |
| `DASH-01-20261105.md` | DASH-01 | Dashboard truth audit: provenance matrix P1–P17 verified at HEAD (bundle shapes diverge by subcommand; NO NAV series; NO daemon/heartbeat; PaperBroker flat synthetic 100 bps; observe fixture-only) + defects D-1–D-8; REQUIREMENTS (API v1 contract, financial/calendar contracts, state catalog) + DESIGN (contrast-measured tokens via antislop-human checker) + ARCHITECTURE (crates, SQLite schema, threat model DA-1–7) + TEST_PLAN (red-first T-ids) + ADR-0013; docs+ADR only, no product code |

Adding a receipt: new file `{ID}.md` in this directory, indexed here and in `docs/PROJECT_STATUS.md`, following the contract above.
