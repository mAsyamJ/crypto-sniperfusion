#!/usr/bin/env python3
"""Sampled diagnostic on the completed G0 window (2026-11-05).

The checkpoint persists signatures, not block payloads, so the question
"were there FAILED create attempts?" needs a small re-fetch. This samples
N random slots from the completed window, getBlock's each (read-only),
and scans pump-program instructions (outer+inner) by success/failure,
with anchor discriminators. Read-only; env-only endpoint; no keys.
Usage: python3 scripts/diag_sample_failed_creates.py [sample_n] [ckpt]
"""
import base64, collections, hashlib, json, os, random, sys, time, urllib.request

CKPT = sys.argv[2] if len(sys.argv) > 2 else "harness/runs/launch-cohort/ckpt-sweep1.json"
N = int(sys.argv[1]) if len(sys.argv) > 1 else 60
PUMP = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
URL = os.environ.get("SNF_MAINNET_RPC_URL", "")
assert URL, "SNF_MAINNET_RPC_URL not set"

ALPH = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
def b58dec(s: str) -> bytes:
    n = 0
    for c in s:
        n = n * 58 + ALPH.index(c)
    h = n.to_bytes((n.bit_length() + 7) // 8, "big")
    return b"\x00" * (len(s) - len(s.lstrip("1"))) + h

def rpc(method, params):
    req = urllib.request.Request(
        URL, data=json.dumps({"jsonrpc": "2.0", "id": 1, "method": method,
                              "params": params}).encode(),
        headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=30) as r:
        d = json.loads(r.read())
    if "error" in d:
        raise RuntimeError(d["error"].get("message", "rpc error"))
    return d["result"]

DISC_CREATE = hashlib.sha256(b"global:create").digest()[:8]
print(f"create disc: {DISC_CREATE.hex()}")

state = json.load(open(CKPT))
slots = sorted(state["verified_blocks"])
lo, hi = slots[0], slots[-1]
sample = sorted(random.Random(20261105).sample(slots, min(N, len(slots))))
print(f"window [{lo}, {hi}]  sampling {len(sample)} slots")

pump_tx = {"ok": 0, "fail": 0}
disc_ok, disc_fail = collections.Counter(), collections.Counter()
create_attempts = {"ok": 0, "fail": 0}
all_tx = {"ok": 0, "fail": 0}
fetched = 0
for s in sample:
    try:
        b = rpc("getBlock", [s, {"encoding": "json", "transactionDetails": "full",
                                 "rewards": False, "commitment": "confirmed",
                                 "maxSupportedTransactionVersion": 1}])
    except Exception as e:
        print(f"  slot {s}: {e}")
        continue
    fetched += 1
    for tx in b.get("transactions", []):
        txm = tx.get("transaction") or {}
        msg = txm.get("message") or tx.get("message") or {}
        meta = tx.get("meta") or {}
        keys = list(msg.get("accountKeys", []))
        la = meta.get("loadedAddresses") or {}
        keys += la.get("writable", []) + la.get("readonly", [])  # versioned txs: ALT keys
        ok = meta.get("err") is None
        all_tx["ok" if ok else "fail"] += 1
        if not any(k == PUMP for k in keys):
            continue
        pump_tx["ok" if ok else "fail"] += 1
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
            (disc_ok if ok else disc_fail)[data[:8].hex()] += 1
            if data[:8] == DISC_CREATE:
                create_attempts["ok" if ok else "fail"] += 1
    time.sleep(0.15)

print(f"\nfetched {fetched}/{len(sample)} blocks")
t = all_tx["ok"] + all_tx["fail"]
print(f"all txs: {t} (ok {all_tx['ok']} / failed {all_tx['fail']})")
pt = pump_tx["ok"] + pump_tx["fail"]
print(f"pump txs: {pt} (ok {pump_tx['ok']} / failed {pump_tx['fail']})")
print(f"create attempts: {create_attempts}")
print("\ntop pump ixs among SUCCESSFUL:")
for d, c in disc_ok.most_common(8):
    print(f"  {d}  x{c}")
print("top pump ixs among FAILED:")
for d, c in disc_fail.most_common(12):
    mark = "  <-- global:create" if bytes.fromhex(d) == DISC_CREATE else ""
    print(f"  {d}  x{c}{mark}")
