#!/usr/bin/env python3
"""SniperFusion H5-H8 mission runner: safe, bounded, synthetic research only.

A runnable agent handoff and strict evidence coordinator. This is NOT an
autonomous AI agent, not a canonical mainnet backtester, not a wallet runner,
and NOT an executor of Devnet/mainnet orders. Does not expose --submit.
"""
import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

from research_batch import EVIDENCE_CLASS, hash_file, load_events, require_offline
from research_cycle import cycle_report

ROOT = Path(__file__).resolve().parents[1]
MASTER_MISSION = "harness/docs/MASTER_AGENT_MISSION_H5_H8.md"
MAX_CYCLES = 20


def digest(obj):
    return hashlib.sha256(
        json.dumps(obj, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def gate_inventory():
    """Explicit distinction: GitHub/source existence is not runtime evidence."""
    return {
        "G0_full_launch_dataset": "BLOCKED_NO_COMPLETE_MAINNET_COHORT",
        "G1_python_gate": "PENDING",
        "G1_rust_ci": "REQUIRES_EXTERNAL_CI_RECEIPT",
        "G2_execution_faithful_accounting": "RUST_ENGINE_B_IMPLEMENTED_REAL_SNAPSHOTS_NOT_VALIDATED",
        "G3_real_temporal_cluster_oos_alpha": "NOT_DEMONSTRATED",
        "G4_full_program_fork_parity": "M3C_SCOPE_FORK_EXECUTED_NOT_REVERIFIED_BY_THIS_RUN",
        "G5_mainnet_read_only_shadow": "NOT_STARTED",
        "G6_operator_key_and_live_gate_review": "REQUIRES_INDEPENDENT_REVIEW",
        "G7_paper_only_promotion": "DISABLED",
    }


def plan():
    return {
        "mission_version": "H5-H8-v2-evidence-status",
        "source": MASTER_MISSION,
        "safety": "OFFLINE_RESEARCH_ONLY",
        "goal": "After-cost risk-adjusted OOS value relative to NO_TRADE, not guaranteed win rate",
        "phases": [
            {"id": "H5", "target": "Test protocol execution on Surfpool/Devnet under operator gate",
             "current": "PILOT_SOURCE_IMPLEMENTED_NO_OPERATOR_TRANSACTION_RECEIPT"},
            {"id": "H6", "target": "Immutable unbiased mainnet read-only launch lifecycle dataset",
             "current": "BOUNDED_COLLECTOR_CODE_PRESENT_NO_COMPLETE_MAINNET_COHORT"},
            {"id": "H7", "target": "Rust strategy SDK and independent full portfolio accounting/OOS",
             "current": "RUST_STRATEGY_SDK_ENGINES_ORACLE_PRESENT_NO_REAL_MARKET_EDGE"},
            {"id": "H8", "target": "Offline agent council and deterministic paper-only promotion",
             "current": "SKILLS_AND_FINITE_RESEARCH_LOOP_ONLY"},
        ],
        "gates": gate_inventory(),
        "prohibited_actions": [
            "mainnet transaction signing or broadcasting",
            "agent access to production keys or devnet pilot key",
            "automatic devnet_pump_pilot --submit",
            "unbounded looping or multiple reads of a sealed holdout",
            "declaring synthetic results as real profit",
        ],
    }


def python_gate():
    """Run fixed, network-free repository unit tests; no arbitrary commands."""
    cmd = [sys.executable, "-m", "unittest", "discover",
           "-s", "harness/tests", "-q"]
    # Do not forward any RPC endpoints, signing credentials, LLM API keys,
    # or plugin tokens from the operator environment to the child process.
    env = {k: os.environ[k] for k in ("PATH", "HOME", "LANG", "SYSTEMROOT")
           if k in os.environ}
    env.update({
        "LIVE_TRADING_ENABLED": "false",
        "PYTHONDONTWRITEBYTECODE": "1",
        "PYTHONNOUSERSITE": "1",
    })
    result = subprocess.run(
        cmd, cwd=ROOT, env=env, capture_output=True,
        text=True, check=False, timeout=120,
    )
    return {
        "status": "PASS" if result.returncode == 0 else "FAIL",
        "exit_code": result.returncode,
        "test_command": "python3 -m unittest discover -s harness/tests -q",
        "failure_tail": (result.stderr or "")[-600:] if result.returncode else "",
    }


def execute_research(events_path, cycles, gate=python_gate):
    """Execute the offline slice only. No stage can call an RPC or signer."""
    require_offline()
    if type(cycles) is not int or not 1 <= cycles <= MAX_CYCLES:
        raise ValueError("cycles must be an integer from 1 to 20")
    source = Path(events_path)
    event_hash = hash_file(source)
    gates = gate_inventory()
    test_receipt = gate()
    if test_receipt.get("status") not in ("PASS", "FAIL"):
        raise ValueError("Python gate reported an invalid status")
    gates["G1_python_gate"] = test_receipt["status"]
    result = {
        "schema_version": 1,
        "mission_version": "H5-H8-v1",
        "mission_document": MASTER_MISSION,
        "mission_doc_sha256": hash_file(ROOT / MASTER_MISSION),
        "evidence_class": EVIDENCE_CLASS,
        "promotable": False,
        "mainnet_execution": False,
        "devnet_execution": False,
        "signatures": [],
        "synthetic_input_sha256": event_hash,
        "cycle_limit": cycles,
        "tests": test_receipt,
        "gates": gates,
        "experiment": None,
        "status": "BLOCKED" if test_receipt["status"] != "PASS" else "SYNTHETIC_RESEARCH_COMPLETE",
        "next_verified_dependency": "full, unbiased read-only mainnet launch collector",
    }
    if test_receipt["status"] == "PASS":
        events = load_events(source)
        report = cycle_report(events, event_hash, cycles)
        if report["promotable"] is not False or report["evidence_class"] != EVIDENCE_CLASS:
            raise ValueError("research cycle evidence-class invariant violated")
        result["experiment"] = report
        result["gates"]["G3_real_temporal_cluster_oos_alpha"] = "NOT_DEMONSTRATED"
    result["receipt_sha256"] = digest(result)
    return result


def write_once(path, obj):
    dest = Path(path)
    dest.parent.mkdir(parents=True, exist_ok=True)
    # Exclusive create, fsync: do not overwrite an existing experimental
    # run or accidentally make an old failed gate appear green.
    with dest.open("x", encoding="utf-8") as output:
        json.dump(obj, output, sort_keys=True, indent=2)
        output.write("\n")
        output.flush()
        os.fsync(output.fileno())


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    sub = p.add_subparsers(dest="command", required=True)
    sub.add_parser("plan", help="Print H5-H8 goals and current evidence gate states.")
    cmd = sub.add_parser("run", help="Run local tests + finite synthetic research, never send.")
    cmd.add_argument("--events", required=True, help="Strict synthetic observation JSONL")
    cmd.add_argument("--cycles", type=int, default=3, help="Max 20 development cycles")
    cmd.add_argument("--out", required=True, help="Write-once receipt path")
    args = p.parse_args(argv)
    try:
        # Even plan mode must not imply the presence of a funded trading route.
        require_offline()
        if args.command == "plan":
            print(json.dumps(plan(), indent=2, sort_keys=True))
            return 0
        report = execute_research(args.events, args.cycles)
        write_once(args.out, report)
        print(json.dumps({
            "status": report["status"],
            "evidence_class": report["evidence_class"],
            "promotable": False,
            "receipt_path": args.out,
            "receipt_sha256": report["receipt_sha256"],
            "cycles": len(report["experiment"]["cycles"])
            if report["experiment"] else 0,
            "candidate_for_research_only":
                report["experiment"]["final_candidate"]
                if report["experiment"] else None,
        }, indent=2, sort_keys=True))
        return 0 if report["status"] == "SYNTHETIC_RESEARCH_COMPLETE" else 2
    except (OSError, ValueError, KeyError, subprocess.TimeoutExpired) as exc:
        print(f"AGENT_MISSION_REJECTED: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
