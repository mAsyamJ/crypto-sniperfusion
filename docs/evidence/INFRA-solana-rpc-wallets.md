# INFRA — Solana disposable wallets + operator RPC validation

**Date:** 2026-11-04 (blockTime of smoke transfer: 1793728804 ≈ 2026-11-03 20:26:44 UTC)
**Authorization:** operator provided both Infura endpoints in the task. Docs-only diff; no repo code changed; `LIVE_TRADING_ENABLED` never set; nothing pushed.

## Safety invariants held

- Mainnet endpoint used **read-only** via JSON-RPC (`getGenesisHash`, `getVersion`, `getSlot`, `getEpochInfo`). No keypair was ever combined with the mainnet URL; no mainnet broadcast attempt exists in this session.
- RPC URLs contain an embedded API key and live **only** in `~/.config/solana/sniperfusion/rpc.env` (mode 0600, outside the repository). The key is redacted from this document.
- All signing used disposable, generated-this-session wallets (no funded credentials). Recovery phrases were discarded at generation (stdout to `/dev/null`).

## Environment

| Component | Version / provenance |
|---|---|
| solana-cli | 4.3.0 (src 825efd18, feat c9ad34d2, client Agave) — pre-existing pinned binary, untouched |
| solana-keygen | 4.3.0 (same src hash) — installed this session, only `solana-keygen` extracted from `https://release.anza.xyz/v4.3.0/solana-release-x86_64-unknown-linux-gnu.tar.bz2`, tarball sha256 `c97289a8abb1d0efb497d8b5cb285baabd9b7f8ea6647f5d145c5dc8ff3611e8` |
| Host | WSL2 Ubuntu, RAM-constrained — no cargo builds this session |

## Wallets (disposable)

Generated with `solana-keygen new --no-passphrase --force`, files 0600 under `~/.config/solana/sniperfusion/` (dir 0700):

| Name | Address |
|---|---|
| devnet (signer) | `32qjUtSaDQy7t8AseCg4QGzu7wLFVV3uGBFcpXcsrYTM` |
| testnet | `5xTPhFkH6AUzAXo1tXSZjHi7b6RuR1gyuPWEZaF7BLe8` |
| devnet-recipient | `AgVycWfZZV4oK4z9SD1sC4QzcEVRfTRtevjveH3b4wV3` |

## RPC validation (read-only probes, curl)

| Endpoint | Genesis hash | Version | Slot / epoch |
|---|---|---|---|
| Infura devnet (`$SNF_DEVNET_RPC_URL`) | `EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG` | solana-core 4.4.0 (fs 3722100051) | 518298547 / 1199 |
| public `api.devnet.solana.com` (cross-check) | same — **MATCH** | — | — |
| Infura mainnet (`$SNF_MAINNET_RPC_URL`, read-only) | `5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d` | solana-core 4.3.2 (fs 3383571666) | 464991314 / 1076 |
| public `api.mainnet-beta.solana.com` (cross-check) | same — **MATCH** | — | — |
| public `api.testnet.solana.com` | `4uhcVJyU9pJkvQyS88uRDiswHXSCkY3zQawwpjk2NsNY` | solana-core 4.4.0 | — |

## Devnet live smoke — executed through the operator's private endpoint

1. **Faucet:** Infura devnet **supports `requestAirdrop`** (nonstandard but working): signature
   `5SpfGPS6vjuRqu4LzgC2MhbsLVVKnLvrzZjmghaa1NuwKrx8ney16chwA8wDLGjRqdyGbbmQMd6JKY7LJMC2GYqA`,
   finalized (slot 518298698), balance → 1 SOL. The public devnet faucet was rate-limited (not needed).
2. **Transfer 0.001 SOL** devnet signer → devnet-recipient, submitted and confirmed via `$SNF_DEVNET_RPC_URL`:
   - Signature `28a3q3FEpEjTh75vVLWWQ5WSQTky2cEK756FmzwmQgs3KsWyBnvL2BoDpvjm5ybdBHJBKLzUDwqBnvKsGpDzcG1X`
   - Finalized, `err: null`; slot **518298780**; blockTime 1793728804; fee **5000 lamports**; computeUnitsConsumed **150**; logs: `Program 11111111111111111111111111111111 invoke [1]` → `success`.
   - Balances: sender 1 → 0.998995 SOL (−0.001000 transfer − 0.000005 fee, exact); recipient 0 → 0.001 SOL.
   - `getSignatureStatuses` and full `getTransaction` receipt both served by the private endpoint (public devnet returned `null` for `getTransaction` on this signature — the private endpoint retains detail the public one does not).

## Gates

| Gate | Result |
|---|---|
| Offline harness tests (`python3 -m unittest discover -s harness/tests`) | **PASSED** — 3/3, exit 0 |
| Wallet generation + perms (0700 dir, 0600 files, phrases discarded) | **PASSED** |
| Genesis-hash cross-checks, devnet + mainnet | **PASSED** (both MATCH) |
| Devnet airdrop + signed transfer lifecycle (submit → confirm → receipt → balance deltas) | **PASSED** via private endpoint |
| Testnet faucet funding | **BLOCKED** — `api.testnet.solana.com` rate-limited on 2 bounded attempts; wallet + read probes passed; no testnet send possible (0 SOL) |
| Rust fmt / clippy / tests | **NOT_RUN** — no Rust code changed; HEAD `bf694d6` CI green |
| Surfpool fork Pump.fun execution | **NOT_RUN** — already evidenced in M3c (`b305106`); heavy, out of infra scope |
| "S00–S10 synthetic strategies" (external prompt) | **NOT_RUN** — do not exist in this repo (`research/` empty) |

## Open notes

- The Infura API key was shared in plaintext chat by the operator; it is nowhere in the repo (pre-commit grep verified). Rotation recommended at the operator's convenience.
- Infura devnet `requestAirdrop` availability is a provider behavior, not a guarantee — treat as convenience only.
- This SOL transfer validates wallet/RPC plumbing only. It is **not** evidence of sniper trading capability; Pump.fun execution evidence remains M3c's Surfpool fork tests.
