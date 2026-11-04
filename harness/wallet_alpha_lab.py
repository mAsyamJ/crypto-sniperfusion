#!/usr/bin/env python3
"""OFFLINE wallet-follow copyability negative-control experiment.

No RPC, HTTP, signer, streaming or mainnet broadcaster. A row is a *completed
hypothetical follower round trip*, not a complete portfolio. Gross entry/exit
quotes already include pool, creator and price-impact costs; network/priority
fees and tips are charged once separately. Real data adapters are WA1-WA3.
"""
import argparse
import hashlib
import json
import os
import sys
from collections import defaultdict
from pathlib import Path

REQUIRED_STR = ("trade_id", "wallet", "mint", "coverage", "proof_state")
REQUIRED_INT = (
    "ranking_asof_ms", "leader_buy_ms", "signal_observed_ms",
    "follower_entry_quote_at_ms", "leader_sell_ms", "exit_signal_observed_ms",
    "follower_exit_quote_at_ms", "leader_entry_debited_lamports",
    "leader_exit_received_lamports", "entry_quote_debited_lamports",
    "exit_quote_received_lamports", "network_fee_lamports",
    "priority_fee_lamports", "tip_lamports",
)
ALL_COVERAGE = {"COMPLETE", "INCOMPLETE", "UNKNOWN", "CENSORED"}
ALL_PROOF = {"SYNTHETIC", "MAINNET_READ_ONLY_ESTIMATE"}

def validate_row(row, line_no):
    if not isinstance(row, dict):
        raise ValueError(f"line {line_no}: expected object")
    for key in REQUIRED_STR:
        if not isinstance(row.get(key), str) or not row[key]:
            raise ValueError(f"line {line_no}: missing or invalid {key}")
    for key in REQUIRED_INT:
        value = row.get(key)
        if type(value) is not int or value < 0:
            raise ValueError(f"line {line_no}: {key} must be nonnegative integer")
    if row["coverage"] not in ALL_COVERAGE:
        raise ValueError(f"line {line_no}: invalid coverage")
    if row["proof_state"] not in ALL_PROOF:
        raise ValueError(f"line {line_no}: invalid proof_state")
    t = row
    if not (t["ranking_asof_ms"] <= t["leader_buy_ms"] <=
            t["signal_observed_ms"] <= t["follower_entry_quote_at_ms"] <=
            t["leader_sell_ms"] <= t["exit_signal_observed_ms"] <=
            t["follower_exit_quote_at_ms"]):
        raise ValueError(f"line {line_no}: lookahead or noncausal timing")
    if min(t["leader_entry_debited_lamports"], t["entry_quote_debited_lamports"]) <= 0:
        raise ValueError(f"line {line_no}: zero/negative entry spend")
    if t["coverage"] == "COMPLETE" and t["follower_exit_quote_at_ms"] == 0:
        raise ValueError(f"line {line_no}: incomplete exit quote")
    return row

def load_records(path):
    records, ids = [], set()
    for n, line in enumerate(Path(path).read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        row = validate_row(json.loads(line), n)
        if row["trade_id"] in ids:
            raise ValueError(f"line {n}: duplicate trade_id")
        ids.add(row["trade_id"])
        records.append(row)
    if not records:
        raise ValueError("no rows supplied; cannot infer wallet profitability")
    return records

def metrics(records, min_trades=20):
    if min_trades < 1:
        raise ValueError("min_trades must be >=1")
    by_wallet = defaultdict(list)
    excluded = defaultdict(int)
    proof_classes = set()
    for row in records:
        proof_classes.add(row["proof_state"])
        if row["coverage"] != "COMPLETE":
            excluded[row["coverage"]] += 1
            continue
        by_wallet[row["wallet"]].append(row)
    output = []
    for wallet, trades in sorted(by_wallet.items()):
        follower_net, leader_net, follower_spend, leader_spend = 0, 0, 0, 0
        follower_wins, costs = 0, 0
        for r in trades:
            extra = r["network_fee_lamports"] + r["priority_fee_lamports"] + r["tip_lamports"]
            net = r["exit_quote_received_lamports"] - r["entry_quote_debited_lamports"] - extra
            follower_net += net
            leader_net += r["leader_exit_received_lamports"] - r["leader_entry_debited_lamports"]
            follower_spend += r["entry_quote_debited_lamports"]
            leader_spend += r["leader_entry_debited_lamports"]
            costs += extra
            follower_wins += net > 0
        follower_return_bps = (10_000 * follower_net) // follower_spend
        leader_return_bps = (10_000 * leader_net) // leader_spend
        enough = len(trades) >= min_trades
        output.append({
            "wallet": wallet,
            "complete_roundtrips": len(trades),
            "sample_sufficient_for_screen": enough,
            "verdict": ("INSUFFICIENT_SAMPLE" if not enough else
                        "POSITIVE_REQUIRES_OUT_OF_SAMPLE" if follower_net > 0 else "NEGATIVE_NET"),
            "follower_net_pnl_lamports": follower_net,
            "leader_net_pnl_lamports_excluding_leader_chain_costs": leader_net,
            "follower_invested_quote_lamports_cumulative": follower_spend,
            "follower_costs_excluding_embedded_pool_fees_lamports": costs,
            "follower_win_rate": follower_wins / len(trades),
            "follower_return_bps_on_cumulative_spend": follower_return_bps,
            "leader_return_bps_on_cumulative_spend": leader_return_bps,
            "copyability_return_penalty_bps": leader_return_bps - follower_return_bps,
            "net_edge_vs_no_trade_lamports": follower_net,
        })
    output.sort(key=lambda x: (-x["follower_net_pnl_lamports"], x["wallet"]))
    data_hash = hashlib.sha256(
        "\n".join(json.dumps(r, sort_keys=True, separators=(",", ":")) for r in records).encode()
    ).hexdigest()
    return {
        "schema": "wallet_alpha_offline_v1",
        "evidence_class": "SYNTHETIC_AND_OR_READ_ONLY_ESTIMATES_ONLY",
        "proof_states_seen": sorted(proof_classes),
        "dataset_sha256": data_hash,
        "promotable": False,
        "realized_mainnet_pnl": None,
        "portfolio_nav_reconciled": False,
        "caveat": "Independent completed roundtrips only. NOT a shared-capital portfolio, not a live fill, and not evidence of stable profit.",
        "rows": len(records),
        "excluded_incomplete_rows": dict(sorted(excluded.items())),
        "wallets": output,
    }

def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--events", required=True, help="JSONL of completed hypothetical follower round trips")
    parser.add_argument("--out", help="write-once JSON report; otherwise stdout")
    parser.add_argument("--min-trades", type=int, default=20)
    args = parser.parse_args(argv)
    if os.environ.get("LIVE_TRADING_ENABLED", "false").lower() not in ("false", "0", ""):
        parser.error("offline wallet-alpha evaluator refuses live trading")
    try:
        report = metrics(load_records(args.events), args.min_trades)
        rendered = json.dumps(report, sort_keys=True, indent=2) + "\n"
        if args.out:
            out = Path(args.out)
            out.parent.mkdir(parents=True, exist_ok=True)
            with out.open("x", encoding="utf-8") as f:
                f.write(rendered)
                f.flush()
                os.fsync(f.fileno())
        else:
            print(rendered, end="")
        return 0
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"wallet alpha offline evaluation refused: {exc}", file=sys.stderr)
        return 2

if __name__ == "__main__":
    raise SystemExit(main())
