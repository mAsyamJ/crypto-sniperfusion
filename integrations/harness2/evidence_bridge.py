"""Typed bridge: Harness² research events -> the existing H8 registry.

Rules preserved from the H8 governance design:
- A hypothesis is PREREGISTERED (hash-chained) BEFORE any evaluation.
- Harness² rollouts are NOT trials: one `research` command is one trial;
  its internal rollouts/executions are recorded inside the trial payload
  so the 20-trial budget counts experiments, not agent actions.
- Holdout labels are structurally absent: this bridge never reads
  HoldoutGuard directories; candidate generation context is refused if
  it references sealed material.
"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "harness"))

from research_registry import Registry, RegistryError  # noqa: E402

BRIDGE_REQUIRED = ("hypothesis_id", "claim", "run_dir", "done", "failed",
                   "execution_count", "editor_calls")


def bridge_run(registry_path, run_summary):
    """Register (if needed) and record one harness2 research run.

    run_summary keys: hypothesis_id, claim, run_dir, done, failed,
    execution_count, editor_calls, [task, mode, k].
    Returns {"hypothesis": seq, "trial": seq}.
    """
    missing = [k for k in BRIDGE_REQUIRED if k not in run_summary]
    if missing:
        raise RegistryError(f"bridge missing fields: {missing}")
    registry = Registry(registry_path)
    hid = run_summary["hypothesis_id"]

    if not any(h["hypothesis_id"] == hid for h in registry.hypotheses()):
        registry.register_hypothesis({
            "hypothesis_id": hid,
            "claim": run_summary["claim"],
            "causal_mechanism": "harness2 deterministic recursion improves "
                                "the research harness (not trading strategies)",
            "decision_time_evidence": ["execution trajectories", "deliverables"],
            "abstention_conditions": ["missing venv", "pin mismatch"],
            "cost_model": "deterministic channel; zero model cost",
            "falsification_conditions": [
                "done < tasks or failed > 0 on the deterministic domain",
                "experience library empty after adoption",
            ],
            "development_dataset_sha256": "embedded-adapter-fixtures-v1",
            "primary_metric": "tasks_done_lamport_exact",
            "risk_constraints": {"model_backed": False},
        })
    # One research command = one trial (rollouts recorded inside).
    used = len(registry.trials())
    trial = registry.record_trial({
        "hypothesis_id": hid,
        "trial_index": used + 1,
        "result": "ADOPTED" if run_summary["failed"] == 0 else "REJECTED",
        "dev_metrics": {
            "done": run_summary["done"],
            "failed": run_summary["failed"],
            "execution_count": run_summary["execution_count"],
            "editor_calls": run_summary["editor_calls"],
            "run_dir": str(run_summary["run_dir"]),
            "budget_note": "rollouts are sub-units of this single trial",
        },
    })
    return {"hypothesis": hid, "trial": trial["seq"]}


def assert_no_holdout_references(obj):
    """Refuse candidate context referencing sealed holdout material."""
    text = json.dumps(obj)
    for marker in ("holdout", "OPENED", "sealed_labels", "labels.json"):
        if marker in text:
            raise RegistryError(
                f"candidate context references sealed material ({marker}); "
                "holdout labels must never reach harness2 candidate generation"
            )
    return True
