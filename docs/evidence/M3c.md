# M3c Evidence — Real Local Program Execution (Surfpool Mainnet Fork)

Branch: `codex/harness-m3c-security` (off `main@52c0e19`; H0 commit `bd5c3b0`)
Date: 2026-11-02. All execution LOCAL on a loopback Surfpool fork. No funded
keys, no mainnet broadcast, `LIVE_TRADING_ENABLED` unset. Signing used
throwaway `Keypair::new()` wallets funded by the fork's local faucet.

## 1. Toolchain (reviewed, pinned)

- **Surfpool v1.7.0** (Solana Foundation, Apache-2.0), release published
  2026-10-28. Installed by direct release-asset download — NOT the
  `curl -sL https://run.surfpool.run/ | bash` installer (unpinned
  pipe-to-shell, rejected).
  - Asset: `surfpool-linux-x64.tar.gz` (34,588,771 bytes)
  - SHA-256: `5bef2b0b186a74622508f104b6d2e4e2db040932b6d8f3a811172e241f802a63`
  - Reproduction commands: `harness/docs/surfpool-pin.md`
  - Binary lives in gitignored `harness/bin/`.
- Fork launch (guarded by `harness/scripts/start_fork.sh`):
  `SNF_SOURCE_RPC=https://api.mainnet-beta.solana.com bash harness/scripts/start_fork.sh`
  → binds `127.0.0.1:18899` (WS 18900), `--ci` headless. Upstream RPC is used
  ONLY as the fork's account datasource; transactions are executed locally by
  the SVM. Script refuses: source==fork RPC, `LIVE_TRADING_ENABLED!=false`,
  any `SOLANA_PRIVATE_KEY`/`PRIVATE_KEY` env.
- Smoke: `getVersion` → surfnet 1.7.0 / solana-core 4.3.2 (upstream parity);
  `getSlot` → live mainnet slot; `getAccountInfo(pump.fun program)` →
  executable, owner `BPFLoaderUpgradeab1e…`, JIT-fetched. `fork_health.sh` →
  "Local fork health: ok".

## 2. Ground truth captured from production (read-only)

Recent mainnet buy via `getSignaturesForAddress`/`getTransaction` (tx
`2yCcufczQBbvccL9Mf3M3VYDimvPXMPtA38BpGvFqosTLxDw8FWTypFsg8cqVqaXzfmz2La7dkXXQ4te45JmjoKD`):

- Modern pump.fun coins are **Token-2022 mints**; production buys CPI
  Token-2022 `TransferChecked`.
- Emitted event discriminator `bddb7fd3…` == `event:TradeEvent` (our M2
  decoder's target — confirmed live).
- Full account set + PDA addresses captured and embedded as test vectors
  (mint `Ah7ghb…upump`, curve `7sTXzyg…`, vault `6JSaxG…`, user vol-acc
  `FGFrX2…`, global vol-acc `Hq2wp8…`).
- A router program (`MAyhSmz…`) wraps production buys; its inner 6EF8 CPI
  uses discriminator `8b8dd6794046280e` with an 18-account layout that
  matches NO instruction in the public IDL — open question Q-3 below.

## 3. Differentials: docs/oracle vs official IDL vs deployed program

Official IDLs: `pump-fun/pump-public-docs@639b87f` (`idl/pump.json`,
`idl/pump_amm.json` — repo HEAD, refreshed 2026-11-02).

| # | Finding | Source | Fix |
|---|---------|--------|-----|
| D-1 | Buy family (`buy`, `buy_v2`, `buy_v3`, `buy_exact_quote_in_v2/v3`, `buy_exact_sol_in`) takes a trailing `partial_fill: bool`; our builders serialized only two u64s | IDL args | `buy_*` builders now take + encode `partial_fill` (sells stay 2×u64; cloned sell layouts truncate the bool) |
| D-2 | `associated_quote_bonding_curve`, `associated_quote_user`, `buyback_fee_recipient` must be WRITABLE; `user` must be writable+signer; PumpSwap `buyback_fee_recipient` is the wallet, writable | IDL account flags | All metas corrected in `builders.rs`; first violations surfaced as on-fork anchor errors `ConstraintMut 2000` before the fix |
| D-3 | `buy_v3` does NOT create `associated_base_user` — pre-existing Token-2022 ATA required | Deployed program (anchor `AccountNotInitialized 3012` on fork) | Tests prepend `AssociatedTokenProgram.CreateIdempotent` (data `[1]`) |
| D-4 | Trade SOL legs are routed to a per-user `user_volume_accumulator` PDA (fresh account funded with the buy's SOL leg, 1,346,200 lamports observed), not the wallet; sell proceeds likewise accrue there (wallet net = −network fee) | Deployed program on fork (balance deltas) | Documented; asserted in tests; H2 paper model must account for the accumulator claim path |
| D-5 | Quote-era TradeEvent carries `quote_mint` (SystemProgram pubkey = native-SOL sentinel); observed `sol_amount`/`quote_amount`/`fee` are tiny sentinel values (1/0), NOT the lamport legs | Deployed program on fork (raw frame hex in test output) | Tests assert token/P&L accounting on chain balances; event-field semantics = open question Q-2 |

## 4. Executed matrix (real program, local fork)

Command: `SNF_FORK_RPC=http://127.0.0.1:18899 cargo test -p sniper-protocols
--test fork_execution -- --ignored --nocapture` → **3 passed, 0 failed**
(plus always-on `pda_derivations_match_live_mainnet_production_tx` in the
default suite).

| Case | Result | Evidence |
|------|--------|----------|
| `buy_v3` 100,000 tokens, partial_fill=false | PASS — `Instruction: BuyV3` executed by real ELF | Token-2022 TransferChecked curve-vault→user; user ATA +100,000, curve vault −100,000; wallet −2,865,043 lamports = 5,000 fee + 1,513,920 rent + 1,346,123 curve leg; CU 70,661/73,661 (runs vary); TradeEvent decoded by our M2 decoder (mint/is_buy/token_amount exact) |
| `sell_v3` round trip | PASS — `Instruction: SellV3` | Position flattened exactly (ATA −100,000, curve vault +100,000); wallet net −5,000 (fee only; proceeds to volume accumulator per D-4); CU 45,066 |
| Swapped-accounts negative control | PASS (expected rejection) | `InstructionError(1, Custom(3007))` — program validates metas; execution is genuine, not a stub |
| PDA correctness vs live mainnet | PASS (default suite) | bonding-curve, assoc curve vault, user ATA, user/global volume accumulators all reproduce production addresses |

Structural-only (NOT counted as execution): builder shape tests,
`required_accounts()` counts, discriminator vectors — all remain in the
default suite and now also pin the IDL (`tests/idl_conformance.rs`, 6 tests).

Not yet executed (honest gaps): `buy_exact_quote_in_v3` on-fork run, V2
cashback fallback (needs creator-vault extras), PumpSwap `sell_v2` on-fork
run (needs a migrated pool state), migration-edge cases, Token-2022
transfer-hook mints. Builder layouts for these are IDL-pinned but
execution-verified only for the V3 pair + negative control above.

## 5. Surfpool vs LiteSVM (comparison, per user decision "Both")

- Surfpool v1.7.0 is itself built on the low-level solana-svm API **via the
  LiteSVM wrapper** (per its README) — same execution core as our planned
  fallback, plus an RPC/WS server, JIT mainnet account fetching (no snapshot
  download), and IaC tooling. Boot ≈1s on this 7.6 GiB WSL2 box.
- Surfpool-first delivered the full matrix above with zero fixture
  management: accounts (program, curve, vaults, fee configs) were fetched
  on demand from a read-only public RPC.
- Raw-LiteSVM fallback (dev-dep `litesvm =0.7.0`, registry-cached): NOT yet
  wired as a second driver. Surfpool already covers local execution; the
  LiteSVM path remains the documented fallback if Surfpool becomes
  unavailable, and would additionally give in-process (no-RPC) execution for
  CI. Deferred — not a blocker for the vertical slice.

## 6. Open questions

- **Q-1** Router-era instruction: 18-account CPI with discriminator
  `8b8dd6794046280e` matches no public-IDL instruction. Likely a newer
  deployed entry (or router-private). Our direct `buy_v3`/`sell_v3` path is
  confirmed live on the deployed program, so this does not block us.
- **Q-2** Quote-era `TradeEvent.sol_amount/quote_amount/fee` semantics
  (observed sentinels 1/0 while balances moved ~1.35M lamports). Need an
  IDL type-version check or on-chain IDL diff before relying on those
  fields in P&L math.
- **Q-3** `user_volume_accumulator` claim flow
  (`close_user_volume_accumulator`/`claim_cashback`) untested — affects
  realized-P&L timing for H2/H3 models.

## 7. Gates

- `cargo fmt --all -- --check` CLEAN
- `cargo clippy --all-targets -- -D warnings` CLEAN; `--all-features` CLEAN
- `cargo test` 151 passed / 0 failed; `--all-features` 151 / 0
  (H0 baseline 144; +6 `idl_conformance`, +1 PDA ground-truth; 3 fork tests
  are `#[ignore]`d opt-in and excluded from the default count)
- `cargo deny check advisories licenses sources` unchanged from H0 (green)

Files changed: `crates/sniper-protocols/src/builders.rs` (partial_fill args,
IDL mutability metas, PumpSwap buyback slot), `crates/sniper-protocols/src/pda.rs`
(unchanged; now live-pinned by test), `crates/sniper-protocols/Cargo.toml`
(+dev-dep solana-client), `crates/sniper-protocols/tests/idl_conformance.rs`
(new), `crates/sniper-protocols/tests/fork_execution.rs` (new),
`harness/scripts/start_fork.sh` (pinned-binary resolution),
`harness/docs/surfpool-pin.md` (new), `.gitignore` (+`/harness/bin/`).
