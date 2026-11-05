---
name: harness2-integration
description: Operate the REAL Google Harness² integration (ADR-0012) — deterministic recursion via the SniperFusion adapter, agentctl commands, H8 registry bridge, security boundaries, and the model-backed BLOCKED gate.
---

# Harness² Integration — Operating Playbook

This describes ACTUAL installed capabilities (branch
`codex/wallet-alpha-upstream-integration-20261104`, commits
`804e957`…): genuine `google-research/harness2` (Apache-2.0) pinned at
`6868817…` under `.external/harness2`, editable-installed into
`.venv-harness2` (uv, Python 3.12), zero base deps.

## Commands (verified)

```bash
bash scripts/install-harness2.sh        # idempotent install/verify
python3 harness/agentctl.py harness2 doctor
python3 harness/agentctl.py harness2 demo --k 1
python3 harness/agentctl.py harness2 research --task wallet-fee-reconciliation --mode sequential --k 1
python3 harness/agentctl.py harness2 status
python3 harness/agentctl.py harness2 report --run <RUN_DIR_NAME>
```

## What actually runs

- `doctor`: verifies lock-vs-checkout pin, import path, CLI surface,
  license; reports `model_backed: BLOCKED_NO_ISOLATED_WORKER`.
- `demo`: upstream credential-free recursion (deterministic editors +
  real Python subprocess execution).
- `research`: the REAL upstream Propose–Probe–Compose recursion over
  `integrations/harness2/adapter.py` (the 13-method BenchmarkAdapter
  contract) on the `wallet-fee-reconciliation` domain — the base harness
  carries the PF0 CE-1 fee-double-count defect; the deterministic
  proposer applies the real fix; `score()` is lamport-exactness.
  Verified: sequential+parallel k=1 → 2/2 tasks ok, experience library
  persisted per task.
- `report`: run results + persisted experience entries.

## Boundaries (hard)

- Model-backed substrates are **BLOCKED** (Codex runs unsandboxed with
  approvals bypassed upstream-side). The isolated-worker spec is
  PREPARED at `integrations/harness2/worker/` — enable only per
  `OPERATOR_CHECKLIST.md`, on a separate host, never with wallet
  material. `security.py` drops SNF_*/HARNESS2_*/OPENAI*/tokens/secrets
  from every child env and forces LIVE_TRADING_ENABLED=false.
- Harness² optimizes the RESEARCH HARNESS ONLY. Trading-strategy truth
  stays with the Rust P&L V2 oracle + H8 registry. `evidence_bridge.py`
  preregisters hypotheses before evaluation; ONE research command = ONE
  H8 trial (rollouts are sub-units); sealed-holdout references in
  candidate context are refused.
- CI runs only deterministic checks; they skip gracefully without
  `.external/`.

## Tests

`python3 -m unittest discover -s integrations/harness2/tests` — 13
tests: adapter contract, live recursion, experience persistence, lock
consistency, env scrubbing, model-blocking, path jail, bridge
preregistration/trial-budget/holdout refusal.
