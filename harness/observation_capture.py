#!/usr/bin/env python3
"""Bounded READ-ONLY Pump.fun program-touch sample collector for Solana RPC.

This does not have a complete creation-event denominator, tx-index order,
confirmed feature availability, continuous stream, or a shadow window.
Its output is untrusted RAW evidence, not a backtest or sniping signal.
"""
import argparse
import hashlib
import json
import os
import sys
import time
from pathlib import Path

from cluster_probe import safe_endpoint, rpc_call

GENESIS = {
    "mainnet": "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d",
    "devnet": "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG",
}
RPC = {
    "mainnet": "https://api.mainnet-beta.solana.com",
    "devnet": "https://api.devnet.solana.com",
}
PUMP = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
RAW_EVIDENCE_CLASS = "READ_ONLY_PROGRAM_TOUCH_SAMPLE_INCOMPLETE"


def canonical(obj):
    return json.dumps(obj, sort_keys=True, separators=(",", ":"))


def capture(cluster, endpoint, limit=20, call=rpc_call, pause=None):
    if cluster not in GENESIS:
        raise ValueError("collector cluster must be mainnet or devnet")
    if not (1 <= limit <= 100):
        raise ValueError("limit must be 1..100 to bound shared RPC demand")
    safe_endpoint(endpoint)
    genesis = call(endpoint, "getGenesisHash")
    if genesis != GENESIS[cluster]:
        raise ValueError("WRONG_CLUSTER: source genesis did not match")
    signatures = call(endpoint, "getSignaturesForAddress", [
        PUMP, {"limit": limit, "commitment": "confirmed"},
    ])
    if not isinstance(signatures, list):
        raise ValueError("unexpected signatures shape")
    seen, rows, missing = set(), [], 0
    for rank, row in enumerate(signatures):
        if not isinstance(row, dict):
            raise ValueError("invalid signature record")
        sig = row.get("signature")
        if not isinstance(sig, str) or not sig or sig in seen:
            raise ValueError("duplicate/invalid signature in provider response")
        seen.add(sig)
        if not isinstance(row.get("slot"), int):
            raise ValueError("missing raw slot")
        received_wall_ns = time.time_ns()
        received_mono_ns = time.monotonic_ns()
        try:
            tx = call(endpoint, "getTransaction", [
                sig, {"encoding": "json",
                      "maxSupportedTransactionVersion": 0,
                      "commitment": "confirmed"},
            ])
        except (OSError, ValueError, TimeoutError):
            tx = None
        if tx is None:
            missing += 1
        rows.append({
            "rpc_rank_newest_first": rank,
            "signature": sig, "slot": row["slot"],
            "block_time_seconds": row.get("blockTime"),
            "source_status": row.get("confirmationStatus"),
            "provider_error": row.get("err"),
            "received_wall_ns": received_wall_ns,
            "received_mono_ns": received_mono_ns,
            "transaction_data_status": "AVAILABLE" if tx is not None else "UNAVAILABLE",
            "raw_transaction": tx,
        })
        if pause is not None:
            pause()
    report = {
        "schema_version": 1, "source_cluster": cluster, "genesis": genesis,
        "program_id": PUMP,
        "evidence_class": RAW_EVIDENCE_CLASS,
        "coverage": "UNKNOWN_INCOMPLETE_PROGRAM_TOUCH_ONLY",
        "all_launches_captured": False,
        "transaction_index_reconstructed": False,
        "market_labels_derived": False,
        "nonpromotable": True,
        "read_only": True,
        "submitted_transactions": 0,
        "requested_limit": limit,
        "observed_signatures": len(rows), "missing_transactions": missing,
        "records": rows,
    }
    # This digest includes observed timestamps; to reproduce comparisons,
    # compare raw sig+tx payload identities, not wall-clock receipt hashes.
    report["receipt_sha256"] = hashlib.sha256(
        canonical(report).encode("utf-8")
    ).hexdigest()
    return report


def write_once(path, report):
    dest = Path(path)
    dest.parent.mkdir(parents=True, exist_ok=True)
    with dest.open("x", encoding="utf-8") as f:
        f.write(json.dumps(report, sort_keys=True, indent=2) + "\n")
        f.flush()
        os.fsync(f.fileno())
    try:
        dest.chmod(0o444)
    except OSError:
        pass


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cluster", choices=list(GENESIS), default="mainnet")
    parser.add_argument("--limit", type=int, default=20)
    parser.add_argument("--out", required=True)
    parser.add_argument("--pause-ms", type=int, default=400)
    args = parser.parse_args(argv)
    try:
        if os.environ.get("LIVE_TRADING_ENABLED", "false").lower() not in ("", "0", "false"):
            raise ValueError("collector requires LIVE_TRADING_ENABLED=false")
        if any(os.environ.get(k) for k in ("SOLANA_PRIVATE_KEY", "PRIVATE_KEY")):
            raise ValueError("collector refuses signer variables")
        if not (0 <= args.pause_ms <= 10000):
            raise ValueError("pause-ms out of bounds")
        env_key = "SNF_MAINNET_READ_RPC" if args.cluster == "mainnet" else "SNF_DEVNET_RPC"
        endpoint = os.environ.get(env_key) or RPC[args.cluster]
        report = capture(args.cluster, endpoint, args.limit,
                         pause=lambda: time.sleep(args.pause_ms / 1000))
        write_once(args.out, report)
        print(json.dumps({
            "status": "READ_ONLY_CAPTURED",
            "report": args.out, "observed": report["observed_signatures"],
            "unavailable": report["missing_transactions"],
            "coverage": report["coverage"], "submitted_transactions": 0,
            "receipt_sha256": report["receipt_sha256"],
        }, indent=2, sort_keys=True))
        return 0
    except (ValueError, OSError, TimeoutError) as exc:
        print(f"OBSERVATION_CAPTURE_REFUSED: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
