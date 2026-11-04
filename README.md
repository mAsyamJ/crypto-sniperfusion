# SniperFusion — Rust Solana Memecoin Research & Execution Engine

**Solana-first, chain-agnostic long-term design.** SniperFusion is an actively developed Rust protocol/replay/strategy research project. It is **not** a demonstrated profitable sniper or an agent-controlled funded trading service.

> **Current checked milestone: H7 (2026-11-04).** M0–M3c and H0/H2/H4/H6/H7 code are recorded; the Surfpool real-program test scope is documented in M3c. H6's *code* exists but no full independently covered mainnet creation cohort has been collected. H7 has 11 strategy policies and two economics engines, but **S00 NO_TRADE beat all ten trading policies on synthetic data**. Verdict: `EDGE_NOT_DEMONSTRATED`. H3 seven-day read-only shadow is not complete. Refer to [PROJECT_STATUS](docs/PROJECT_STATUS.md), [H6 evidence](docs/evidence/H6.md), [H7 evidence](docs/evidence/H7.md) and the actual HEAD before reporting progress.

## Start here — coding agents

1. Read [AGENTS.md](AGENTS.md), [Agent Operating System](docs/AGENT_OPERATIONS.md) and [next engineering goals](docs/NEXT_GOAL_H8_G0.md).
2. Inspect specialist skills under [.agents/skills](.agents/skills); these are task-specific markdown playbooks, not installed external powers.
3. Run non-mutating self-inspection and optional synthetic demonstration:

```bash
cd ~/project/crypto-bot
git pull --ff-only origin main
python3 harness/agentctl.py doctor
python3 harness/agentctl.py skills
python3 harness/agentctl.py commands
python3 harness/agentctl.py lint-skills
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
- `.agents/skills/`: expert playbooks; `docs/`: architecture, integration and evidence.

## Next objective

**Unlock G0 data first:** provider capability probe → bounded, block-verified `COMPLETE` creation cohort → actual fee/reserve snapshots → Engine A/B comparison → bounded H8 research council → genuinely prospective read-only shadow. See [H8/G0 plan](docs/NEXT_GOAL_H8_G0.md) and [upstream integration map](docs/UPSTREAM_INTEGRATION_MATRIX.md).

No successful synthetic experiment, testnet faucet trade, or Surfpool fork is evidence of mainnet profitability. Mainnet operations remain read-only; no automated funding or signing is authorized.

## Protocol truth and licensing

Source of truth: [Pump.fun official IDLs](https://github.com/pump-fun/pump-public-docs) and verified on-chain account/program state. Chainstack is the Apache-2.0 pinned reference baseline; yuto/m8s are source-available without an explicit license in the pinned versions; mev-bot is Ms-PL. Use clean-room integration rules in [ADR-0002](docs/ADRS/0002-clean-room-licensing.md).

Official Solana agent workflow: [Solana Agent Skills](https://solana.com/skills) and [Coding with agents](https://solana.com/docs/intro/coding-with-agents). Review source and versions before optionally installing: `npx skills add https://github.com/solana-foundation/solana-dev-skill`.

The repository is public; never commit keys, API URLs containing tokens, signing material, private datasets or market-data agreements. Existing operator RPC keys are *not* changed by this project documentation.

## License

The SniperFusion repository has no general redistribution license granted beyond any explicit files. See individual upstream dependencies for their applicable licenses.
