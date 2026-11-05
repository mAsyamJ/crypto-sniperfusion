# NEXT ENGINEERING MISSION — SniperFusion H0–H4

**Archive status (2026-11-05):** HISTORICAL — superseded by `docs/NEXT_GOAL_H8_G0.md`.

Role: Principal Solana/Rust trading-systems engineer, quantitative researcher, CI and security lead.
Project root: ~/project/crypto-bot
Canonical repository: mAsyamJ/crypto-sniperfusion

## First inspect, never assume

Read AGENTS.md, docs/history/AGENT_MASTER_PROMPT.md, docs/PROJECT_STATUS.md, docs/evidence/M3.md and the entire harness/ directory. Inspect actual HEAD and worktree. External audit baseline at 2d51a32b4a480618483e2018d9f234d0906594e0 found four Rust crates, offline M3 maths/builders and mock Geyser tests, with M3c real-program local execution not completed. Existing CLI paper mode uses CountingHook with Lamports::ZERO, not a true portfolio simulator. RiskAuthorization is publicly constructible/serializable with unchecked slot addition. The JSONL journal flush does not guarantee fsync. Gap recovery auto-resumes on later contiguous observations without proving backfill. GitHub reported PUBLIC visibility while docs claimed private.

Verify each fact again against the current working tree before changing it. Preserve the four crates, milestones, commits and existing tests.

## Non-negotiable controls

- Never use funded private keys, real mainnet broadcasts, paid APIs or live trading without specific owner authorization.
- Do not push, force-push, merge or change repository visibility without explicit new authorization.
- Work on a clean local feature branch, for example codex/harness-m3c-security.
- Do not copy unlicensed third-party bot source. Use official IDLs and validated protocol behavior.
- No claimed PASS based on skipped, mock-only or synthetic results for a real-execution requirement.
- Observer/replay/paper code cannot construct a mainnet broadcaster or funded signer.

## H0 — security and correctness FIRST

1. Build a red/green test for forgeable RiskAuthorization; implement private/non-serializable execution capability with a trusted issuance boundary. Bind chain, wallet, quote mint, trade size, CU price and limit, total network fee in lamports, tip, slippage, validity slot, nonce, risk evidence and remaining budget. Use checked arithmetic and nonce rollover tests.
2. Journal: single writer, durable write-ahead before external submission, sync_all, crash-tail recovery only for a truly final partial record, sequence validation/checksums and injected crash/restart tests.
3. Gap recovery: explicit missing-slot backfill and reconciled account/position state before entry resumes. Root/slot source for continuous slot coverage, not transaction event-only heuristics.
4. CI: history-aware secret scanning, dependency vulnerabilities and license checks. Keep all tests no-broadcast and no-secrets.
5. Correct source status documentation and visibility contradiction after verifying the live repo setting.
6. Collect exact evidence and commit locally.

## H1 — M3c actual program execution on isolated fork

1. Inspect official Surfpool / LiteSVM docs and version compatibility; install only with reviewed, pinned source/install method.
2. Launch localhost-only Surfpool mainnet-state fork, with upstream RPC for account fetching only, not transaction submission.
3. Use ephemeral test keys, cloned deployed programs and representative account state. Test locally executed Pump.fun V3 (buy_v3, buy_exact_quote_in_v3, sell_v3), V2 cashback fallback, PumpSwap V2 and migration edge cases.
4. Verify real local account token/SOL deltas, program logs, CU, rent/fee accounting and failure cases. Compare official program behavior and versioned IDL data.
5. Record exact snapshots/commits/commands/outputs in docs/evidence/M3c.md. If an owner-only account or missing executable blocks the test, document blocker and continue independent work; do not manufacture success.

## H2 — real Rust paper broker

1. Wire existing paper_buy / paper_sell into an event-sourced PaperBroker. Replace CountingHook-only CLI path; a reported buy must have a non-zero position when fill criteria pass.
2. Persist virtual orders/fills, wallet cash, token balances and positions by chain/mint/venue/quote. Include network fee/tip, slippage, price impact, fee config, confirmation delay, mark-to-executable-exit price, realized/unrealized P&L, max drawdown.
3. Reject fills when market data is stale, liquidity cannot support a sale, or features were not observed before the decision. Include no-fill and uncertainty scenarios.
4. Export deterministic JSONL completed trade records that harness/scripts/evaluate_paper.py can assess; produce in-Rust canonical portfolio MTM accounting.
5. Tests: profit, cost-adjusted loss, unfilled order, migration mid-position, process crash with unresolved position, data lookahead, deterministic reruns.
6. Paper command must actually generate fills and a reconciled P&L receipt, not merely intents.

## H3 — live-market read-only shadow trading

1. Connect WebSocket streams read-only; use Yellowstone only when provider access exists. Backfill gaps by actual RPC, prove data completeness. Track receive/decision/quote/fill timestamps and protocol versions.
2. Record all token launches, not just successful migrated coins; maintain point-in-time features and strict temporal split.
3. Add creator first-buy, related wallet concentration and creator selling signals; Token-2022 technical risk controls; sell-feasibility and current executable exit depth. Separate technical BLOCK from empirical trading opportunity scores.
4. Evaluate no-trade and delayed-entry baselines over the same market sessions and capital assumptions. Observe for at least seven days before initial generalization; longer independent holdout required for performance claims.
5. Report fill model uncertainty, p50/p95/p99 latencies, data gaps, fee/tip costs and full portfolio exposure. Do not claim profitability from synthetic fixtures.

## H4 — full improvement harness

Develop a safe CLI with doctor, verify, replay, fork, shadow, evaluate, compare, promote. Persist immutable run manifests including Git SHA, account/program hashes, input dataset and strategy config hashes, timestamps, commands, exact exit codes, signal statistics and uncertainty.

Promote paper strategy configurations only when correctness, security, shadow and out-of-sample gates pass. Promotion MUST NOT enable live trading.

Research Carbon performance before replacing custom decoders; investigate official Jito integration only after H0–H2; add Raydium and Meteora one venue at a time with executed buy/sell fixtures and license review.

## Development and evidence loop

INSPECT → WRITE CONTRACT/FAILING TEST → MINIMAL IMPLEMENTATION → FORMAT/CLIPPY/TEST → SVM/FORK → REPLAY → FAULT INJECTION → SHADOW → BENCHMARK/QUANT REVIEW → REVIEW DIFF/SECURITY → EVIDENCE → LOCAL COMMIT.

For each milestone report files changed, actual commands/exit codes, PASS/FAIL/SKIPPED clearly, failures and fixes, achieved latency/correctness metrics, commit SHA and unresolved risks. Do not stop at planning if testable safe implementation is possible.

## Immediate deliverable

Implement H0 security fixes, demonstrate the local Surfpool smoke test, then ONE real local Pump.fun buy_v3 transaction against an executable program and a reproducible Rust paper buy→sell with portfolio accounting. If local fork execution is blocked, record the evidence and complete H0/H2 offline. Do not begin broader DEX or MEV optimization until the validated vertical slice is complete.
