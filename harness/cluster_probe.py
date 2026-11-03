#!/usr/bin/env python3
"""Read-only Solana cluster readiness and Pump.fun/PumpSwap deployment probe.

No keys, airdrops or transaction submission. A genesis mismatch is a HARD
failure even when the endpoint hostname looks correct.
"""
import argparse
import json
import os
import sys
from urllib.request import Request, urlopen
from urllib.parse import urlsplit

GENESIS = {
    "devnet": "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG",
    "testnet": "4uhcVJyU9pJkvQyS88uRDiswHXSCkY3zQawwpjk2NsNY",
}
PUBLIC_RPC = {
    "devnet": "https://api.devnet.solana.com",
    "testnet": "https://api.testnet.solana.com",
}
PROGRAMS = {
    "pump": "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
    "pumpswap": "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
}


def rpc_call(endpoint, method, params=None):
    req = Request(
        endpoint,
        data=json.dumps({
            "jsonrpc": "2.0", "id": 1, "method": method,
            "params": [] if params is None else params,
        }).encode("utf-8"),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urlopen(req, timeout=8) as response:
        obj = json.load(response)
    if "error" in obj or "result" not in obj:
        raise ValueError(f"RPC {method} returned error (details suppressed)")
    return obj["result"]


def safe_endpoint(endpoint):
    parts = urlsplit(endpoint)
    if parts.scheme != "https" or not parts.hostname or parts.username or parts.password:
        raise ValueError("RPC endpoint must be HTTPS without URL embedded credentials")
    if parts.fragment:
        raise ValueError("RPC endpoint fragments forbidden")
    return endpoint


def probe(cluster, endpoint, call=rpc_call):
    if cluster not in GENESIS:
        raise ValueError("only named Devnet/Testnet RPC probes allowed")
    safe_endpoint(endpoint)
    genesis = call(endpoint, "getGenesisHash")
    if genesis != GENESIS[cluster]:
        raise ValueError("WRONG_CLUSTER: network genesis does not match requested cluster")
    slot = call(endpoint, "getSlot", [{"commitment": "confirmed"}])
    programs = {}
    for name, address in PROGRAMS.items():
        account = call(endpoint, "getAccountInfo", [
            address, {"encoding": "base64", "commitment": "confirmed"},
        ])
        info = account.get("value") if isinstance(account, dict) else None
        programs[name] = {
            "program_id": address,
            "exists": info is not None,
            "executable": bool(info and info.get("executable") is True),
        }
    return {
        "schema_version": 1, "cluster": cluster, "genesis": genesis,
        "confirmed_slot": slot,
        "read_only": True, "tx_submitted": 0,
        "programs": programs,
        "pump_trade_pilot_supported": cluster == "devnet" and all(
            item["executable"] for item in programs.values()
        ),
        "caveat": "Program executability does NOT establish a valid mint, quote, fee, or executable profitable strategy",
    }


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--cluster", choices=list(GENESIS), default="devnet")
    p.add_argument("--rpc", default=None)
    a = p.parse_args()
    try:
        env = "SNF_DEVNET_RPC" if a.cluster == "devnet" else "SNF_TESTNET_RPC"
        endpoint = a.rpc or os.environ.get(env) or PUBLIC_RPC[a.cluster]
        report = probe(a.cluster, endpoint)
        print(json.dumps(report, indent=2, sort_keys=True))
    except (ValueError, OSError, TimeoutError) as exc:
        print(f"CLUSTER_PROBE_REFUSED: {exc}", file=sys.stderr)
        sys.exit(2)


if __name__ == "__main__":
    main()
