# SniperFusion — Rust Solana Memecoin Research & Execution Engine

**Solana-first, chain-agnostic long-term design.** SniperFusion is an actively developed Rust protocol/replay/strategy research project. It is **not** a demonstrated profitable sniper or an agent-controlled funded trading service.

> **Verified state at D1a (2026-11-06):** identical 3,001-slot, block-verified COMPLETE mainnet read-only cohort re-captured with the fixed `create_v2` decoder: **232 launches** (181 SOL-quoted, 51 alternative-quoted), not zero. Source: [D1a receipt](docs/evidence/D1A-TIME-OF-DAY-CREATE-V2-20261106.md). Trading on this cohort is NOT yet priced/evaluated; D2 as-of reserves/fees → PR2 real-cohort historical paper execution remain first. H8 and PF1 tooling exist, but **S00 NO_TRADE remains champion (`EDGE_NOT_DEMONSTRATED`)**. No proven profitable strategy, funded mainnet trading or completed ≥7-day prospective shadow.

## Start here — coding agents

1. **One active agent entrypoint:** read [SOUL.md](SOUL.md) → [AGENTS.md](AGENTS.md) → [SESSION_LAUNCHER_PROMPT.md](SESSION_LAUNCHER_PROMPT.md) → [GOAL_PLAN_PROMPT.md](GOAL_PLAN_PROMPT.md) → [docs/GOALS.md](docs/GOALS.md) and the [current acceptance contract](docs/NEXT_GOAL_24H_FILTERED_PAPER_RESEARCH.md). Use [DOC_INDEX](docs/DOC_INDEX.md) for supporting specs and history. Old dashboard and H5–H8 prompts are HISTORICAL, not alternate instructions.
2. Inspect specialist skills under [.agents/skills](.agents/skills); these are task-specific markdown playbooks, not installed external powers.
3. Run non-mutating self-inspection and optional synthetic demonstration:

```bash
cd ~/project/crypto-bot
git pull --ff-only origin main
python3 harness/agentctl.py doctor
python3 harness/agentctl.py skills
python3 harness/agentctl.py commands
python3 harness/agentctl.py lint-skills
# Optional operator-owned read-only RPC capability check (no wallet):
# python3 harness/rpc_capability_probe.py --out harness/runs/rpc-sample-unique.json
python3 -m unittest discover -s harness/tests -v
python3 harness/agentctl.py research-demo --cycles 3 --out harness/runs/agent-demo-v1.json
```

The demo is **synthetic only**, non-promotable and uses no remote RPC or key. `--out` must be a *new* path; write-once receipts must not be overwritten.

## Existing tools (use the implementation, not imagined commands)

| Tool | Available now | Role |
|---|---|---|
| `python3 harness/agentctl.py` | Agent dispatch helper | Skill registry, run instructions, synthetic demonstration |
| `python3 harness/agent_mission.py plan` | Legacy mission planner | Synthetic-only; some status strings predate H6/H7 |
| `python3 harness/agent_mission.py run ...` | Synthetic tests | Bounded 1–20 cycles; cannot sign or broadcast |
| `python3 harness/launch_cohort_capture.py --help` | Read-only Mainnet collector | Requires a provider with honest `getBlock` coverage |
| `python3 harness/snf-harness --help` | Harness CLI | Doctor/verify/replay/fork/shadow/evaluate/compare/promote; shadow is not started and promotion is paper-only |
| `cargo run -p sniper-app -- --help` | Rust runtime CLI | Actual supported modes visible via CLI |
| `cargo test --workspace --locked` | Rust checks | Run on dedicated dev host; RAM cap `CARGO_BUILD_JOBS=2` |

## Project map

- `crates/sniper-core`: typed chain events, provenance, coverage, authorization.
- `crates/sniper-protocols`: Pump.fun/PumpSwap decoding, PDAs, V2/V3 math/instruction builders.
- `crates/sniper-runtime`: stream adapters, journal, lifecycle, paper broker, read-only RPC probe.
- `crates/sniper-strategy`: S00–S10 policies, Engine A/B, independent accounting and evaluator.
- `crates/sniper-app`: Rust CLI, Devnet pilot isolated from funded mainnet.
- `harness/`: research tools, collection, run manifests, synthetic fixtures, tests.
- `.agents/skills/`: expert playbooks; `docs/`: indexed in [DOC_INDEX](docs/DOC_INDEX.md) — architecture, evidence, [glossary](docs/GLOSSARY.md), [strategy registry](docs/STRATEGY_REGISTRY.md), [provider matrix](docs/RPC_PROVIDERS.md).

## Next objective

**G0 launch capture is COMPLETE.** The canonical next goal is **D2A/D2B** actual point-in-time reserves and fees → **PR2** Engine A/B + independent PF1 net liquidation accounting on the 232 real launches → universal G1–G7 filters → vetted additional on-chain/API token reach → no-AI 24h paper daemon → preregistered strategy/Harness² evaluation → ≥7-day prospective shadow. See [current goals](docs/GOALS.md) and [one active goal plan](GOAL_PLAN_PROMPT.md). Prior G0 plans are retained for historical context only.

No successful synthetic experiment, testnet faucet trade, or Surfpool fork is evidence of mainnet profitability. Mainnet operations remain read-only; no automated funding or signing is authorized.

## Protocol truth and licensing

Source of truth: [Pump.fun official IDLs](https://github.com/pump-fun/pump-public-docs) and verified on-chain account/program state. Chainstack is the Apache-2.0 pinned reference baseline; yuto/m8s are source-available without an explicit license in the pinned versions; mev-bot is Ms-PL. Use clean-room integration rules in [ADR-0002](docs/ADRS/0002-clean-room-licensing.md).

Official Solana agent workflow: [Solana Agent Skills](https://solana.com/skills) and [Coding with agents](https://solana.com/docs/intro/coding-with-agents). Review source and versions before optionally installing: `npx skills add https://github.com/solana-foundation/solana-dev-skill`.

The repository is public; never commit keys, API URLs containing tokens, signing material, private datasets or market-data agreements. Existing operator RPC keys are *not* changed by this project documentation.

## License

The SniperFusion repository has no general redistribution license granted beyond any explicit files. See individual upstream dependencies for their applicable licenses.

## Safe Mainnet RPC block-capability probe

If your existing operator-owned `SNF_MAINNET_RPC_URL` is already set in your shell, run `python3 harness/rpc_capability_probe.py --out harness/runs/rpc-sample-unique.json` to check Mainnet genesis, a finalized slot, and one recent full `getBlock` sample within five read-only requests. The receipt never includes the URL/key. This probes **provider compatibility only**; success does not establish G0 coverage or strategy profitability. Do not paste the RPC URL/key into a prompt or public log. The key is never modified by this tool.

## Profit-first H8 and P&L evaluation

Business objective: **prove repeatable, risk-constrained net trading profit after all fees and realistic execution**, not maximize trade count or win rate. Current H7 synthetic evidence remains `EDGE_NOT_DEMONSTRATED` and S00 NO_TRADE dominates. For current work invoke the root [goal plan](GOAL_PLAN_PROMPT.md); [Profit-First Plan](docs/PROFIT_FIRST_GOAL_PLAN.md) and [Profit-First Agent Mission](harness/docs/PROFIT_FIRST_AGENT_MISSION.md) remain historical financial-design references. Dispatch `.agents/skills/sniper-pnl-architect/SKILL.md` for P&L review; PF1 V2 accounting code already exists, but real-cohort paper P&L is not yet established. Retain mainnet read-only and prohibit independent funded agents.

## Wallet Alpha — existing specialized research (not a second current mission)

[Wallet Alpha Goal Plan](docs/WALLET_ALPHA_GOAL_PLAN.md), [upstream audit](docs/WALLET_ALPHA_UPSTREAM_AUDIT.md) and [historical agent mission](harness/docs/WALLET_ALPHA_AGENT_MISSION.md) are specialized references. W01 naive follow was rejected; S14 provenance-filtered follow remains hypothesis only. Route current work through the root session launcher and goal plan; use `.agents/skills/wallet-alpha-solana-tracker/SKILL.md` for this research family.

`python3 harness/wallet_alpha_lab.py --events harness/fixtures/wallet_alpha_synthetic.jsonl --min-trades 2` runs a strictly offline, **synthetic** follower-copyability negative control. It is NOT a production wallet tracker, live shadow bot or shared-capital P&L report; it does not find or follow mainnet wallets. P&L V2 in Rust remains the authoritative portfolio accountant for future WA1–WA3 integration.

Do not equate GitHub stars with trading profitability. The highest-engagement wallet alert tracker reviewed, Handi Cat, lacks an explicit license grant; the licensed architectural reference is `Kelows/million` (MIT) with a separate balance-delta/FIFO control from `DoradoDevs` (MIT). Research must distinguish leader returns from executable follower returns after latency and every fee.
