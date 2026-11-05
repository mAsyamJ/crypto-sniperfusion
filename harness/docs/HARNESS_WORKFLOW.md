# SniperFusion — Continuous Engineering and Quantitative Research Harness

**Archive status (2026-11-05):** POINT-IN-TIME — V0-era workflow; superseded by `AGENTS.md` §7.

**Objective:** Maximize measured, risk-adjusted after-cost expected return subject to strict correctness, security and reliability gates. Sniping is speculative and a fast implementation does not guarantee profits.

## Proposed command interface (NOT yet implemented)

- snf-harness doctor — repo SHA, disk/RAM/toolchains, providers, safe credential detection, visibility.
- snf-harness verify — fmt, Clippy, unit/property/fuzz, decoder differential, licenses, secret/security checks.
- snf-harness replay — deterministic historical events, state-at-decision snapshots, no look-ahead, comparable baselines.
- snf-harness fork — local Surfpool / LiteSVM real-program execution with ephemeral wallets only.
- snf-harness shadow — real-time read-only launch and price streams, estimated fills, no signer.
- snf-harness evaluate — portfolio equity/MTM, realized/unrealized P&L, fees, slippage, latency and risk false-positives.
- snf-harness compare — paired champion/challenger statistics with temporal holdouts and fixed risk budgets.
- snf-harness promote — only evidence-backed paper strategy config promotion; never enable live execution or increase capital.

These names are future interfaces; do not report them as runnable until actually implemented and CI tested.

## Continuous loop

OBSERVE → HYPOTHESIS → PRE-REGISTER ACCEPTANCE → IMPLEMENT → FORMAT/LINT/TEST → REPLAY → LOCAL FORK → SHADOW → EVALUATE → CHALLENGER COMPARISON → ACCEPT/REJECT → ARCHIVE EVIDENCE.

An experiment has immutable run ID, git SHA, IDL/program hashes, dataset hash, config hash, tool version, time window, eligible token universe, raw event/decision timestamps, fee model and results. Run evidence stays in a gitignored directory until reviewed and sanitized.

## Independent review roles

Protocol engineer validates live IDLs and local SVM account changes. Runtime engineer validates stream supervision, backfill and transaction journal. Security engineer validates permissions, authorization and signer isolation. Quant engineer validates pricing, timestamps, sample selection, risk model calibration, and P&L. QA/SRE validates crash recovery and metrics. A reviewer independently re-runs decisive tests before declaring the milestone passing.

## Critical invariant gates

- Observe/replay/paper binaries cannot broadcast, demonstrated by capability isolation and tests.
- Real execution requires non-forgeable authorization, spend and fee/tip caps, mandatory technical token screening and kill switch.
- No duplicate economic buys from transaction timeouts, restarts, concurrency or relay failovers.
- Unknown transaction states are reconciled, not optimistically resent.
- No speculative entries while a critical data gap remains unresolved; re-entry only after proven backfill and position reconciliation.
- Local Pump.fun/PumpSwap executable tests, not only instruction-shape tests.
- At least seven consecutive days of real read-only shadow observation, reporting gaps and fill uncertainty; longer out-of-sample windows before performance conclusions.
- Compare no-trade, delayed-entry, and champion strategies on identical fee models and capital.
- Treat all profitable backtests as hypotheses until walk-forward/out-of-sample validation; report uncertainty and total return including operating costs.

## Token intelligence and strategy research

Separate the hard technical safety verdict (BLOCK/WATCH/ELIGIBLE) from opportunity scoring. Verify token owner/program, mint/freeze controls, Token-2022 transfer hooks and permanent delegate, actual sell feasibility, fee config, available exit liquidity and stale data. Estimate creator initial buy accurately from trade inner instructions and token balances, excluding rent and protocol vaults; measure linked-wallet holdings with explicit uncertainty. Detect repeated launch rugs, coordinated same-slot buys and creator sell pressure relative to executable exit liquidity. Fetch social metadata asynchronously with SSRF protection; never mistake social profile existence for safety.

Strategy candidates: fresh launch, delayed momentum, bonding-curve migration, selective copy trade, creator dump avoidance, concentration filters, exit horizon/partial TP/SL and adaptive fee/tip spend. Avoid market manipulation (sandwiches/forced pump-and-dump).

## Testing matrix

Unit/property → IDL and SDK differential → real Solana SVM → Surfpool local fork → deterministic event replay → network-failure/reorg injection → real read-only shadow → out-of-sample performance. Do not confuse a current-state Surfpool fork with a historical slot-perfect fork or a simulation of real MEV auctions. Record exact commands, output, errors, chain state, account deltas and CU.

Local fork RPC must be loopback (default 127.0.0.1:18899). A separate upstream source RPC is only for read-only account loading. Use ephemeral local keys and block unauthorized network egress from signing/broadcast clients. Never send funded keys or transactions to upstream mainnet through fork tools.

## Research/reference source priorities

1. https://github.com/pump-fun/pump-public-docs — official IDLs and protocol behavior
2. https://github.com/solana-foundation/surfpool — local mainnet-state fork
3. https://github.com/sevenlabs-hq/carbon — evaluate Rust ingest/decoder benchmark before replacement
4. https://github.com/jito-labs/searcher-examples — relay integration reference; check current APIs
5. https://github.com/raydium-io/raydium-sdk-V2 — Raydium reference; review GPL obligations
6. https://github.com/MeteoraAg/dynamic-bonding-curve-sdk — Meteora DBC reference; verify separate program licenses
7. https://github.com/git-disl/MELT — research data; validate licensing and selection bias

## Experiment promotion rules

A challenger must pass code + security + fork + replay gates, avoid look-ahead and survivor bias, and outperform the champion on a withheld period after costs with an uncertainty interval. Never tune against holdout data. Failed experiments remain in evidence records. Promotion cannot activate funded trading or spend money.
