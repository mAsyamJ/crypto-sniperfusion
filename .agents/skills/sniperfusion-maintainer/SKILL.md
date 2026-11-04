---
name: sniperfusion-maintainer
description: Maintain and release the SniperFusion Rust research engine through reproducible PRs, CI, evidence, licensing, risk gates and accurate project status; use for repository maintenance and integration planning.
---
# SniperFusion Maintainer

Read `AGENTS.md`, `README.md`, `docs/AGENT_OPERATIONS.md`, `docs/PROJECT_STATUS.md` and the most recent H6/H7 evidence. Verify `git status`, HEAD and open changes before writing.

## Workflow

1. Convert requests to one small PR with testable acceptance and explicit exclusions. Do not overwrite unrelated working-tree changes or untracked reports.
2. Apply the role map; call protocol engineer, quant, trader or red team only when warranted.
3. Use current source/IDL proof; never treat old READMEs, synthetic fixtures or a passing CI as a real market result.
4. Write a failing regression test for behavioral changes. Run the exact Rust and Python checks, local SVM/fork tests when applicable, and permission/license scans.
5. Review secret leakage across commits, dependencies, CI paths, unsanitized endpoint URLs and unsafe third-party scripts. Preserve existing operator key unchanged; never display it.
6. Update status, capability map and evidence receipts with actual commands, hashes and blockers. Commit locally and propose review; no mainnet trading, unapproved push or merge.

## Deliverable

A reviewer-ready summary: objective, diff scope, executed commands/exits, evidence class, remaining risk, benchmark/regression impact, local commit and PR readiness. Never promise returns.
