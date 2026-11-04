#!/usr/bin/env python3
"""WA: wallet watch subscriptions + offline alert ledger.

UX concept from handi-cat's wallet tracker (add/remove tracked wallets,
per-wallet subscriptions, alerts) reimplemented from scratch: NO code
copied (handi-cat ships no license), NO network, NO Telegram, NO signer.
Signals arrive as a JSONL file (from wallet_observer/WA3 flows); alerts
are appended to an immutable per-run ledger. Only CURRENTLY-subscribed
wallets alert — unsubscribing stops future alerts without rewriting
history. Nonpromotable, offline, deterministic.
"""

import argparse
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from launch_cohort_capture import write_once  # noqa: E402


class WatchlistError(Exception):
    pass


def _now():
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def load_watchlist(path):
    if not Path(path).exists():
        return {"version": 1, "subscriptions": {}}
    data = json.loads(Path(path).read_text())
    if data.get("version") != 1 or "subscriptions" not in data:
        raise WatchlistError("malformed watchlist")
    return data


def save_watchlist(path, data):
    """Watchlist is working state (mutable by design); atomic replace."""
    tmp = Path(path).with_suffix(".tmp")
    tmp.write_text(json.dumps(data, indent=2, sort_keys=True))
    tmp.replace(path)


def subscribe(path, wallet, note=""):
    data = load_watchlist(path)
    sub = data["subscriptions"].get(wallet)
    if sub and sub.get("active"):
        raise WatchlistError(f"already subscribed: {wallet}")
    prior = data["subscriptions"].get(wallet) or {}
    data["subscriptions"][wallet] = {
        "active": True,
        "subscribed_at": _now(),
        # History is never rewritten: keep prior unsubscribe timestamp.
        "unsubscribed_at": prior.get("unsubscribed_at"),
        "note": note,
    }
    save_watchlist(path, data)
    return data


def unsubscribe(path, wallet):
    data = load_watchlist(path)
    sub = data["subscriptions"].get(wallet)
    if not sub or not sub.get("active"):
        raise WatchlistError(f"not subscribed: {wallet}")
    sub["active"] = False
    sub["unsubscribed_at"] = _now()
    save_watchlist(path, data)
    return data


def active_subscriptions(path):
    return [w for w, s in load_watchlist(path)["subscriptions"].items() if s.get("active")]


def process_signals(watchlist_path, signals_path, out_path):
    """Emit alerts for signals from ACTIVE subscriptions only.

    Signal rows: {"signal_id","wallet","mint","leader_side","observed_at_ms",
    "quote_available_at_ms"} (the WA3 WalletSignal shape). The alert ledger
    is write-once (0444) and records every signal — alerted or skipped —
    so misses are auditable, never silent.
    """
    if Path(out_path).exists():
        raise WatchlistError("refusing to overwrite immutable alert ledger")
    active = set(active_subscriptions(watchlist_path))
    lines = []
    alerted = skipped = 0
    for line in Path(signals_path).read_text().splitlines():
        if not line.strip():
            continue
        signal = json.loads(line)
        wallet = signal["wallet"]
        if wallet in active:
            alerted += 1
            lines.append(json.dumps({
                "kind": "ALERT",
                "signal_id": signal["signal_id"],
                "wallet": wallet,
                "mint": signal["mint"],
                "leader_side": signal.get("leader_side"),
                "observed_at_ms": signal["observed_at_ms"],
                "alerted_at": _now(),
            }, sort_keys=True))
        else:
            skipped += 1
            lines.append(json.dumps({
                "kind": "SKIPPED_NOT_SUBSCRIBED",
                "signal_id": signal["signal_id"],
                "wallet": wallet,
            }, sort_keys=True))
    write_once(Path(out_path), "\n".join(lines) + ("\n" if lines else ""))
    return {"alerted": alerted, "skipped": skipped, "active": len(active)}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    add = sub.add_parser("subscribe")
    add.add_argument("--watchlist", required=True)
    add.add_argument("--wallet", required=True)
    add.add_argument("--note", default="")
    rem = sub.add_parser("unsubscribe")
    rem.add_argument("--watchlist", required=True)
    rem.add_argument("--wallet", required=True)
    proc = sub.add_parser("process")
    proc.add_argument("--watchlist", required=True)
    proc.add_argument("--signals", required=True)
    proc.add_argument("--out", required=True)
    args = parser.parse_args(argv)
    if args.command == "subscribe":
        subscribe(args.watchlist, args.wallet, args.note)
        print(json.dumps({"subscribed": args.wallet, "active": len(active_subscriptions(args.watchlist))}))
    elif args.command == "unsubscribe":
        unsubscribe(args.watchlist, args.wallet)
        print(json.dumps({"unsubscribed": args.wallet, "active": len(active_subscriptions(args.watchlist))}))
    else:
        summary = process_signals(args.watchlist, args.signals, args.out)
        print(json.dumps(summary))
    return 0


if __name__ == "__main__":
    sys.exit(main())
