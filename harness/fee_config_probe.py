#!/usr/bin/env python3
"""Read-only on-chain fee-configuration probe (PF0).

Decodes the four Pump/PumpSwap fee accounts on Mainnet-beta so P&L fee
provenance comes from OBSERVED account data instead of doc constants:

  - pump Global                 4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf
  - PumpSwap GlobalConfig       ADyA8hdefvWN2dbGGWFotWxrAvLW83WG6QCVXvJKqw
  - curve FeeConfig (pump-fees) 8Wf5TiAheLUqBrKXeYg2JtAFFMWtKdG2BSFgqUcPVwTt
  - AMM FeeConfig (pump-fees)   5PHirr8joyTMp9JMm6nW7hNDVyEYdkzDqazxPD7RaTjx

Layouts are the pinned pump-docs IDLs (anchor: 8-byte account discriminator
then borsh fields; decodes are prefix-tolerant where the IDL appends
trailing fields). This settles the PF0 CE-2 discrepancy: the 25 bps
`canonical_reference` is only the feeConfig==null fallback; canonical pools
are market-cap-TIERED from these accounts.

Strictly read-only; no keys; receipt write-once 0444; nonpromotable.
"""

import argparse
import base64
import hashlib
import json
import os
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from launch_cohort_capture import (  # noqa: E402
    JsonRpc, MAINNET_GENESIS, RpcError, WrongCluster, require_safe_env, write_once,
)

PUMP_GLOBAL = "4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf"
AMM_GLOBAL_CONFIG = "ADyA8hdefvWN2dbGGWFotWxrAvLW83WG6QCVXvJKqw"
CURVE_FEE_CONFIG = "8Wf5TiAheLUqBrKXeYg2JtAFFMWtKdG2BSFgqUcPVwTt"
AMM_FEE_CONFIG = "5PHirr8joyTMp9JMm6nW7hNDVyEYdkzDqazxPD7RaTjx"

EVIDENCE_CLASS = "MAINNET_READ_ONLY_OBSERVED"


def anchor_account_discriminator(name: str) -> bytes:
    import hashlib
    return hashlib.sha256(f"account:{name}".encode()).digest()[:8]


class _Reader:
    """Prefix-tolerant borsh reader (mirrors the Rust decoder style)."""

    def __init__(self, data):
        self.data = data
        self.pos = 0

    def _take(self, n):
        if self.pos + n > len(self.data):
            raise EOFError
        chunk = self.data[self.pos:self.pos + n]
        self.pos += n
        return chunk

    def u8(self):
        return self._take(1)[0]

    def bool(self):
        return self.u8() != 0

    def u64(self):
        return int.from_bytes(self._take(8), "little")

    def u128(self):
        return int.from_bytes(self._take(16), "little")

    def pubkey(self):
        return self._take(32).hex()

    def vec(self, item_fn):
        n = int.from_bytes(self._take(4), "little")
        if n > 64:
            raise ValueError(f"implausible vec length {n}")
        return [item_fn(self) for _ in range(n)]


def decode_fees(r: _Reader):
    return {
        "lp_fee_bps": r.u64(),
        "protocol_fee_bps": r.u64(),
        "creator_fee_bps": r.u64(),
    }


def decode_fee_tier(r: _Reader):
    return {
        "market_cap_lamports_threshold": r.u128(),
        "fees": decode_fees(r),
    }


def decode_fee_config(data: bytes):
    """FeeConfig per pump_fees.json IDL (used for BOTH curve and AMM PDAs)."""
    disc = anchor_account_discriminator("FeeConfig")
    if data[:8] != disc:
        return {"decode_ok": False,
                "reason": "discriminator mismatch for account:FeeConfig"}
    r = _Reader(data[8:])
    try:
        return {
            "decode_ok": True,
            "bump": r.u8(),
            "admin": r.pubkey(),
            "flat_fees": decode_fees(r),
            "fee_tiers": r.vec(decode_fee_tier),
            "stable_fee_tiers": r.vec(decode_fee_tier),
            "exotic_flat_fees": decode_fees(r),
        }
    except (EOFError, ValueError) as exc:
        return {"decode_ok": False, "reason": f"truncated: {exc}"}


def decode_amm_global_config(data: bytes):
    """PumpSwap GlobalConfig per pump_amm.json IDL, prefix-tolerant."""
    disc = anchor_account_discriminator("GlobalConfig")
    if data[:8] != disc:
        return {"decode_ok": False,
                "reason": "discriminator mismatch for account:GlobalConfig"}
    r = _Reader(data[8:])
    out = {"decode_ok": True, "admin": r.pubkey(),
           "lp_fee_basis_points": r.u64(),
           "protocol_fee_basis_points": r.u64()}
    try:
        out["disable_flags"] = r.u8()
        # protocol_fee_recipients: fixed [pubkey; 8]
        for _ in range(8):
            r.pubkey()
        # IDL-appended trailing fields (present on current accounts):
        out["coin_creator_fee_basis_points"] = r.u64()
        r.pubkey()  # admin_set_coin_creator_authority
        r.pubkey()  # whitelist_pda
        r.pubkey()  # reserved_fee_recipient
        out["mayhem_mode_enabled"] = r.bool()
        for _ in range(7):
            r.pubkey()  # reserved_fee_recipients
        out["is_cashback_enabled"] = r.bool()
    except EOFError:
        out["trailing_fields_decoded"] = False
    return out


def decode_pump_global(data: bytes):
    """Pump bonding-curve Global per pump.json IDL, prefix-tolerant."""
    disc = anchor_account_discriminator("Global")
    if data[:8] != disc:
        return {"decode_ok": False,
                "reason": "discriminator mismatch for account:Global"}
    r = _Reader(data[8:])
    try:
        out = {
            "decode_ok": True,
            "initialized": r.bool(),
            "authority": r.pubkey(),
            "fee_recipient": r.pubkey(),
            "initial_virtual_token_reserves": r.u64(),
            "initial_virtual_sol_reserves": r.u64(),
            "initial_real_token_reserves": r.u64(),
            "token_total_supply": r.u64(),
            "fee_basis_points": r.u64(),
        }
        r.pubkey()  # withdraw_authority
        out["enable_migrate"] = r.bool()
        out["pool_migration_fee"] = r.u64()
        out["creator_fee_basis_points"] = r.u64()
    except EOFError as exc:
        return {"decode_ok": False, "reason": f"truncated: {exc}"}
    return out


DECODERS = {
    PUMP_GLOBAL: ("pump_global", decode_pump_global),
    AMM_GLOBAL_CONFIG: ("amm_global_config", decode_amm_global_config),
    CURVE_FEE_CONFIG: ("curve_fee_config", decode_fee_config),
    AMM_FEE_CONFIG: ("amm_fee_config", decode_fee_config),
}


def probe(rpc: JsonRpc, out_dir):
    """Read + decode all four fee accounts; write-once receipt. Read-only."""
    require_safe_env()
    genesis = rpc.get_genesis_hash()
    if genesis != MAINNET_GENESIS:
        raise WrongCluster(genesis, MAINNET_GENESIS)
    slot_before = rpc.get_slot()

    accounts = {}
    for address, (label, decoder) in DECODERS.items():
        entry = {"label": label, "address": address}
        try:
            info = rpc.get_account_info(address)
        except RpcError as exc:
            entry["error"] = str(exc)
            accounts[address] = entry
            continue
        value = (info or {}).get("value") or {}
        entry["exists"] = value.get("data") is not None
        entry["owner"] = value.get("owner")
        entry["lamports"] = value.get("lamports")
        if value.get("data"):
            raw = base64.b64decode(value["data"][0])
            entry["data_len"] = len(raw)
            entry["data_sha256"] = hashlib.sha256(raw).hexdigest()
            entry.update(decoder(raw))
        accounts[address] = entry

    slot_after = rpc.get_slot()
    stamp = time.strftime("%Y%m%dT%H%M%S", time.gmtime()) + f"-{os.getpid()}"
    receipt = {
        "schema_version": 1,
        "evidence_class": EVIDENCE_CLASS,
        "nonpromotable": True,
        "live_trading_enabled": False,
        "read_only": True,
        "mainnet_genesis_verified": genesis,
        "slot_before": slot_before,
        "slot_after": slot_after,
        "accounts": accounts,
        "discrepancy_check": {
            "canonical_reference_bps_in_code": 25,
            "note": "fees.rs canonical_reference is the feeConfig==null "
                    "GlobalConfig fallback; observed fee_tiers above are the "
                    "real canonical-pool pricing (PF0 CE-2)",
        },
    }
    path = Path(out_dir) / f"fee-config-{stamp}.json"
    write_once(path, json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    return {"receipt_path": str(path), "receipt": receipt}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", default=os.environ.get("SNF_MAINNET_RPC_URL",
                                                        "https://api.mainnet-beta.solana.com"))
    parser.add_argument("--out-dir", default="harness/runs/fee-config")
    args = parser.parse_args(argv)
    rpc = JsonRpc(url=args.url)
    result = probe(rpc, args.out_dir)
    accounts = result["receipt"]["accounts"]
    summary = {addr: {k: ent.get(k) for k in ("label", "decode_ok", "data_len")}
               for addr, ent in accounts.items()}
    print(json.dumps({"receipt_path": result["receipt_path"],
                      "nonpromotable": True, "accounts": summary},
                     indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
