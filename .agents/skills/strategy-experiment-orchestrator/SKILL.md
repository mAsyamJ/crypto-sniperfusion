---
name: strategy-experiment-orchestrator
description: Orchestrate bounded preregistered SniperFusion strategy iteration, adverse execution stress, single sealed final check, and independent falsification without live funds.
---
# Strategy Experiment Orchestrator

Read \`harness/docs/RESEARCH_CYCLE_V1.md\`, \`harness/research_batch.py\`, \`harness/research_cycle.py\`, and \`.agents/skills/independent-strategy-critic/SKILL.md\`.

## Present executable loop (SYNTHETIC ONLY)
\`\`\`bash
python3 -m unittest discover -s harness/tests -v
python3 harness/research_cycle.py --events harness/fixtures/research/synthetic_lifecycle.jsonl \
  --out harness/runs/local-3-rounds.json --cycles 3
\`\`\`

Research engine preregisters bounded liquidity and buyer parameters each cycle; compares 11 policies under three stress models, freezes the development winner, then performs one sealed toy mint-disjoint check. It is not a genuine chronological creator-cluster holdout, a Bayesian agent or proof of profitability.

## Future research agent loop
1. Evidence curator validates all launch events, nontrading mints, missing slots and dual observation/chain times.
2. Quant proposes one falsifiable improvement with expected mechanism and primary metric.
3. Protocol expert derives version-correct fees, sellability and migration conditions.
4. Security expert vetoes unsafe token/session/credential controls.
5. Engineer writes a small deterministic Rust feature/strategy patch, pinned by SHA.
6. Independent critic reconstructs P&L from raw account deltas, runs negative controls and adversarial fills.
7. Deterministic non-AI gate accepts only a candidate for **paper-only** testing after real untouched holdout, local fork and >=7-day read-only shadow.
8. Archive failed hypotheses and counts before starting a fresh truly independent experiment.

AI may not run \`devnet-pump-pilot --submit\` without explicit separate Devnet test authorization, may not read private keys, and may never enable \`sniper live\`. Unbounded loops are forbidden due data snooping and operational risk.
