#!/usr/bin/env python3
"""Read-only, deterministic *surrogate* multi-strategy research harness.

This is NOT a Solana emulator, fork runner, fill simulator calibrated to
real execution, backtest evidence of profitability, AI model, or broadcaster.
No network, signing, RPC or secret loading is possible in this module.
"""
import argparse
import hashlib
import json
import os
import sys
from pathlib import Path

PHASES = frozenset({
    "CREATED", "EARLY_CURVE", "ACTIVE_CURVE", "NEAR_COMPLETION",
    "CURVE_COMPLETED", "MIGRATION_PENDING", "PUMPSWAP_OPEN",
    "POST_MIGRATION", "DECAYING", "INACTIVE",
})
TRADEABLE = frozenset({"EARLY_CURVE", "ACTIVE_CURVE", "NEAR_COMPLETION",
                       "PUMPSWAP_OPEN", "POST_MIGRATION"})
STRATEGIES = {
    "S00": "no_trade", "S01": "immediate_safe", "S02": "delay_5s",
    "S03": "delay_15s", "S04": "organic_flow", "S05": "dip_rebound",
    "S06": "flow_creator_veto", "S07": "curve_regime",
    "S08": "pre_graduation", "S09": "post_migration",
    "S10": "liquidity_first",
}
REQUIRED = {
    "event_id", "mint", "slot", "chain_at_ms", "observed_at_ms",
    "feature_available_at_ms", "phase", "safety", "coverage_ok",
    "price_lamports_per_token", "exit_liquidity_lamports",
    "net_flow_lamports", "unique_buyers", "creator_sell_lamports",
    "curve_progress_bps", "fee_bps",
}
UNSIGNED = {
    "slot", "chain_at_ms", "observed_at_ms", "feature_available_at_ms",
    "price_lamports_per_token", "exit_liquidity_lamports",
    "unique_buyers", "creator_sell_lamports", "curve_progress_bps", "fee_bps",
}
EVIDENCE_CLASS = "SYNTHETIC_SURROGATE_UNCALIBRATED"
MODEL_LIMITATIONS = [
    "synthetic or externally supplied observations, not verified complete mainnet cohort",
    "fills priced immediately at visible snapshots; no actual transaction landing model",
    "linear token-price approximation, not the deployed Pump.fun/PumpSwap reserve math",
    "unknown exit liquidity treated as worthless; all modeled positions are virtual",
    "no out-of-sample evaluation, uncertainty intervals or production promotion",
]


def canonical(obj):
    return json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=True)


def hash_file(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def fail(message):
    raise ValueError(message)


def require_offline():
    if os.environ.get("LIVE_TRADING_ENABLED", "false").lower() not in ("", "false", "0"):
        fail("offline research refuses LIVE_TRADING_ENABLED")
    if any(os.environ.get(key) for key in ("SOLANA_PRIVATE_KEY", "PRIVATE_KEY")):
        fail("remove private-key environment variables from offline research process")


def load_events(path):
    records, ids = [], set()
    text = Path(path).read_text(encoding="utf-8")
    last_observed = -1
    last_seq_by_mint = {}
    for lineno, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        try:
            e = json.loads(line)
        except json.JSONDecodeError as ex:
            fail(f"line {lineno}: invalid JSON: {ex}")
        if not isinstance(e, dict) or REQUIRED - e.keys():
            fail(f"line {lineno}: missing required fields {sorted(REQUIRED - e.keys()) if isinstance(e, dict) else REQUIRED}")
        if not isinstance(e["event_id"], str) or not e["event_id"] or e["event_id"] in ids:
            fail(f"line {lineno}: invalid/duplicate event_id")
        ids.add(e["event_id"])
        if not isinstance(e["mint"], str) or not e["mint"]:
            fail(f"line {lineno}: invalid mint")
        for key in UNSIGNED:
            if type(e[key]) is not int or e[key] < 0:
                fail(f"line {lineno}: {key} must be nonnegative integer")
        if type(e["net_flow_lamports"]) is not int:
            fail(f"line {lineno}: net_flow_lamports must be integer")
        if type(e["coverage_ok"]) is not bool:
            fail(f"line {lineno}: coverage_ok must be boolean")
        if e["phase"] not in PHASES or e["safety"] not in ("ELIGIBLE", "WATCH", "BLOCK"):
            fail(f"line {lineno}: invalid phase/safety")
        if e["curve_progress_bps"] > 10000 or e["fee_bps"] >= 10000:
            fail(f"line {lineno}: invalid bps")
        if e["phase"] in TRADEABLE and e["price_lamports_per_token"] == 0:
            fail(f"line {lineno}: tradeable phase has zero price")
        if not (e["chain_at_ms"] <= e["observed_at_ms"] <= e["feature_available_at_ms"]):
            fail(f"line {lineno}: lookahead/clock-order violation (strict demo contract)")
        if e["observed_at_ms"] < last_observed:
            fail(f"line {lineno}: observation chronology violated; never sort unseen data")
        last_observed = e["observed_at_ms"]
        prev = last_seq_by_mint.get(e["mint"])
        if prev and (e["slot"], e["event_id"]) == prev:
            fail(f"line {lineno}: duplicate chain coordinate")
        last_seq_by_mint[e["mint"]] = (e["slot"], e["event_id"])
        records.append(e)
    if not records:
        fail("no events")
    return records


def eligible(strategy, event, state, spend, knobs=None):
    knobs = knobs or {}
    min_buyers = knobs.get("min_buyers", 5)
    liquidity_multiple = knobs.get("liquidity_multiple", 20)
    if (type(min_buyers) is not int or not 1 <= min_buyers <= 30 or
            type(liquidity_multiple) is not int or not 1 <= liquidity_multiple <= 100):
        fail("invalid preregistered strategy knobs")
    if strategy == "S00":
        return False
    if not event["coverage_ok"] or event["safety"] != "ELIGIBLE":
        return False
    if event["phase"] not in TRADEABLE:
        return False
    age = event["observed_at_ms"] - state["first_observed"]
    buyers = event["unique_buyers"]
    flow = event["net_flow_lamports"]
    liquidity = event["exit_liquidity_lamports"]
    phase = event["phase"]
    progress = event["curve_progress_bps"]
    if strategy == "S01":
        return True
    if strategy == "S02":
        return age >= 5000
    if strategy == "S03":
        return age >= 15000
    if strategy == "S04":
        return phase == "ACTIVE_CURVE" and buyers >= min_buyers and flow > 0
    if strategy == "S05":
        return (phase == "ACTIVE_CURVE" and state["seen_dip"]
                and event["price_lamports_per_token"] > state["previous_price"])
    if strategy == "S06":
        return (phase == "ACTIVE_CURVE" and buyers >= min_buyers and flow > 0
                and event["creator_sell_lamports"] == 0)
    if strategy == "S07":
        return phase == "ACTIVE_CURVE" and 7000 <= progress < 9000
    if strategy == "S08":
        return phase == "NEAR_COMPLETION" and progress >= 9000
    if strategy == "S09":
        return phase in ("PUMPSWAP_OPEN", "POST_MIGRATION") and buyers >= 3
    if strategy == "S10":
        return liquidity >= liquidity_multiple * spend and buyers >= 3 and flow > 0
    fail("unknown strategy")


def executable_exit(position, event, slip_bps, tx_cost):
    """Conservative single-tick surrogate quote, NOT a live executable quote."""
    if event is None or not event["coverage_ok"] or event["safety"] == "BLOCK":
        return 0
    gross = position["tokens"] * event["price_lamports_per_token"]
    liquidity = event["exit_liquidity_lamports"]
    if gross > liquidity or liquidity <= 0:
        return 0
    impact_bps = min(8000, gross * 10000 // liquidity)
    cost_bps = event["fee_bps"] + slip_bps + impact_bps
    if cost_bps >= 10000:
        return 0
    return max(0, gross * (10000 - cost_bps) // 10000 - tx_cost)


def simulate(events, strategy, cash, spend, slip_bps, tx_cost, ata_rent, max_hold_ms,
             knobs=None):
    balance, positions, seen_accounts = cash, {}, set()
    state, latest, trades = {}, {}, []
    equity_curve, attempts, fills, rejected, exits, no_exit_liquidity = [], 0, 0, 0, 0, 0
    for e in events:
        mint = e["mint"]
        st = state.setdefault(mint, {
            "first_observed": e["observed_at_ms"], "peak": e["price_lamports_per_token"],
            "previous_price": e["price_lamports_per_token"], "seen_dip": False,
        })
        previous_peak = st["peak"]
        if previous_peak > 0 and e["price_lamports_per_token"] * 100 <= previous_peak * 95:
            st["seen_dip"] = True
        latest[mint] = e
        if mint in positions:
            p = positions[mint]
            due = (e["feature_available_at_ms"] - p["entry_at_ms"] >= max_hold_ms
                   or e["phase"] in ("DECAYING", "INACTIVE"))
            if due:
                value = executable_exit(p, e, slip_bps, tx_cost)
                if value > 0:
                    balance += value
                    trades.append({"mint": mint, "entry_cost": p["entry_cost"],
                                   "exit_value": value, "net_pnl": value - p["entry_cost"],
                                   "entry_at_ms": p["entry_at_ms"],
                                   "exit_at_ms": e["feature_available_at_ms"]})
                    del positions[mint]
                    exits += 1
                else:
                    no_exit_liquidity += 1
        elif eligible(strategy, e, st, spend, knobs):
            attempts += 1
            rent = 0 if mint in seen_accounts else ata_rent
            total_cost = spend + tx_cost + rent
            cost_bps = e["fee_bps"] + slip_bps
            tokens = (spend * max(0, 10000 - cost_bps) // 10000
                      // e["price_lamports_per_token"])
            if total_cost <= balance and tokens > 0 and e["exit_liquidity_lamports"] >= spend:
                balance -= total_cost
                positions[mint] = {"tokens": tokens, "entry_at_ms": e["feature_available_at_ms"],
                                   "entry_cost": total_cost}
                seen_accounts.add(mint)
                fills += 1
            else:
                rejected += 1
        st["peak"] = max(st["peak"], e["price_lamports_per_token"])
        st["previous_price"] = e["price_lamports_per_token"]
        equity = balance + sum(executable_exit(p, latest.get(m), slip_bps, tx_cost)
                               for m, p in positions.items())
        equity_curve.append(equity)
    final_equity = equity_curve[-1]
    peak, max_drawdown = cash, 0
    for equity in equity_curve:
        peak = max(peak, equity)
        max_drawdown = max(max_drawdown, peak - equity)
    return {
        "strategy": strategy, "name": STRATEGIES[strategy],
        "starting_cash_lamports": cash, "ending_cash_lamports": balance,
        "ending_equity_lamports": final_equity,
        "net_pnl_lamports": final_equity - cash,
        "max_drawdown_lamports": max_drawdown,
        "attempted_entries": attempts, "filled_entries": fills, "rejected_entries": rejected,
        "closed_trades": exits, "open_positions": len(positions),
        "failed_exit_quotes": no_exit_liquidity,
        "closed_net_pnl_lamports": sum(t["net_pnl"] for t in trades),
        "trades": trades,
    }


def run_batch(path, cash=100_000_000, spend=1_000_000, slip_bps=100,
              tx_cost=10000, ata_rent=1_500_000, max_hold_ms=20000):
    require_offline()
    if min(cash, spend, max_hold_ms) <= 0 or min(slip_bps, tx_cost, ata_rent) < 0:
        fail("invalid non-positive cash/spend/horizon or negative costs")
    events = load_events(path)
    results = [simulate(events, s, cash, spend, slip_bps, tx_cost, ata_rent, max_hold_ms)
               for s in STRATEGIES]
    return {
        "schema_version": 1, "evidence_class": EVIDENCE_CLASS,
        "promotable": False, "live_trading_enabled": False,
        "dataset_sha256": hash_file(path), "event_count": len(events),
        "mint_count": len({e["mint"] for e in events}),
        "parameters": {"cash": cash, "spend": spend, "slip_bps": slip_bps,
                       "tx_cost": tx_cost, "ata_rent": ata_rent,
                       "max_hold_ms": max_hold_ms},
        "model_limitations": MODEL_LIMITATIONS,
        "results": results,
    }


def diagnose(report):
    if report.get("evidence_class") != EVIDENCE_CLASS or report.get("promotable") is not False:
        fail("diagnose refuses unexpected/promotable report")
    rows = report["results"]
    return {
        "status": "RESEARCH_ONLY_NOT_PROMOTABLE",
        "hypotheses": [
            {"id": "H-ENTRY-COST", "question": "Does the candidate survive measured protocol fees, latency and impact?",
             "requires": "Fork account deltas plus live read-only shadow latency"},
            {"id": "H-SELLABILITY", "question": "Are open or unexitable positions understating tail loss?",
             "requires": "On-chain reserve based full-position exit quotes"},
            {"id": "H-COHORT", "question": "Are all launched mints represented, including non-graduates?",
             "requires": "Complete creation cohort and gap receipts"},
        ],
        "candidate_order_for_investigation_only": [
            row["strategy"] for row in sorted(
                rows, key=lambda r: (r["net_pnl_lamports"] - r["max_drawdown_lamports"], r["strategy"]),
                reverse=True)
        ],
        "rejected_as_profit_evidence": True,
    }


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    sub = p.add_subparsers(dest="command", required=True)
    v = sub.add_parser("validate")
    v.add_argument("--events", required=True)
    b = sub.add_parser("batch")
    b.add_argument("--events", required=True)
    b.add_argument("--out", required=True)
    for name, value in (("cash", 100000000), ("spend", 1000000),
                        ("slip-bps", 100), ("tx-cost", 10000),
                        ("ata-rent", 1500000), ("max-hold-ms", 20000)):
        b.add_argument("--" + name, type=int, default=value)
    d = sub.add_parser("diagnose")
    d.add_argument("--report", required=True)
    args = p.parse_args(argv)
    try:
        require_offline()
        if args.command == "validate":
            events = load_events(args.events)
            result = {"valid_demo_schema": True, "events": len(events),
                      "dataset_sha256": hash_file(args.events),
                      "warning": "not a verified mainnet lifecycle dataset"}
        elif args.command == "batch":
            result = run_batch(args.events, cash=args.cash, spend=args.spend,
                               slip_bps=args.slip_bps, tx_cost=args.tx_cost,
                               ata_rent=args.ata_rent, max_hold_ms=args.max_hold_ms)
            out = Path(args.out)
            if out.exists():
                fail("refusing to overwrite immutable batch receipt")
            out.parent.mkdir(parents=True, exist_ok=True)
            out.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
        else:
            result = diagnose(json.loads(Path(args.report).read_text()))
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0
    except (ValueError, OSError, KeyError) as exc:
        print(f"RESEARCH_HARNESS_REJECTED: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
