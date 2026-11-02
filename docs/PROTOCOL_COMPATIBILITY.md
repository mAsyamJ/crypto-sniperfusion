# Protocol Compatibility — Pump.fun / PumpSwap (M0 audit)

Sources (authoritative order): `upstream/pump-docs` @ `639b87f` (2026-10-13) official IDLs + instruction docs; on-chain fixtures in `upstream/chainstack/tests/regression/*.json`; Chainstack vendored IDLs (`upstream/chainstack/idl/`) as the *legacy* comparison only. **Where Chainstack's vendored IDL conflicts with pump-docs, pump-docs wins** and the delta is a required Rust-scope item (this happened — see the gap table).

## Instruction-version capability matrix — bonding curve (program `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P`)

| Instruction | Accounts | Args (order) | Official IDL | Chainstack vendored IDL | Chainstack code uses | Rust scope |
|---|---|---|---|---|---|---|
| `create` (legacy) | 14 | name, symbol, uri, creator (+3 opts: share-metadata, spl-vs-token22, mayhem flag) | ✅ | ✅ | ✅ (atomic launch v1) | decode only |
| `create_v2` | 16 +3–5 remaining (non-native quote) | full arg set; rejects `cashback=true` | ✅ | ✅ | ✅ | decode + builder |
| `buy` (legacy) | 16 | amount, max_sol_cost | ✅ | ✅ | fallback path | decode + builder (fallback) |
| `buy_exact_sol_in` | 16 | sol_in, min_tokens_out | ✅ | ✅ | — (advertised, not wired) | decode |
| `buy_v2` | 27 | amount, max_sol_cost (+cashback opts) | ✅ | ✅ | ✅ preferred buy | decode + builder |
| `buy_exact_quote_in_v2` | 27 | spendable_quote_in, min_tokens_out | ✅ | ✅ | — | decode + builder |
| `buy_v3` | **17** | amount, max_sol_cost, `partial_fill: OptionBool?` (mayhem only) | ✅ | **❌ MISSING** | ❌ | decode + builder (primary) |
| `buy_exact_quote_in_v3` | **17** | spendable_quote_in, min_tokens_out, partial_fill? | ✅ | **❌ MISSING** | ❌ | decode + builder |
| `sell` (legacy) | 14 | amount, min_sol_output | ✅ | ✅ | fallback path | decode + builder (fallback) |
| `sell_v2` | 26 | amount, min_sol_output | ✅ | ✅ | ✅ preferred sell | decode + builder |
| `sell_v3` | **17** | amount, min_sol_output | ✅ | **❌ MISSING** | ❌ | decode + builder (primary) |
| `migrate` | 25 | — | ✅ | ✅ | ✅ (event decode) | decode |
| `migrate_v2` | 27 | — | ✅ | ✅ | partial | decode |
| `multi_hop_curve_swap` | 13 | — | ✅ | **❌ MISSING** | ❌ | decode only (M6+) |
| `migrate_bonding_curve_creator`, fee/admin/quote-control set | 3–13 | — | ✅ | mostly ✅ | admin ops unused | out of scope (documented) |

**Gap finding G-1:** Chainstack's vendored `pump_fun_idl.json` (47 instructions) lacks the entire V3 trade family + `multi_hop_curve_swap` present in official `pump.json` (56 instructions). Chainstack *code* (per its own docs/tests) trades via v2 with legacy fallback. **The Rust implementation must implement V3 from the official IDL/docs — this is beyond-Chainstack scope, tracked in the capability matrix as `OFFICIAL-ONLY`.**

## Instruction-version capability matrix — PumpSwap (program `pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA`)

| Instruction | Accounts | Official IDL | Chainstack vendored IDL | Rust scope |
|---|---|---|---|---|
| `buy` / `buy_exact_quote_in` (v1) | 23 (+ remaining: protocol fee recipients, buyback wallet+ATA, volume accumulator, fee program) | ✅ | ✅ | decode + builder (fallback for cashback pools) |
| `sell` (v1) | 21 (+ remaining) | ✅ | ✅ | decode + builder (fallback) |
| `buy_v2` / `buy_exact_quote_in_v2` / `sell_v2` | **17, no remaining accounts** | ✅ | **❌ MISSING** | decode + builder (primary) |
| `multi_hop_swap` | 16 | ✅ | **❌ MISSING** | decode only (M6+) |
| `create_pool`, `deposit`, `withdraw`, boosts, sweeps | 2–18 | ✅ | mostly ✅ | decode-only subset; LP ops out of scope |

**Gap finding G-2:** same pattern as G-1 for the AMM (official 38 vs vendored 32).

## V3 semantics (curve) — `docs/instructions/TRADE_V3.md`

- Same pricing, slippage checks, and fee **amounts** as v2. Difference is fee **destination**: protocol fee accrues to `BondingCurve.protocol_fees`, creator fee to `BondingCurve.creator_fee`; paid out later via sweeps (`SweepBondingCurveFeeEvent`).
- Cashback coins: v3 fails `CashbackCoinNotSupported (6094)` → **v2 fallback mandatory** in the router.
- 17-account layout (order): global, base_mint, quote_mint, base_token_program, quote_token_program, bonding_curve, assoc_base_bonding_curve, assoc_quote_bonding_curve, user, assoc_base_user, assoc_quote_user, user_volume_accumulator (PDA `[b"user_volume_accumulator", user]`, created-if-missing, rent **0.0018444 SOL**/137 B), fee_config, buyback_fee_recipient, system_program, event_authority, program.
- `fee_config` PDA (curve): `[b"fee_config", pump_program_id]` = `8Wf5TiAheLUqBrKXeYg2JtAFFMWtKdG2BSFgqUcPVwTt`.
- `buyback_fee_recipient`: one of the 8 buyback wallets (SOL-paired: the wallet; token-paired: its quote ATA); **checked every trade even at fee=0, never created by the trade — must exist**.
- User base ATA **must pre-exist** (trade does not create it) → idempotent ATA create stays in our builder.
- Events: `TradeEvent` with `ix_name` ∈ {buy_v3, buy_exact_quote_in_v3, sell_v3}; `fee_recipient` = zero key (`111…1`) meaning fee-kept-on-curve; `creator_fee_unclaimed` field = unclaimed creator fee after trade.
- `partial_fill` `OptionBool` only affects mayhem coins (no synthetic migration there); may be omitted from the tail of ix data.

## PumpSwap V2 semantics — `docs/instructions/PUMP_SWAP_TRADE_V2.md`

- 17 accounts, no remaining accounts; protocol+creator fees stay in the pool's quote vault (`Pool.protocol_fees`, `Pool.creator_fees`); LP fee unchanged (into reserves each trade).
- Cashback pools: v2 fails `CashbackCoinNotSupported (6079)` → v1 fallback.
- `global_config` PDA `[b"global_config"]` = `ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw`; `fee_config` PDA (AMM) = `5PHirr8joyTMp9JMm6nW7hNDVyEYdkzDqazxPD7RaTjx`.
- Args: `buy_v2(base_amount_out, max_quote_amount_in)`, `buy_exact_quote_in_v2(spendable_quote_in, min_base_amount_out)`, `sell_v2(base_amount_in, min_quote_amount_out)`.

## Signed-arithmetic requirements

- **`Pool::virtual_quote_reserves` is `i128` and can be negative since 2026-09-30** (`NEGATIVE_VIRTUAL_QUOTE_RESERVES.md`). Pricing must compute `effective_quote_reserves = pool_quote_token_account.amount + virtual_quote_reserves` in `i128`, then treat as `u64` (program guarantees non-negative fit). Base reserves are the raw vault amount.
- **Synthetic migration** (`TRADE_V3.md` §Synthetic migration + `SYNTHETIC_MIGRATION.md`): an over-curve `buy_v3`/`buy_exact_quote_in_v3` takes the remainder at curve price, completes the curve, buys the rest at the would-be pool price, and the migration opens the pool where the buy stopped. `max_sol_cost`/`min_tokens_out` cover **both** parts. Only the boundary-crossing buy gets this; afterwards no curve trade until migration. Events: `TradeEvent` (curve part) → `CompleteEvent` → `PostCompleteBuyEvent` (pool part). Mayhem coins excluded (old behavior via `partial_fill`).
- Migration math fixtures: chainstack `raw_graduated_markets_from_getaccountinfo.json` + `verify_sell_after_graduation.py` / `verify_pumpswap_account_layout.py`.

## Fees (authoritative posture)

- Fee **schedules are read on-chain from the Pump Fees `fee_config` accounts** (both programs) — the official docs route all fee-rate questions there; legacy docs still mention `100 bps` to `fee_recipient` on curve `buy`/`sell`, and the research review cites a published 1.25% curve total. **Implementation rule: no hardcoded fee rates; fetch+cache `fee_config` with `as_of_slot`/TTL; fixture-validate amounts.** Cache staleness is a `WATCH`-grade fact in the risk engine.
- 24 documented fee-recipient addresses: 8 normal, 8 reserved (mayhem), 8 buyback — enumerated in `docs/FEE_RECIPIENTS.md` (upstream). Builders select: normal|reserved per mayhem flag + always one buyback.

## Quote assets (non-SOL)

- `create_v2` supports USDC and **pump-coin quotes** with nested depth: quote depth+1, `Global.max_curve_depth` gate, depth-2 enabled 2026-10-13 (`CREATE_WITH_PUMP_COIN_QUOTE.md`). Remaining accounts 17–19 (+ quote curve/pool for nested) in exact order.
- `create_v2` mints are **Token-2022** (`TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb`), 6 decimals; Token-2022 transfer-fee reward coins exist (`max_transfer_fee_bps` filter in Chainstack mirrors this).
- SOL-paired trades: pass WSOL mints/accounts; user quote ATA may be the derived address without existing for the SOL case (v3 rows 8/11 nuance).

## Rust implementation target (initial vertical slice — M2/M3)

`pump.fun create/TradeEvent → decode (logs + ix-data both v2 and v3 discriminators) → screening stub (pass-through) → simulated entry (buy_v3 + ATA + volume-accumulator accounting) → position tracking → simulated exit (sell_v3; graduation-aware PumpSwap sell_v2 fallback v1)`, all under LiteSVM with the chainstack fixtures + new sanitized fixtures, no broadcast. Byte/account-layout validation against this document's tables is an acceptance test before any signing path exists.

## SDK cross-reference

- `pump-rust-client` 0.5.0 (crates.io; docs reference 0.4.1) — requires solana-sdk `>=2.1.21,<3` → compatible with our chosen Generation 2. Evaluated at M3 as a build-helper reference; **instruction bytes we emit must validate against the official IDL tables above, not trust the crate blindly**.
