---
name: sniper-quant-research
description: Design point-in-time alpha experiments, ten-strategy batches, independent portfolio metrics, stress tests and statistical challenger/champion validation.
---
# Sniper Quantitative Strategy Research

Read `harness/docs/AI_STRATEGY_HARNESS_V1.md` and run `python3 -m unittest discover -s harness/tests -v`.

## Workflow
1. Preregister hypothesis and metric before opening temporal holdout. Count every tested variant; enforce creator/funding cluster separation and label-window embargo.
2. Compare S01–S10 to S00 no-trade, identical creation cohorts and initial capital, independent virtual wallets first; shared-wallet competition only as a separately designed experiment.
3. Control technical safety, costs, exits, fees, risk budget and observed event availability. Distinguish complete trade logs, unresolved inventory, failed fills and survivor bias.
4. Calculate whole-position executable-equity, drawdown, tail losses, concentration and sensitivity to pessimistic latency/fees/slippage.
5. Bootstrap by token creator/day clusters rather than individual fills; treat imprecise confidence, multiplicity and low N as INCONCLUSIVE.
6. Check negative controls (time-shuffled signals, future feature injection, missing event windows and negative open positions).

## Required deliverable
Typed experiment specification, dataset + strategy hashes, full P&L receipt, uncertainty report, negative controls and explicit `EDGE_DEMONSTRATED` or `NOT_DEMONSTRATED`.

Synthetic surrogate results from `harness/research_batch.py` are **never** evidence of actual edge or permission to promote.
