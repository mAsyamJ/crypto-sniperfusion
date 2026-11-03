# INFRA-3 — Mainnet readiness: unfunded wallet + read-only watch capability

**Date:** 2026-11-04 (local) / 2026-11-03 ~19:0x UTC (live runs)
**Authorization:** operator asked to "prepare the mainnet too" after INFRA-2. Delivered
within the repo's non-negotiable bounds: **no funded keys, no mainnet broadcast, agents never
sign mainnet.** What "prepared" means here is spelled out below — an address exists and the
system can OBSERVE mainnet through the operator's read-only RPC; no signing path exists.

## What was delivered

1. **Unfunded disposable mainnet wallet** — `i2x5bD3ZZfTa6SShT3yd2TPA8TKtCJuhuzXKXaH6r2f`
   - Keypair at `~/.config/solana/sniperfusion/mainnet.json` (0600, outside the repo),
     generated with the recovery phrase discarded, exactly like the devnet/testnet wallets.
   - Balance verified **0 lamports** via the operator's mainnet RPC (read-only getBalance).
   - Policy (recorded in a README next to the keypairs): this is a placeholder/watch address.
     If the operator ever funds it, it becomes a real-money credential and must be moved off
     this machine; agents must never be given a funded mainnet signer (AGENTS.md /
     DEVNET_EXECUTION_PILOT.md policy).
2. **`sniper rpc-probe --address <PUBKEY>`** — read-only watch capability:
   - Adds exactly one getter call (`getBalance`) for a validated base58 pubkey to the probe;
     report gains `watch_address` + `watch_balance_lamports` (omitted when not requested).
   - Invalid address fails BEFORE any network call (`ProbeError::InvalidAddress`), exit 1.
   - Still structurally read-only: getter calls only, no signer/keypair import, no send path.

## Files changed

- `crates/sniper-runtime/src/rpc_probe.rs` — watch param, `InvalidAddress` variant, report fields.
- `crates/sniper-runtime/tests/rpc_probe.rs` — watch-path mock assertions (5th `getBalance`
  call served by the offline mock) + invalid-address pre-network test; now 7 tests.
- `crates/sniper-app/src/main.rs` — `--address` flag wired through `run_rpc_probe`.

## Acceptance results

| Acceptance | Result |
|---|---|
| watch address appears in report with balance (offline mock, 5-call order asserted) | **PASSED** |
| invalid base58 address → `InvalidAddress` before any network call | **PASSED** |
| report omits watch fields when `--address` absent (serde skip) | **PASSED** |
| LIVE mainnet: `rpc-probe --cluster mainnet --address i2x5…r2f` via operator env | **PASSED** — genesis verified, slot 465069252, `watch_balance_lamports: 0`, exit 0 |
| LIVE devnet positive control: `--address 32qj…rYTM` via operator env | **PASSED** — `watch_balance_lamports: 998995000` (exactly the funded balance) |
| LIVE invalid address on mainnet | **PASSED** — `error: invalid watch address "not-a-pubkey!!"`, exit 1 |

## Gates (run on the final tree, see concurrency note)

| Gate | Result |
|---|---|
| `cargo fmt --check` | **PASSED** |
| `cargo clippy --all-targets -- -D warnings` | **PASSED** |
| `cargo clippy --all-targets --all-features -- -D warnings` | **PASSED** |
| `cargo test` / `--all-features` | **PASSED** — 249 passed / 0 failed both |
| Secret guard (added lines, blocking) | **PASSED** (0 hits; no endpoint/key material in diff) |

## Concurrent-change note (2026-11-04, second occurrence)

During this work the parallel session advanced `main` again: `dd6015d..4124839` = 6 commits
(H6 collector + H7 accounting/strategy work). Its commit messages reference "live Mainnet
attempts" (data-collection hardening) — flagged here so the operator is aware multiple
agents are active on this worktree. My delta stayed uncommitted-in-worktree until this
commit; all gates above ran on the combined final tree.

## Open items

- Testnet wallet remains unfunded (manual web faucet, see INFRA-2).
- If the operator wants richer read-only mainnet observation (token accounts, signatures,
  stakes), extend the probe the same way — one getter at a time, no signer, ever.
