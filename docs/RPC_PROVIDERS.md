# RPC Providers and Data Availability — G0 critical-path tracker

**Version:** 1.0 (2026-11-05) · **Status:** ACTIVE · **Update rule:** every probe, provider change, or new failure mode lands here with its receipt reference.

The entire S11+ strategy frontier is gated on real `coverage_state=COMPLETE` launch cohorts (`SOUL.md` §6). This file tracks who can serve that data, what has been verified, and what remains blocked. Credentials never appear here — env-only, outside the repo (`AGENTS.md` §2).

## Current provider matrix

| Provider | Scope | Verified capability | Receipts |
|---|---|---|---|
| Operator Infura (env-only, `SNF_MAINNET_RPC_URL`) | Mainnet read-only | **Serves full `getBlock` at `maxSupportedTransactionVersion=1`** — genesis verified, 3 read-only probe calls, sample block 534 txs; three COMPLETE block-verified windows captured (3/7/41 slots, zero gaps, signatures reconciled) | `docs/evidence/G0-H8-session.md`, `harness/runs/rpc-capability/` |
| Operator Infura (`SNF_DEVNET_RPC_URL`) | Devnet | Airdrop + signed transfer verified (slot 518298780, fee 5000 lamports, exact balance deltas, full `getTransaction` receipt) | `docs/evidence/INFRA-solana-rpc-wallets.md` |
| Public mainnet RPC | Mainnet read-only | HTTP 429 under load for `getBlock` — not usable for cohort capture | `docs/AGENT_OPERATIONS.md` |
| Public testnet | Testnet | Reachable, but faucet rate-limited / closed (HTTP 410 cluster-wide); testnet funding is operator-manual via faucet.solana.com | `docs/evidence/INFRA-2-rpc-integration.md` |
| Surfpool local fork (`SNF_SOURCE_RPC` ← mainnet env) | Local fork execution | Pinned v1.7.0, sha256 recorded; used for H1/M3c real-ELF execution — mechanics evidence, never market evidence | `harness/docs/surfpool-pin.md`, `docs/evidence/M3c.md` |

## Known capability failure catalog (lessons, kept)

| Failure | Root cause / note |
|---|---|
| "Infura cannot serve getBlock" (H6 conclusion, 2026-11-04) | **Wrong — our parameter was.** The collector pinned `maxSupportedTransactionVersion: 0`; versioned (ALT) blocks require 1. Fixed in `aaa881c` with a wire-param regression test. Lesson: classify `BLOCKED_RPC_CAPABILITY` only after proving the provider, not our client, is the blocker |
| HTTP 429 on public RPC | Rate limit under load; do not build cohort capture on public endpoints |
| Per-signature recovery fallback | Exceeded the runtime bound and produced no receipt — block-level evidence or nothing |
| Faucet HTTP 410 | Cluster-wide closure, not our bug; operator-manual funding only |

## What G0 still needs (open blockers)

1. ~~Launch capture at scale~~ **DONE (2026-11-05 coverage; 2026-11-06 content — D1a)**: the 3,001-block COMPLETE window re-captured with the D0-fixed collector yields **232 `create_v2` launches / 232 unique mints, 0 gaps** — the prior "zero launches" was a decoder blind spot, not market structure (`docs/evidence/D1A-TIME-OF-DAY-CREATE-V2-20261106.md`). Time-of-day probes: creation not hour-gated. Capture hardening: transport-layer failures (timeouts/resets) now convert to `RpcError` (retry/gap, never crash); provider leg-deaths are resumable via flock+checkpoint; `getBlockHeight` on this endpoint serves stale values — use `getEpochInfo` for tip; 20-page `getSignaturesForAddress` cannot reach day-old windows from tip (block scan is the completeness path for historical windows).
2. **Reserve/fee snapshots as-of decision** — Engine B refuses to run without observed reserves; the next collector capability (Mission B in `docs/evidence/G0-H8-session.md`); launch-time reserves already decoded in the D1a create records.
3. **Multi-hour cohort** — prerequisite for the first real H8 tournament (W02) and any S11+ registration.

## Provider evaluation checklist (before any new provider or large collection)

From `docs/AGENT_OPERATIONS.md` and `docs/NEXT_GOAL_H8_G0.md` PR-1 — run this before trusting a provider with a capture:

1. `getGenesisHash` / `getVersion` / `getSlot` / `getBlock` probes — old **and** recent finalized slots.
2. Record exact HTTP error kinds **without credentials**: distinguish 429 / 403 / 504 / UnsupportedTxVersion / Pruned / SkippedSlot.
3. Set `maxSupportedTransactionVersion` correctly for versioned transactions; never assume provider compatibility.
4. Tiny 1–4 slot window first; then checkpoint/resume without duplicate blocks; time and request budgets enforced.
5. Completeness only from `getBlock` evidence + signature reconciliation; never infer success from `getSignaturesForAddress`.
6. Provider failure ⇒ `BLOCKED_RPC_CAPABILITY`, never a green run.
7. Entry point: `python3 harness/rpc_capability_probe.py --out harness/runs/rpc-sample-unique.json` (≤5 read-only calls, sanitized write-once receipt). `FULL_BLOCK_SAMPLE_AVAILABLE` = provider compatibility only — it proves nothing about G0 coverage or profitability.

## Rules

- Endpoints/keys live only in a 0600 env file outside the repo; never in prompts, logs, commits, CI, or run manifests.
- `redact_url` behavior is the standard: keys unprintable by construction.
- Adding a provider row requires a receipt (probe output or capture manifest) — no hearsay rows.
