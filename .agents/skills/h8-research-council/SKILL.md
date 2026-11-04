---
name: h8-research-council
description: Orchestrate finite, preregistered SniperFusion strategy research with strict development/holdout separation, independent criticism, full after-cost accounting, safety reviews and paper-only promotion.
---
# H8 Research Council and Trial Registry

Consult `docs/AGENT_OPERATIONS.md`, `docs/NEXT_GOAL_H8_G0.md`, `independent-strategy-critic` and `research-harness-conductor`. This skill is a procedural specification; real H8 one-use holdout machinery is NOT yet implemented.

## Council

Maintainer approves bounded hypothesis scope; trader proposes; data engineer certifies coverage; protocol engineer certifies quote/fee account state; quant independently evaluates; security vetoes unauthorized access; **independent critic must not be the proposer**.

## Loop (max 20 total candidate trials)

Freeze raw dataset and config hashes → preregister one falsifiable hypothesis → run S00 and challenger on *the same* valid cohort with the same capital → run negative controls, fee/latency stress, full NAV oracle and crash/fork tests → critic rejects or retains → count every trial including failed variants. Never re-open a sealed holdout to tune a strategy. Holdout is only accessible once through a durable recorded control after registration; do not claim this exists until implemented and tested.

The council must not hold signing keys, instantiate mainnet send routes, access operator RPC secrets, change paid service configuration, or approve live trading. Even success on synthetic/fork/testnet data does not establish actual profit. A genuine >=7-day read-only forward shadow and independent human review precede paper-only champion promotion.

## Deliverable

Preregistration, trial budget ledger, immutable receipts, corrected multiple-comparison tests, independent critic report, verdict EDGE_NOT_DEMONSTRATED/INCONCLUSIVE/PAPER_REVIEW and explicit proof-class/authorization boundaries.
