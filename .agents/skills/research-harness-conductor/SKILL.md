---
name: research-harness-conductor
description: Orchestrate SniperFusion's offline research loop, immutable datasets, batch runs, agent hypotheses, independent review and gated paper-only promotion.
---
# SniperFusion Research Harness Conductor

Read `AGENTS.md`, `harness/docs/RESEARCH_LOOP_RUNBOOK.md`, `harness/docs/AI_STRATEGY_HARNESS_V1.md`.

## Loop
DISCOVER → VERIFY DATA → FREEZE EXPERIMENT → RUN BASELINES → RUN CANDIDATES → EVALUATE → DIAGNOSE → PROPOSE → INDEPENDENT CRITIQUE → FORK → SHADOW → PAPER-ONLY PROMOTION.

## Operators
```bash
python3 -m unittest discover -s harness/tests -v
python3 harness/research_batch.py validate --events harness/fixtures/research/synthetic_lifecycle.jsonl
python3 harness/research_batch.py batch --events harness/fixtures/research/synthetic_lifecycle.jsonl --out harness/runs/synthetic-batch.json
python3 harness/research_batch.py diagnose --report harness/runs/synthetic-batch.json
```

These commands are **synthetic research demonstrations**: the new runner does not perform historical chain replay, actual program calls, statistical promotion or live trading. Its `promotable=false` output is permanent.

## Agent boundaries
Delegate protocol research, lifecycle dataset quality, microstructure, quant evaluation and adversarial security to the specialist skills. Require an independent critic not involved in proposing the hypothesis. Reject unverifiable evidence. AI may draft only offline candidate code/JSON or a PR, never grant authorization or enable funded routes.
