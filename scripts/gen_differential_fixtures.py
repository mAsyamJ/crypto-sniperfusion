#!/usr/bin/env python3
"""Generate differential fixtures: random valid pump.fun Anchor events,
borsh-encoded per the IDL layout, decoded by the CHAINSTACK ORACLE
(src/utils/idl_parser.py + vendored IDL) as ground truth.

Run from the chainstack checkout via uv so the oracle imports resolve:

    cd upstream/chainstack && uv run python ../../scripts/gen_differential_fixtures.py \
        --out ../../fixtures/m2_events.oracle.jsonl --count 40

Output JSONL: {"index", "event", "payload_b64", "oracle": {...}|null}
No network, no keys.
"""

from __future__ import annotations

import argparse
import base64
import json
import random
import struct
import sys
from pathlib import Path

# Make chainstack's src/ importable (oracle, Apache-2.0 behavioral baseline).
CHAINSTACK = Path(__file__).resolve().parents[1] / "upstream" / "chainstack"
sys.path.insert(0, str(CHAINSTACK / "src"))

from utils.idl_parser import IDLParser  # noqa: E402  oracle under test

IDL_PATH = CHAINSTACK / "idl" / "pump_fun_idl.json"

# Field layouts from the vendored IDL `types` section (identical shapes to the
# official pump-docs IDL where both define them).
CREATE_FIELDS = [
    ("name", "string"), ("symbol", "string"), ("uri", "string"),
    ("mint", "pubkey"), ("bonding_curve", "pubkey"), ("user", "pubkey"),
    ("creator", "pubkey"), ("timestamp", "i64"),
    ("virtual_token_reserves", "u64"), ("virtual_sol_reserves", "u64"),
    ("real_token_reserves", "u64"), ("token_total_supply", "u64"),
    ("token_program", "pubkey"), ("is_mayhem_mode", "bool"),
    ("is_cashback_enabled", "bool"),
    # trailing, optional-era:
    ("quote_mint", "pubkey"), ("virtual_quote_reserves", "u64"),
    ("creator_fee_bps", "u64"), ("is_holder_reward", "bool"),
]

TRADE_FIELDS = [
    ("mint", "pubkey"), ("sol_amount", "u64"), ("token_amount", "u64"),
    ("is_buy", "bool"), ("user", "pubkey"), ("timestamp", "i64"),
    ("virtual_sol_reserves", "u64"), ("virtual_token_reserves", "u64"),
    ("real_sol_reserves", "u64"), ("real_token_reserves", "u64"),
    ("fee_recipient", "pubkey"), ("fee_basis_points", "u64"), ("fee", "u64"),
    ("creator", "pubkey"), ("creator_fee_basis_points", "u64"), ("creator_fee", "u64"),
    ("track_volume", "bool"), ("total_unclaimed_tokens", "u64"),
    ("total_claimed_tokens", "u64"), ("current_sol_volume", "u64"),
    ("last_update_timestamp", "i64"), ("ix_name", "string"),
    ("mayhem_mode", "bool"), ("cashback_fee_basis_points", "u64"), ("cashback", "u64"),
    ("buyback_fee_basis_points", "u64"), ("buyback_fee", "u64"),
    ("shareholders", "vec<Shareholder>"),
    ("quote_mint", "pubkey"), ("quote_amount", "u64"),
    ("virtual_quote_reserves", "u64"), ("real_quote_reserves", "u64"),
    ("holder_rewards_bps", "u64"), ("holder_rewards", "u64"),
]

COMPLETE_FIELDS = [
    ("user", "pubkey"), ("mint", "pubkey"), ("bonding_curve", "pubkey"),
    ("timestamp", "i64"), ("quote_mint", "pubkey"),
]

MIGRATE_CREATOR_FIELDS = [
    ("timestamp", "i64"), ("mint", "pubkey"), ("bonding_curve", "pubkey"),
    ("sharing_config", "pubkey"), ("old_creator", "pubkey"), ("new_creator", "pubkey"),
]


def rand_bytes(rng: random.Random, n: int) -> bytes:
    return bytes(rng.getrandbits(8) for _ in range(n))


def encode_value(rng: random.Random, ty: str, spec: str | None) -> tuple[bytes, object]:
    """Encode one field; return (bytes, python-value-for-oracle-crosscheck)."""
    if ty == "string":
        s = "".join(rng.choice("abcdefABCDEF0123456789 ") for _ in range(rng.randint(0, 24)))
        return struct.pack("<I", len(s)) + s.encode(), s
    if ty == "pubkey":
        b = rand_bytes(rng, 32)
        return b, b
    if ty == "u64":
        v = rng.getrandbits(64)
        return struct.pack("<Q", v), v
    if ty == "i64":
        v = rng.getrandbits(63) * (1 if rng.getrandbits(1) else -1)
        return struct.pack("<q", v), v
    if ty == "bool":
        v = rng.getrandbits(1)
        return bytes([v]), bool(v)
    if ty == "u16":
        v = rng.getrandbits(16)
        return struct.pack("<H", v), v
    if ty == "vec<Shareholder>":
        n = rng.randint(0, 4)
        body = b""
        vals = []
        for _ in range(n):
            addr = rand_bytes(rng, 32)
            bps = rng.getrandbits(16)
            body += addr + struct.pack("<H", bps)
            vals.append({"address": addr, "share_bps": bps})
        return struct.pack("<I", n) + body, vals
    raise ValueError(f"unknown type {ty} (spec={spec})")


def build_event(rng: random.Random, name: str, fields: list, cut_trailing: int = 0):
    """Encode discriminator + borsh body. cut_trailing drops N trailing fields
    (old-format events)."""
    disc_table = {
        "CreateEvent": bytes.fromhex("1b72a94ddeeb6376"),
        "TradeEvent": bytes.fromhex("bddb7fd34ee661ee"),
        "CompleteEvent": bytes.fromhex("5f72619cd42e9808"),
        "MigrateBondingCurveCreatorEvent": bytes.fromhex("9ba768dcd56cf303"),
    }
    keep = fields[: len(fields) - cut_trailing] if cut_trailing else fields
    body = b""
    expected = {}
    for fname, ftype in keep:
        enc, val = encode_value(rng, ftype, None)
        body += enc
        expected[fname] = val
    return disc_table[name] + body, expected, len(keep)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--count", type=int, default=40, help="events per kind")
    ap.add_argument("--seed", type=int, default=20261102)
    args = ap.parse_args()

    rng = random.Random(args.seed)
    parser = IDLParser(str(IDL_PATH))

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    kinds = [
        ("CreateEvent", CREATE_FIELDS, 0),
        ("CreateEvent", CREATE_FIELDS, 5),  # old format: no trailing fields
        ("CreateEvent", CREATE_FIELDS, 1),
        ("TradeEvent", TRADE_FIELDS, 0),
        ("TradeEvent", TRADE_FIELDS, 6),
        ("CompleteEvent", COMPLETE_FIELDS, 0),
        ("CompleteEvent", COMPLETE_FIELDS, 1),
        ("MigrateBondingCurveCreatorEvent", MIGRATE_CREATOR_FIELDS, 0),
    ]

    n_written = 0
    with out_path.open("w") as fh:
        idx = 0
        for name, fields, cut in kinds:
            for _ in range(args.count):
                payload, expected, n_fields = build_event(rng, name, fields, cut)
                oracle = parser.decode_event_data(payload, name)
                def _default(o):
                    if isinstance(o, bytes):
                        return base64.b64encode(o).decode()  # how the oracle ships pubkeys
                    raise TypeError(o)

                rec = {
                    "index": idx,
                    "event": name,
                    "n_fields": n_fields,
                    "truncated": bool(cut),
                    "payload_b64": base64.b64encode(payload).decode(),
                    "oracle": oracle,
                    "oracle_expected_shape": expected,
                }
                fh.write(json.dumps(rec, default=_default) + "\n")
                idx += 1
                n_written += 1
    print(f"wrote {n_written} fixtures to {out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
