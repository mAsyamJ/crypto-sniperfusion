#!/usr/bin/env python3
"""Sampled diagnostic on the completed G0 window (2026-11-05).

D0 (2026-11-05): counts create attempts for BOTH creation families —
legacy `global:create` AND `create_v2` — with discriminators pinned to the
Rust table (crates/sniper-protocols/src/curve.rs
`discriminators_match_idl_derived_constants`). The earlier version matched
only the legacy family, so its "zero create attempts" output could not see
create_v2 attempts at all.

The checkpoint persists signatures, not block payloads, so the question
"were there FAILED create attempts?" needs a small re-fetch. This samples
N random slots from the completed window, getBlock's each (read-only),
and scans pump-program instructions (outer+inner) by success/failure,
with anchor discriminators. Read-only; env-only endpoint; no keys.
Usage: python3 harness/diag_sample_failed_creates.py [sample_n] [ckpt]
"""
import collections
import hashlib
import json
import os
import random
import sys
import time
import urllib.request

PUMP = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
DEFAULT_CKPT = "harness/runs/launch-cohort/ckpt-sweep1.json"
DEFAULT_SAMPLE_N = 60
SAMPLE_SEED = 20261105

ALPH = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def discriminator(name: str) -> bytes:
    """Anchor instruction discriminator: sha256("global:<name>")[:8].

    Same convention as the Rust decoder (curve.rs `anchor_discriminator`);
    the hex pins in tests/test_diag_sample_failed_creates.py assert the
    values against the Rust-pinned table.
    """
    return hashlib.sha256(f"global:{name}".encode()).digest()[:8]


DISC_CREATE = discriminator("create")        # 181ec828051c0777
DISC_CREATE_V2 = discriminator("create_v2")  # d6904cec5f8b31b4
CREATE_FAMILIES = {DISC_CREATE: "create", DISC_CREATE_V2: "create_v2"}


def b58dec(s: str) -> bytes:
    n = 0
    for c in s:
        n = n * 58 + ALPH.index(c)
    h = n.to_bytes((n.bit_length() + 7) // 8, "big")
    return b"\x00" * (len(s) - len(s.lstrip("1"))) + h


def b58enc(data: bytes) -> str:
    n = int.from_bytes(data, "big")
    out = []
    while n > 0:
        n, rem = divmod(n, 58)
        out.append(ALPH[rem])
    for byte in data:
        if byte == 0:
            out.append("1")
        else:
            break
    return "".join(reversed(out))


def resolved_keys(tx) -> list:
    """Canonical versioned-tx key resolution: static accountKeys, then
    ALT-loaded writable, then readonly — the index order for
    programIdIndex. (The G0 collector had the dict-flattening defect this
    mirrors correctly; see docs/evidence/D0-LAUNCH-ATTRIBUTION-AUDIT-20261105.md.)"""
    txm = tx.get("transaction") or {}
    msg = txm.get("message") or tx.get("message") or {}
    meta = tx.get("meta") or {}
    keys = list(msg.get("accountKeys", []))
    la = meta.get("loadedAddresses") or {}
    if isinstance(la, dict):
        keys += list(la.get("writable") or []) + list(la.get("readonly") or [])
    else:
        keys += list(la)
    return keys


def scan_block(block: dict) -> dict:
    """Count transaction outcomes, pump-tx outcomes, per-discriminator
    histograms, and create attempts for BOTH creation families
    (outer + inner instructions). Pure: no I/O, no network."""
    stats = {
        "all_tx": {"ok": 0, "fail": 0},
        "pump_tx": {"ok": 0, "fail": 0},
        "disc_ok": collections.Counter(),
        "disc_fail": collections.Counter(),
        "create_attempts": {
            "create": {"ok": 0, "fail": 0},
            "create_v2": {"ok": 0, "fail": 0},
        },
    }
    for tx in block.get("transactions") or []:
        txm = tx.get("transaction") or {}
        msg = txm.get("message") or tx.get("message") or {}
        meta = tx.get("meta") or {}
        keys = resolved_keys(tx)
        ok = meta.get("err") is None
        stats["all_tx"]["ok" if ok else "fail"] += 1
        if not any(k == PUMP for k in keys):
            continue
        stats["pump_tx"]["ok" if ok else "fail"] += 1
        ixs = list(msg.get("instructions", []))
        for grp in meta.get("innerInstructions") or []:
            ixs.extend(grp.get("instructions", []))
        for ix in ixs:
            idx = ix.get("programIdIndex")
            if idx is None or idx >= len(keys) or keys[idx] != PUMP:
                continue
            try:
                data = b58dec(ix.get("data", ""))
            except Exception:
                continue
            if len(data) < 8:
                continue
            (stats["disc_ok"] if ok else stats["disc_fail"])[data[:8].hex()] += 1
            family = CREATE_FAMILIES.get(bytes(data[:8]))
            if family:
                stats["create_attempts"][family]["ok" if ok else "fail"] += 1
    return stats


def aggregate(totals: dict, stats: dict) -> None:
    for bucket in ("all_tx", "pump_tx"):
        for outcome in ("ok", "fail"):
            totals[bucket][outcome] += stats[bucket][outcome]
    for outcome, hist_key in (("ok", "disc_ok"), ("fail", "disc_fail")):
        totals[hist_key].update(stats[hist_key])
    for family in ("create", "create_v2"):
        for outcome in ("ok", "fail"):
            totals["create_attempts"][family][outcome] += \
                stats["create_attempts"][family][outcome]


def empty_totals() -> dict:
    return {
        "all_tx": {"ok": 0, "fail": 0},
        "pump_tx": {"ok": 0, "fail": 0},
        "disc_ok": collections.Counter(),
        "disc_fail": collections.Counter(),
        "create_attempts": {
            "create": {"ok": 0, "fail": 0},
            "create_v2": {"ok": 0, "fail": 0},
        },
    }


def rpc(method, params, url):
    req = urllib.request.Request(
        url, data=json.dumps({"jsonrpc": "2.0", "id": 1, "method": method,
                              "params": params}).encode(),
        headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=30) as r:
        d = json.loads(r.read())
    if "error" in d:
        raise RuntimeError(d["error"].get("message", "rpc error"))
    return d["result"]


def main(argv=None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    ckpt = argv[1] if len(argv) > 1 else DEFAULT_CKPT
    n = int(argv[0]) if len(argv) > 0 else DEFAULT_SAMPLE_N
    url = os.environ.get("SNF_MAINNET_RPC_URL", "")
    assert url, "SNF_MAINNET_RPC_URL not set"

    print(f"create disc:     {DISC_CREATE.hex()} (global:create)")
    print(f"create_v2 disc:  {DISC_CREATE_V2.hex()} (global:create_v2)")

    state = json.load(open(ckpt))
    slots = sorted(state["verified_blocks"])
    lo, hi = slots[0], slots[-1]
    sample = sorted(random.Random(SAMPLE_SEED).sample(slots, min(n, len(slots))))
    print(f"window [{lo}, {hi}]  sampling {len(sample)} slots")

    totals = empty_totals()
    fetched = 0
    for s in sample:
        try:
            b = rpc("getBlock", [s, {"encoding": "json", "transactionDetails": "full",
                                     "rewards": False, "commitment": "confirmed",
                                     "maxSupportedTransactionVersion": 1}], url)
        except Exception as e:
            print(f"  slot {s}: {e}")
            continue
        fetched += 1
        aggregate(totals, scan_block(b))
        time.sleep(0.15)

    print(f"\nfetched {fetched}/{len(sample)} blocks")
    t = totals["all_tx"]["ok"] + totals["all_tx"]["fail"]
    print(f"all txs: {t} (ok {totals['all_tx']['ok']} / failed {totals['all_tx']['fail']})")
    pt = totals["pump_tx"]["ok"] + totals["pump_tx"]["fail"]
    print(f"pump txs: {pt} (ok {totals['pump_tx']['ok']} / failed {totals['pump_tx']['fail']})")
    print(f"create attempts (legacy create):   {totals['create_attempts']['create']}")
    print(f"create attempts (create_v2):       {totals['create_attempts']['create_v2']}")
    print("\ntop pump ixs among SUCCESSFUL:")
    for d, c in totals["disc_ok"].most_common(8):
        print(f"  {d}  x{c}")
    print("top pump ixs among FAILED:")
    for d, c in totals["disc_fail"].most_common(12):
        note = ""
        if bytes.fromhex(d) == DISC_CREATE:
            note = "  <-- global:create"
        elif bytes.fromhex(d) == DISC_CREATE_V2:
            note = "  <-- global:create_v2"
        print(f"  {d}  x{c}{note}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
