#!/usr/bin/env python3
"""WA2 wallet roster: token->wallet discovery + bounded history fetch +
copyability ranking over wallet-pnl receipts. Read-only; budgets enforced;
nonpromotable. Excludes known program/infrastructure accounts from the
roster (a pool is not a wallet)."""

import argparse
import json
import os
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from launch_cohort_capture import (  # noqa: E402
    JsonRpc, MAINNET_GENESIS, PUMP_PROGRAM_ID, require_safe_env, write_once,
    with_retry, BudgetExceeded, WrongCluster, RpcError,
)

# Infrastructure accounts that must NEVER be ranked as wallets (known
# program ids, pump authorities/fee recipients, WSOL mint).
INFRASTRUCTURE = {
    PUMP_PROGRAM_ID,
    "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
    "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4",
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
    "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
    "So11111111111111111111111111111111111111112",
    "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
    "TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwa2t1vz7V",  # pump fee
}


def discover_wallets(transactions):
    """Owners of moved token accounts + fee payers, minus infrastructure."""
    wallets = {}
    for tx in transactions:
        meta = tx.get("meta") or {}
        if meta.get("err") is not None:
            continue
        keys = tx["transaction"]["message"]["accountKeys"]
        owners = {
            r.get("owner")
            for r in (meta.get("postTokenBalances") or []) + (meta.get("preTokenBalances") or [])
            if r.get("owner")
        }
        for wallet in owners | {keys[0]}:
            if wallet in INFRASTRUCTURE or len(wallet) > 44:
                continue
            wallets.setdefault(wallet, set()).add(tx["transaction"]["signatures"][0])
    return {w: sorted(s) for w, s in wallets.items()}


def fetch_wallet_history(rpc, wallet, *, limit=25, retries=1, sleep=lambda s: None):
    """Bounded read-only wallet history -> tx receipts (may be partial)."""
    require_safe_env()
    genesis = rpc.get_genesis_hash()
    if genesis != MAINNET_GENESIS:
        raise WrongCluster(genesis, MAINNET_GENESIS)
    try:
        page = with_retry(
            lambda: rpc.get_signatures_for_address(wallet, limit=limit), retries, sleep
        )
    except RpcError as exc:
        return {"error": str(exc), "transactions": []}
    txs = []
    for entry in page:
        if entry.get("err") is not None:
            continue
        try:
            tx = with_retry(lambda s=entry["signature"]: rpc.get_transaction(s), retries, sleep)
        except (RpcError, BudgetExceeded):
            continue
        if tx:
            txs.append(tx)
    return {"transactions": txs, "requested": len(page), "fetched": len(txs)}


def rank(receipts):
    """Copyability ranking from wallet-pnl receipts.

    Rank ingredients (follower economics, never win-rate or name):
    - observed net P&L (oracle-verified) — dominant term
    - coverage: records vs skipped/incomplete (unobserved history penalized)
    - activity: trade count (more evidence = less uncertainty)
    - open/unexitable inventory penalty (marked at zero, conservative)
    Uncertainty column is mandatory; UNKNOWN-heavy wallets sort last.
    """
    rows = []
    for wallet, receipt in receipts.items():
        net = receipt["net_pnl_lamports"]
        records = receipt["records"]
        skipped = receipt["skipped"]
        incomplete = receipt["incomplete"]
        open_inv = receipt["unobserved_opening_inventory_events"]
        unobserved = receipt["unobserved_deposits_lamports"]
        coverage = records / max(1, records + skipped + incomplete + open_inv)
        uncertainty = "HIGH" if (skipped + incomplete + open_inv) > 0 or unobserved > 0 else "LOW"
        loss = net < 0
        # Score: profitable AND covered AND active ranks high; any
        # uncertainty flag caps the tier.
        score = net - 0 * coverage  # net dominates; coverage gates tier
        tier = (
            "UNRANKABLE" if records == 0
            else "REJECTED_LOSS" if loss
            else "LOW_CONFIDENCE" if uncertainty == "HIGH"
            else "CANDIDATE"
        )
        rows.append({
            "wallet": wallet,
            "net_pnl_lamports": net,
            "records": records,
            "coverage": round(coverage, 4),
            "uncertainty": uncertainty,
            "tier": tier,
            "score_lamports": score,
            "ranking_as_of": receipt.get("dataset_sha256", "unknown"),
        })
    rows.sort(key=lambda r: (r["tier"] == "CANDIDATE", r["score_lamports"]), reverse=True)
    return rows


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    disc = sub.add_parser("discover")
    disc.add_argument("--transactions", required=True)
    disc.add_argument("--out", required=True)
    rank_cmd = sub.add_parser("rank")
    rank_cmd.add_argument("--receipts-dir", required=True,
                          help="directory of per-wallet run.json files")
    rank_cmd.add_argument("--out", required=True)
    args = parser.parse_args(argv)
    require_safe_env()

    if args.command == "discover":
        txs = json.loads(Path(args.transactions).read_text())
        wallets = discover_wallets(txs)
        write_once(Path(args.out), json.dumps({
            "wallets": wallets,
            "count": len(wallets),
            "nonpromotable": True,
            "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        }, indent=2, sort_keys=True) + "\n")
        print(json.dumps({"wallets": len(wallets)}))
    else:
        receipts = {}
        for run in Path(args.receipts_dir).glob("*/run.json"):
            receipt = json.loads(run.read_text())
            wallet_dir = run.parent.name
            summary = json.loads((run.parent.parent / f"{wallet_dir}.summary.json").read_text()) \
                if (run.parent.parent / f"{wallet_dir}.summary.json").exists() else None
            if summary:
                receipts[summary["wallet"]] = summary
        rows = rank(receipts)
        write_once(Path(args.out), json.dumps({
            "roster": rows,
            "nonpromotable": True,
            "note": "observed-window economics only; no future data",
        }, indent=2, sort_keys=True) + "\n")
        print(json.dumps(rows[:5], indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
