# Capability Matrix

Legend — per upstream: `VERIFIED` (source-verified working), `STUB` (present but non-functional), `ADVERTISED-NOT-IMPL` (claimed, absent), `OUTDATED` (wrong vs current protocol), `DEAD` (unwired code). Per Chainstack parity row: `PASS` / `FAIL` / `PLANNED(M{n})` / `EXCLUDED(scope-reason)`.

## 1. Chainstack baseline (parity target) — `9e22464`

| # | Capability | Upstream evidence | Rust parity status |
|---|---|---|---|
| C1 | pump.fun create-event parsing + instruction fallback | `event_parser.py` (988 L); logs+ix decode | **DECODER PARITY PASS (M2a)** — event decode + log attribution differential 320/320; instruction-data decode lands M3 |
| C2 | Logs listener (WSS logsSubscribe, processed, err-gate) | `universal_logs_listener.py` | PLANNED(M2) |
| C3 | Blocks listener (blockSubscribe, confirmed, v1 envelope, null-guard) | `universal_block_listener.py` | PLANNED(M2) |
| C4 | Geyser listener (Yellowstone Subscribe, PROCESSED, failed=False) | `universal_geyser_listener.py` | PLANNED(M2) |
| C5 | Shreds listener (Geyser `SubscribeDeshred`, pre-exec, ALT-resolve, no router-CPI creates) | `universal_shreds_listener.py` + 2 fixtures | PLANNED(M2) — capability-detect + fallback |
| C6 | Curve buy/sell builders: v2 preferred, legacy fallback, exact-token-out semantics | `pumpfun/instruction_builder.py` | PLANNED(M3) |
| C7 | **V3 trade family (buy_v3/buy_exact_quote_in_v3/sell_v3)** | **ABSENT upstream (G-1 gap)** — official-only | PLANNED(M3) `OFFICIAL-ONLY` |
| C8 | PumpSwap graduation-aware sell (v1 23/21-acct + fees) | `pumpswap_market.py` | PLANNED(M3/M4) |
| C9 | **PumpSwap v2 trades (17-acct)** | **ABSENT upstream (G-2 gap)** | PLANNED(M3/M4) `OFFICIAL-ONLY` |
| C10 | Synthetic migration + negative virtual reserves (i128) | partial (post-graduation sell); math gaps | PLANNED(M3) |
| C11 | Priority fees: dynamic p70 of recent fees × extra% clamped by hard_cap; fixed mode; mutually exclusive | `core/priority_fee/` | PLANNED(M3) |
| C12 | Compute-budget limits buy=100k/sell=60k (+account-data-size) | builders + config | PLANNED(M3) |
| C13 | Extreme-fast zero-RPC buy (fixed token amount, trust_create_event) | `platform_aware.py:216-231`; verified by `verify_extreme_fast_zero_rpc.py` | PLANNED(M4, config parity M1) |
| C14 | Curve refresh budget (processed-commitment, slot-consistent batch read) | `platform_aware.py:42-97` | PLANNED(M4) |
| C15 | Quote-asset handling: aliases, per-mint amounts, decimals/token-program cache warm, unconfigured-quote skip | `core/pubkeys.py`, trader | PLANNED(M4) |
| C16 | LetsBonk (LaunchLab subclass, program `5thqcDwKp5QQ8US4XRMoseGeGbmLKMmoKZmS6zHrQAsA`) | `letsbonk/__init__.py` | PLANNED(M4→M6 venue work) |
| C17 | StonkFun (LaunchLab subclass, 2 platform configs) | `stonkfun/` + 3 fixtures | PLANNED(M4→M6) |
| C18 | LaunchLab shared base: buy_exact_in/sell_exact_in, throwaway WSOL settlement, SHARE_FEE_RATE=0 | `launchlab/` | PLANNED(M6) |
| C19 | Exit strategies: time_based / tp_sl (Position TP/SL+max_hold, dead-feed time exit) / manual; marry/yolo | `universal_trader.py`, `position.py` | PLANNED(M3 state machine, M4 parity) |
| C20 | Exit-sell verdicts SOLD/LATE_SUCCESS/RETRY/STOP; max_exit_sell_attempts≥1 | `ExitSellVerdict` + `verify_exit_sell_confirmation.py` | PLANNED(M3/M7 reconciliation) |
| C21 | No-duplicate-buys / landed-never-failed / reverted-never-success | `verify_buy_result_not_lost.py`, `verify_tx_status_checks.py` | PLANNED(M3) |
| C22 | Account cleanup modes disabled/on_fail/after_sell/post_session, burn close, withheld-fee harvest, 120s shutdown budget | `cleanup/` + 2 verifiers | PLANNED(M4, simulated) |
| C23 | RPC rate limiting (token bucket 25 rps), deadline+429-backoff post_rpc, blockhash cache | `core/client.py`, `rpc_rate_limiter.py` | PLANNED(M2/M4) |
| C24 | Config: YAML + ${ENV} expansion, validation, listener/platform matrix, safe defaults | `config_loader.py`, `bots/*.yaml` | PLANNED(M1 typed config; M4 `CONFIG_MIGRATION.md`) |
| C25 | 38 offline regression verifiers green (baseline evidence) | `tests/regression/run_all.py` | oracle only |
| C26 | Multi-bot process manager, secret redaction, uvloop | `bot_runner.py` | EXCLUDED(architectural: single Rust daemon) |
| C27 | Cookbook scripts (52), tools (11 live/sim) | `cookbook/`, `tools/` | EXCLUDED(reference only) |

## 2. Yuto `b1af7e1` (design reference; no license — clean-room only)

| Claimed | Reality | Our use |
|---|---|---|
| 8 DEX adapters | **3 wired** (pump.fun, pumpswap, raydium-launchpad); 5 DEAD files, 2 of which are misnamed (`meteora_dbc.rs` contains Raydium Launchpad constants; `meteora_damm.rs` contains PumpSwap) | Multi-DEX adapter trait design |
| Copy trading | Target-wallet monitor arms sniper; sells mirrored; buys NOT mirrored until signal | Selective copy strategy (M6) — independent EV decision |
| Sniper signal | 2-slot price drop + ≥1.0 SOL incoming buy; max 3 cycles/10 trades | Signal concept (M6) |
| Risk management | **10-minute wallet-balance poll** — inadequate vs dev dumps | Replaced by event-driven DevFlowMonitor (M5) |
| Jupiter | hardcoded `lite-api.jup.ag/swap/v1` — **deprecated host** | We target Swap API V2 `api.jup.ag` (M7) |
| ZeroSlot | tx-landing service (not zero-slippage); 21 hardcoded tips | Optional M7 route, terms-verified first |
| Trailing exits | tiered 1/5/10/30/100% dynamic trailing, whale/emergency, Jupiter fallback | Exit-engine design (M3/M4) |
| Recovery | gRPC retry ×3; **stream reconnect is a TODO**; sell retry ×3 | We must do better (M2 reconnect, M7 reconcile) |
| Blockhash | 300 ms refresher, 10 s staleness | Blockhash cache design (M2) |
| Stack | solana 2.1.14, yellowstone 4.1, no lockfile | Validates Gen-2 choice |

## 3. m8s `2a0e3bf` (design reference; no license — clean-room only)

| Claimed | Reality | Our use |
|---|---|---|
| Honeypot/metadata check | `check-metadata.ts` = socials presence + 4-handle X blacklist. **No sellability/authority/liquidity checks** | We implement real checks (M5); socials = reputation signal only |
| Launch sniping | **buy is commented out** (`handle-create.ts:68-71`) | — |
| Migration trading | PumpSwap buy after migrate gated by `TARGET_WALLETS=[]` (disabled by default); **sell unfinished** | — |
| Creator initial buy | fee-payer balance delta − fixed 0.02 SOL, clamped 0.3–15 SOL; logged, never gated | Replaced by exact ix-level computation, fee/rent-separated (M5) |
| Holder metrics | top-20 via getTokenLargestAccounts, vault-excluded — computed, logged, **not gated** | Concentration checks with thresholds (M5) |
| 4 venues subscribed | pump.fun, pump.fun→PumpSwap migrate, LaunchLab(bonk-suffix), Believe/Meteora-DBC | Venue detection set (M2/M6) |
| Cron watch-pools | per-minute vault TP check; **sell stubbed** | — |
| Jupiter v6 `quote-api.jup.ag` | **dead endpoint** | — |
| Build | `tsc` fails (axios missing from deps) | — |

## 4. mev-bot `829f251` (design reference; Ms-PL — clean-room only)

| Claimed | Reality | Our use |
|---|---|---|
| Jito bundles | tip-tx + swap-tx to 5 block engines; **no getBundleStatuses** — rejection indistinguishable; 8 static tips; fixed fee | We add bundle-status lifecycle + reconciliation (M7) |
| Circuit breaker | global consecutive-exception counter (4 fails → 5 min pause). No PnL/drawdown/stale-data/per-mint isolation; unconfirmed txs never counted | Failure-classified, per-scope breaker (M7) |
| Position book | in-memory Map; buy cooldown 60 s; sell locks; single scale-out (Raydium only) | Persistent journal + locks + laddered exits (M3/M7) |
| Holder filter | top-1 non-LP only, 50% default | top-1/10/20 aggregate (M5) |
| pump.fun | legacy 12-acct buy; **1% fee hardcoded both sides**; no direct PumpSwap (graduation → Jupiter); curve-progress gate | We read fee_config on-chain; direct PumpSwap (M3) |
| Raydium | AMMv4/OpenBook only | CPMM/CLMM later (M6) |
| Caps | MAX_OPEN_POSITIONS=3, daily buy budgets | Budget framework (M1 config, M7 enforcement) |

## 5. Safety invariants enforced in Rust regardless of upstream behavior

- `LIVE_TRADING_ENABLED=false` default **plus** four independent live gates (build feature, operator token, validated budget, kill-switch) — no upstream had defense in depth here.
- Every money-moving order requires a typed `RiskAuthorization` binding strategy/mint/venue/limits/fees/slippage/validity — no strategy or emergency path can bypass.
- Observer/replay runtimes have **no functional access** to broadcaster or signer (capability-typed, compile-time).
- Unknown transaction outcome → reconcile, never blind re-buy (all four upstreams violate this somewhere).
