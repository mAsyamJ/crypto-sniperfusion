# Upstream Manifest

Last reviewed: 2026-11-02. All repositories cloned read-only under `upstream/`; their working trees are never modified and never committed to the root repository (`.gitignore` excludes `upstream/`).

## Pinned revisions

| Repository | Canonical remote | Pinned commit | Commit date | License | Role |
|---|---|---|---|---|---|
| chainstack | https://github.com/chainstacklabs/pumpfun-bonkfun-bot.git | `9e22464e2334c290944e406d266a859a1b18dbb8` ("build(deps): bump aiohttp (#241)") | 2026-11-01 | **Apache-2.0** (`LICENSE`) | Behavioral baseline + offline test oracle |
| yuto | https://github.com/yuto-kazuma/solana-sniper-bot.git | `b1af7e10da2c291d613a0f539073ddbe2dfe9a5d` ("Add polished README…") | 2026-07-14 | **NONE** — README says "personal and educational use" only; no LICENSE file, no `license` field | Design reference (clean-room only) |
| m8s | https://github.com/m8s-lab/solana-sniping-bot.git | `2a0e3bf5f070c71003e19db3ee35adff3fc0f9b7` ("fix: env template") | 2025-06-12 | **NONE** — `package.json` declares `"license": "MIT"` but no license text exists anywhere in the tree | Design reference (clean-room only) |
| mev-bot | https://github.com/bigmacman1129/solana-sniper-trading-mev-bot.git | `829f251d4e371102b406176c5bd459ebf4ac87ac` ("feat: add Jupiter routing, trailing stops, and cross-DEX arb") | 2026-08-25 | **Ms-PL** (`LICENSE.md` line 1: "Microsoft Public License (Ms-PL)"). README §License falsely claims MIT. Fork of warp.id bot (`package.json` name `warp-solana-bot`) | Design reference (clean-room only) |
| pump-docs | https://github.com/pump-fun/pump-public-docs.git | `639b87fb45c3adeffa6f7c4a1bf0cb85e392bf22` | 2026-10-13 | Docs + official IDLs (repo `.gitignore`d from root) | **Protocol source of truth** |
| million | https://github.com/Kelows/million.git | `c89ad07caaf70773d40b45943e136eadc4f946a5` | 2026-09-14 | **MIT** (`LICENSE`, (c) 2026 Kelows) | Wallet-alpha behavioral reference (discovery infra addresses, copyability design limits, metrics flags); cloned 2026-11-04 for the Wallet Alpha integration, read-only, never executed |

Upstream worktrees verified clean (`git status --short` empty) at pin time.

### Wallet-alpha additions (2026-11-04)

- **million — MIT** (pinned above, cloned under `upstream/million`): permissive; studied and behaviorally re-implemented with independent code (MIT notice here constitutes attribution for derived *behavior*, not code). Adopted FACTS: its audited `INFRA_ADDRESSES` infrastructure exclusions (Raydium AMM v4 + CPMM authorities, pump.fun fee recipient, Jupiter referral), quote-asset mints (WSOL/USDC/USDT), the `find-infra` cross-token-top-buyer bot heuristic, and the `HIGH_WINRATE_SUS` suspiciousness flag concept. Rejected/REPLACED by design: its copyability measurement (fixed 60-second candle delay + 6 recent closed trades + GeckoTerminal minute candles) — superseded by WA3's timestamped multi-delay (500 ms–60 s) follower over full holdings, per `docs/WALLET_ALPHA_GOAL_PLAN.md`.
- **handi-cat (`DracoR22/handi-cat_wallet-tracker` @ `f154c670`) — NO LICENSE located**: cloned to a temp directory OUTSIDE this repository for behavior study ONLY; **never vendored into `upstream/`, never executed, no code copied** (same treatment as yuto/m8s/mev-bot). Adopted UX *concepts*, re-implemented from scratch: per-wallet subscription add/remove with history, and a wallet blocklist constant — realized as `harness/wallet_watch.py` (offline alert ledger; no Telegram/network) and the operator blocklist file. Stars/forks measure visibility, not audit quality or licensing safety.

## License conclusions

- **chainstack — Apache-2.0**: permissive. We may study, adapt behavior, and re-implement with independent code. Apache-2.0 notices must be preserved in derived artifacts; `NOTICE` handling tracked in `docs/ADRS/0002-clean-room-licensing.md`.
- **yuto — no license**: copyright defaults to all-rights-reserved. "Personal and educational use" is not a redistribution/modification grant for our purposes. **Clean-room only**: we observe *what* it does (architecture, thresholds, failure handling) and re-implement from official protocol documentation. No source copying.
- **m8s — no license text**: an unbacked `"license": "MIT"` string in `package.json` does not satisfy MIT's requirement that the license text accompany distributions. **Clean-room only.**
- **mev-bot — Ms-PL with contradictory README**: the governing file is `LICENSE.md` (Ms-PL). Ms-PL §3(D) imposes same-license conditions on source distributions of the code itself. **Clean-room only**; the MIT claim is recorded as a defect of that repository, not a permission.

## Dependency provenance

### chainstack (Python, executed)
- `pyproject.toml`: requires-python `>=3.11`, package `pump_bot` v2.0, setuptools backend, `uv` workflow with committed `uv.lock`.
- Installed via `uv sync --frozen` on 2026-11-02 into an isolated `.venv` (not global). Key locked versions: `solana==0.40.3`, `solders==0.29.0`, `aiohttp` (bumped by pinned commit), `websockets==17.1`, `grpcio`, `protobuf`, `pyyaml==6.0.3`, `uvloop==0.22.1`, `ruff==0.16.8` (dev).
- No native build steps beyond wheels; no postinstall scripts (Python).

### yuto (Rust, NOT built)
- Single crate `solana-vntr-sniper` v0.1.0, edition 2021. **No Cargo.lock, no rust-toolchain pin.**
- Pins observed in `Cargo.toml`: solana client/sdk family `2.1.14`, anchor `0.31.0`, yellowstone-grpc-client `4.1.0`/proto `4.1.1`, spl-token-2022 `6.0.0`, tokio `1.21.2`, reqwest `0.11.27`. `teloxide` declared but unused.
- Hardcoded endpoints (observations for our own compliance matrix, not for reuse): `https://lite-api.jup.ag/swap/v1` (deprecated host), ZeroSlot `https://ny1.0slot.trade/health`, CoinGecko simple price.
- Not executed: no funded keys, and its runtime requires `COPY_TRADING_TARGET_ADDRESS` + live gRPC.

### m8s (TypeScript, NOT built)
- Node ≥ 20.18; `@solana/web3.js 1.68.2`, `anchor 0.30.1`, `@triton-one/yellowstone-grpc ^2.0.0`, `@pump-fun/pump-swap-sdk 0.1.0-beta.32`, `@raydium-io/raydium-sdk 1.3.1-beta.58`.
- Build defect: `lib/jito.ts` imports `axios`, absent from `package.json` — `tsc` build fails; their dev script (`--transpile-only`) masks it. Not installed (`node_modules` absent), therefore not executed.
- Hardcoded: `https://grpc.solanavibestation.com` (no token), deprecated Jupiter `quote-api.jup.ag/v6`, 4 Jito block engines + 8 tip accounts, `JITO_FEE=100000`.

### mev-bot (TypeScript, NOT built)
- Node + ts-node entry (`index.ts`); `@solana/web3.js ^1.89.1`, `@raydium-io/raydium-sdk ^1.3.1-beta.47`, `axios`, `pino`, `async-mutex`.
- Hardcoded: 5 Jito block-engine URLs + 8 tip accounts, `https://tx.warp.id/transaction/execute`, Jupiter `lite-api.jup.ag/swap/v1` (deprecated), `NETWORK='mainnet-beta'` unconfigurable.
- Not installed, not executed.

## Security findings (pre-execution audit, 2026-11-02)

| # | Finding | Severity | Disposition |
|---|---|---|---|
| S-1 | `upstream/m8s/.env.template` ships apparent live credentials: MongoDB Atlas URI `mongodb+srv://testuser:2RXS…@cluster0.tquvepg.mongodb.net/` and XAPI key `d4119277b5c14b8c…` (twitterapi.io) | High (for that repo's author) | **Never used, never copied** into our code/config. Recorded here only. Upstream README labels them "test" credentials; treat as compromised regardless. |
| S-2 | mev-bot README/license contradiction (MIT claim vs Ms-PL file) | Legal | Clean-room policy; documented in ADR-0002 |
| S-3 | yuto/m8s ship no license grants | Legal | Clean-room policy |
| S-4 | All four repos: no `postinstall` hooks, no `curl|sh` patterns, standard build backends | — | Verified before any execution |
| S-5 | Workspace root: no `.env`, no keypair files (`id.json`, `*.key`, `*.pem`), no funded credentials anywhere | — | Confirmed; `.gitignore` blocks them |

**Executed upstream code:** only the chainstack offline regression suite (S-4 verified first; isolated venv; no keys; no network calls that move funds). yuto/m8s/mev-bot code was read, never built or run.

## Toolchain notes

- Host rustc updated during M0 spike from 1.98.0 → **1.99.1** (stable) because agave-4.4 crates require ≥1.98.1 (see `docs/SOLANA_COMPATIBILITY.md`). 1.98.1 also installed as fallback.
- `rtk` command proxy is used for shell commands per operator policy.
