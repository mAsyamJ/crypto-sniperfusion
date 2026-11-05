# Wallet Alpha — million Feature Register (adopt / adapt / reject)

Source: `upstream/million` @ `c89ad07c` (MIT). Every decision below is
behavioral reimplementation or documented rejection — no code copied.

## Adopted (this repo, reimplemented)

| million module | SniperFusion home | Notes |
|---|---|---|
| discovery `INFRA_ADDRESSES` + venue-vault exclusion + quote mints | `harness/wallet_roster.py` | facts adopted with attribution |
| `find-infra` cross-token-top-buyer heuristic | `wallet_roster.flag_probable_infrastructure` | >=3 unrelated mints = bot |
| `metrics.ts` `HIGH_WINRATE_SUS` concept | noted for WA4 ranking | win rate > 0.9 w/ >=20 trades = suspicious |
| copyability design LIMITATIONS (fixed 60s candle, 6 trades) | rejected-by-design → WA3 multi-delay 500 ms–60 s follower | leader price structurally unreachable |
| `owners.service` owner graph (funding/fee-payer/co-entry, union-find, co-entry-never-bridges, hub>6 exclusion) | `harness/wallet_signals.py` | owner-dedup underpins all consensus counts |
| `opportunities` consensus + net-flow gate + ladder (cumulative SOL) | `harness/wallet_signals.py` | breadth ≠ agreement; sell disqualifies ladder |
| `ledger.ts` rotation basis carry / dust close / no-proceeds moves | oracle worked-example tests (`crates-sniper-strategy/src/oracle.rs`) | PF1 oracle stays the single ledger |
| watchlist/banned UX (handi-cat concept via million parity) | `harness/wallet_watch.py` | offline alert ledger |

## Adapt-pending (needs operator decision or data)

| million module | Blocker |
|---|---|
| `consensus.service` conviction snapshots (+6/24/72h vs random control) | WA4: needs a live cohort to snapshot; methodology noted |
| `backtest.service` exit-matrix grid + candle caching (784 combos) | WA4/WA5: needs real candle/quote feeds; design registered |
| `trading/shadow.service` counterfactual guard audit (wouldBlock attribution) | WA4: build on top of wallet_signals guards |
| crawler bounded-credit loop | WA5: orchestration once discovery feeds exist |
| flows (per-mint accumulating/distributing, owner-dedup) | needs live events; pure logic port is trivial when data lands |

## Rejected / BLOCKED (external API or unsafe)

| million module | Reason |
|---|---|
| Helius enhanced-tx fetch / WS / webhooks | paid key; operator terms review required |
| DexScreener / GeckoTerminal / Jupiter / Rugcheck / sol-price services | external APIs — goal plan §security: no third-party service without terms + key-handling + rate-limit review |
| `local.executor.ts` (keypair + Jupiter live rails) | live execution — prohibited without separate owner authorization |
| Telegram/agent UX surfaces | not research evidence |
