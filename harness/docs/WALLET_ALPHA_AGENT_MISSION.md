# SniperFusion — Wallet Alpha Integration Mission (WA0→WA3)

**Archive status (2026-11-05):** POINT-IN-TIME — verify against `docs/evidence/WA1-WA3.md`.

Role: Senior Rust/Solana engineer, quantitative memecoin trader, wallet intelligence researcher, deterministic execution architect and security reviewer.

Project root: `~/project/crypto-bot`. Repository: `mAsyamJ/crypto-sniperfusion`.

Read in order:
- `AGENTS.md`, `README.md`, `docs/PROJECT_STATUS.md`.
- `docs/WALLET_ALPHA_UPSTREAM_AUDIT.md`, `docs/WALLET_ALPHA_GOAL_PLAN.md`.
- `.agents/skills/wallet-alpha-solana-tracker/SKILL.md`, `docs/evidence/PF0.md`, `PF1.md`, `G0-H8-session.md`.

The already cloned five upstream folders are `chainstack`, `mev-bot`, `yuto`, `m8s`, `pump-docs`; do not re-clone or modify. Check their actual pinned SHAs. Research licensed `Kelows/million` and `DoradoDevs/solana-wallet-pnl-tracker` as wallet-intelligence candidates; refer to Handi Cat for alerts only pending license rights. Keep Solana Rust runtime as authoritative.

**Primary objective:** prove repeatable follower/creator-aware strategy edge **after** realistic entry/exit delays, all fees, sell impact and tail risk. Real historical wallet leadership is only a candidate signal. Do not promise profitable traders or copy wallets blindly.

Implement WA1: typed Rust `WalletTradeEvent` adapter for owner-verified buy/sell vs transfer, SPL/token-2022 deltas and major DEX routes; all unavailable fields fail closed, no invented holdings; fixtures and independent parse comparisons. Then WA2: bounded read-only discovered-wallet history, full NAV via existing PF1 oracle and past-only copyability ranking. Then WA3: delayed follower PaperBroker + matched leader/follower P&L V2 and negative controls; adapt to H8 preregistered research. Do not rewrite existing accounts, H7/PF1 or H8.

Before any remote provider call, use an operator-owned read-only key without printing or changing it. No paid accounts, active live signing/mainnet sends, repo-history rewrite or unapproved push/merge. Keep research agents separated from funded signer and protected holdout.

Loop: inspect HEAD/worktree → failing tests → implement minimal slice → fmt/clippy/Rust+Python tests → real local fork/replay where applicable → independent P&L oracle → latency and risk stress → evidence/STATUS → local commit/PR. Tests must actually execute; absence of data means BLOCKED, never a green simulation of live market profits.

**Immediate result:** a deterministic, no-network wallet-copyability negative-control report (new `harness/wallet_alpha_lab.py`) and tests; then extend it to actual wallet feed and Rust accounting in separate PRs. The provided synthetic example **cannot validate profitable trading**.