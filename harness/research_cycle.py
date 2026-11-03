#!/usr/bin/env python3
"""Finite, OFFLINE strategy research cycles with a single sealed final check.

This controller is intentionally for synthetic V0 event contracts only.
It does NOT perform AI inference, mainnet observation, chain transactions,
or actual calibrated trading backtests. NEVER grants promotion.
"""
import argparse
import copy
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

from research_batch import (
    EVIDENCE_CLASS, STRATEGIES, diagnose, hash_file, load_events,
    require_offline, simulate,
)

RESEARCH_VERSION = "synthetic-cycle-v1"
SCENARIOS = {
    "base": {"slip_bps": 100, "tx_cost": 10000, "liquidity_divisor": 1},
    "adverse": {"slip_bps": 300, "tx_cost": 30000, "liquidity_divisor": 2},
    "severe": {"slip_bps": 800, "tx_cost": 150000, "liquidity_divisor": 4},
}


def dump(obj):
    return json.dumps(obj, indent=2, sort_keys=True, separators=(",", ": ")) + "\n"


def sha(obj):
    return hashlib.sha256(dump(obj).encode("utf-8")).hexdigest()


def git_sha():
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
        text=True, check=False, cwd=Path(__file__).resolve().parent.parent,
    )
    return result.stdout.strip() if result.returncode == 0 else "unknown"


def partition(events):
    """Deterministic mint-disjoint development and ONE sealed sanity split.

    This toy split is a demonstrator and NOT creator-cluster/temporal OOS.
    Real experiment data must split by creator clusters + time with embargo.
    """
    mints = sorted({e["mint"] for e in events})
    if len(mints) < 2:
        raise ValueError("at least two distinct mints needed for the toy split")
    development, sealed = set(mints[:-1]), {mints[-1]}
    return (
        [e for e in events if e["mint"] in development],
        [e for e in events if e["mint"] in sealed],
    )


def simulate_scenario(events, scenario, cash, spend, max_hold_ms):
    observed = copy.deepcopy(events)
    div = scenario["liquidity_divisor"]
    for e in observed:
        e["exit_liquidity_lamports"] //= div
    return [
        simulate(
            observed, strategy, cash, spend,
            scenario["slip_bps"], scenario["tx_cost"],
            1_500_000, max_hold_ms,
        )
        for strategy in STRATEGIES
    ]


def make_proposal(cycle_index, scenarios):
    """Only analysis hypotheses; no automatic strategy source modification."""
    losses = {}
    for name, rows in scenarios.items():
        for r in rows:
            key = r["strategy"]
            losses.setdefault(key, []).append({
                "scenario": name, "net": r["net_pnl_lamports"],
                "drawdown": r["max_drawdown_lamports"],
                "open": r["open_positions"],
                "failed_exits": r["failed_exit_quotes"],
            })
    order = sorted(
        STRATEGIES,
        key=lambda key: (
            -min(x["net"] - x["drawdown"] for x in losses[key]),
            -max(x["failed_exits"] for x in losses[key]),
            key != "S00", key,
        ),
    )
    # S00 is a rational winner when no experiment survives after-cost risk.
    candidate = order[0]
    return {
        "hypothesis_id": f"SV0-H-{cycle_index:03d}",
        "candidate": candidate,
        "expected_mechanism": "Does this policy outperform doing nothing under stressed surrogate exits?",
        "observed_features_required": [
            "feature_available_at_ms", "phase", "safety",
            "net_flow_lamports", "exit_liquidity_lamports",
        ],
        "falsification_tests": [
            "independent real program economics", "creator-cluster time holdout",
            "replay completeness", "failure and latency tail costs",
            "no more than 1 holdout read per hypothesis",
        ],
        "research_only": True,
    }


def cycle_report(
    events, dataset_hash, cycles, cash=100_000_000,
    spend=1_000_000, max_hold_ms=20_000,
):
    require_offline()
    if not (1 <= cycles <= 20):
        raise ValueError("cycles must be 1..20; no unbounded self-improvement")
    if cash <= 0 or spend <= 0 or max_hold_ms <= 0 or spend > cash:
        raise ValueError("invalid budget/horizon")
    development, sealed = partition(events)
    # The split is fixed before the first experiment and held unchanged.
    dev_hash, sealed_hash = sha(development), sha(sealed)
    receipts = []
    for idx in range(1, cycles + 1):
        # Repeat an experiment with the SAME frozen version; iteration is
        # recorded and cannot silently promote or optimize sealed labels.
        scenarios = {
            name: simulate_scenario(development, scenario, cash, spend, max_hold_ms)
            for name, scenario in SCENARIOS.items()
        }
        hypothesis = make_proposal(idx, scenarios)
        trial = {
            "cycle": idx,
            "candidate": hypothesis["candidate"],
            "development_sha256": dev_hash,
            "trials_in_cycle": len(SCENARIOS) * len(STRATEGIES),
            "hypothesis": hypothesis,
            "development_results": scenarios,
            "review": {
                "reviewer": "DETERMINISTIC_STATIC_CRITIC_NOT_AI",
                "decision": "RESEARCH_ONLY",
                "reason": "Not calibrated to real program/network economics",
            },
        }
        trial["receipt_sha256"] = sha(trial)
        receipts.append(trial)

    # A single final sanity check; NEVER feed this result back into candidate
    # selection. It cannot count as proper untouched temporal OOS.
    champion = receipts[-1]["candidate"]
    heldout = {
        name: next(
            r for r in simulate_scenario(sealed, sc, cash, spend, max_hold_ms)
            if r["strategy"] == champion
        )
        for name, sc in SCENARIOS.items()
    }
    comparison = {
        name: next(
            r for r in simulate_scenario(sealed, sc, cash, spend, max_hold_ms)
            if r["strategy"] == "S00"
        )
        for name, sc in SCENARIOS.items()
    }
    return {
        "schema_version": 1, "research_version": RESEARCH_VERSION,
        "evidence_class": EVIDENCE_CLASS,
        "promotable": False, "mainnet_execution": False,
        "dataset_sha256": dataset_hash,
        "script_sha256": hash_file(Path(__file__)),
        "code_sha": git_sha(),
        "development_hash": dev_hash, "sealed_hash": sealed_hash,
        "split_type": "SYNTHETIC_MINT_DISJOINT_NOT_TEMPORAL_OOS",
        "development_mints": len({e["mint"] for e in development}),
        "sealed_mints": len({e["mint"] for e in sealed}),
        "total_trials_recorded": cycles * len(SCENARIOS) * len(STRATEGIES),
        "cycles": receipts,
        "final_candidate": champion,
        "sealed_check_read_count": 1,
        "sealed_check": heldout,
        "sealed_no_trade": comparison,
        "independent_review": {
            "decision": "NOT_DEMONSTRATED",
            "reason": "Synthetic surrogate, no real fills, no valid temporal/cluster holdout",
            "next_step": "Build unbiased read-only mainnet lifecycle dataset",
        },
    }


def write_once(path, receipt):
    out = Path(path)
    out.parent.mkdir(parents=True, exist_ok=True)
    # Exclusive file creation even if a concurrent process races.
    with out.open("x", encoding="utf-8") as f:
        f.write(dump(receipt))
    return out


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--events", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--cycles", type=int, default=3)
    parser.add_argument("--cash", type=int, default=100_000_000)
    parser.add_argument("--spend", type=int, default=1_000_000)
    parser.add_argument("--max-hold-ms", type=int, default=20_000)
    args = parser.parse_args(argv)
    try:
        require_offline()
        events = load_events(args.events)
        receipt = cycle_report(
            events, hash_file(args.events), args.cycles,
            cash=args.cash, spend=args.spend, max_hold_ms=args.max_hold_ms,
        )
        write_once(args.out, receipt)
        print(dump({
            "status": "RESEARCH_ONLY",
            "receipt_path": str(args.out),
            "receipt_sha256": sha(receipt),
            "cycles": len(receipt["cycles"]),
            "trials": receipt["total_trials_recorded"],
            "candidate": receipt["final_candidate"],
            "promotable": False,
        }))
    except (OSError, ValueError, KeyError) as exc:
        print(f"RESEARCH_CYCLE_REFUSED: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
