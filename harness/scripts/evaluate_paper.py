#!/usr/bin/env python3
"""Strict independent evaluator of CLOSED round-trip synthetic/paper trades.

Each amount is integer lamports. Pool fee and slippage already reflected in
entry_quote_debited/exit_quote_received. Network and tip expenses are separate.
This is not a broker, blockchain emulator, or real performance evidence.
"""
import argparse
import json
import math
from pathlib import Path

REQUIRED = {
    "trade_id", "strategy", "mint", "decision_at_ms",
    "features_observed_at_ms", "entry_at_ms", "exit_at_ms",
    "entry_quote_debited", "exit_quote_received", "network_fees_lamports",
    "tips_lamports", "entry_slot", "exit_slot",
}
INTEGERS = (
    "decision_at_ms", "features_observed_at_ms", "entry_at_ms", "exit_at_ms",
    "entry_quote_debited", "exit_quote_received", "network_fees_lamports",
    "tips_lamports", "entry_slot", "exit_slot",
)

def read_trades(path):
    records, seen = [], set()
    for line_no, line in enumerate(Path(path).read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        row = json.loads(line)
        missing = REQUIRED - row.keys()
        if missing:
            raise ValueError(f"line {line_no}: missing fields {sorted(missing)}")
        if row["trade_id"] in seen:
            raise ValueError(f'line {line_no}: duplicate trade_id {row["trade_id"]}')
        seen.add(row["trade_id"])
        for field in INTEGERS:
            if type(row[field]) is not int or row[field] < 0:
                raise ValueError(f"line {line_no}: {field} must be nonnegative integer")
        if not (row["features_observed_at_ms"] <= row["decision_at_ms"] <= row["entry_at_ms"] <= row["exit_at_ms"]):
            raise ValueError(f"line {line_no}: lookahead or invalid timestamp order")
        if row["entry_slot"] > row["exit_slot"]:
            raise ValueError(f"line {line_no}: exit precedes entry slot")
        if row["entry_quote_debited"] <= 0:
            raise ValueError(f"line {line_no}: entry amount must be positive")
        records.append(row)
    if not records:
        raise ValueError("no trades")
    return records

def percentile(values, q):
    idx = (len(values) - 1) * q
    lo, hi = math.floor(idx), math.ceil(idx)
    return values[lo] + (values[hi] - values[lo]) * (idx - lo)

def evaluate(trades):
    wins, pnls, latencies = 0, [], []
    network_tip = equity = peak = max_dd = 0
    for row in sorted(trades, key=lambda r: (r["exit_at_ms"], r["trade_id"])):
        fee = row["network_fees_lamports"] + row["tips_lamports"]
        pnl = row["exit_quote_received"] - row["entry_quote_debited"] - fee
        wins += pnl > 0
        pnls.append(pnl)
        network_tip += fee
        latencies.append(row["entry_at_ms"] - row["decision_at_ms"])
        equity += pnl
        peak = max(peak, equity)
        max_dd = max(max_dd, peak - equity)
    latencies.sort()
    return {
        "trades": len(trades),
        "win_rate": round(wins / len(trades), 6),
        "net_pnl_lamports": sum(pnls),
        "mean_net_pnl_lamports": round(sum(pnls) / len(pnls), 3),
        "network_plus_tip_lamports": network_tip,
        "max_closed_pnl_drawdown_lamports": max_dd,
        "decision_to_entry_latency_ms": {
            "p50": percentile(latencies, 0.5),
            "p95": percentile(latencies, 0.95),
            "p99": percentile(latencies, 0.99),
        },
        "scope": "closed-roundtrips-only; NOT portfolio mark-to-market",
        "validation": "accounting-schema-only; NO blockchain execution proof",
    }

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("path")
    args = parser.parse_args()
    print(json.dumps(evaluate(read_trades(args.path)), indent=2))

if __name__ == "__main__":
    main()
