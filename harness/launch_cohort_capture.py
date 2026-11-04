#!/usr/bin/env python3
"""Bounded read-only Pump.fun creation-cohort collector (H6-2).

Sweeps an EXPLICIT slot interval on Mainnet-beta (strictly read-only),
reconciles signature-discovered candidates against block-level evidence, and
writes immutable raw evidence plus a content-addressed coverage manifest.

CRITICAL: `getSignaturesForAddress` alone can NEVER establish a complete
creation cohort. Completeness comes only from `getBlock` coverage of every
slot in the window plus signature/block reconciliation. When the provider
cannot prove the interval, the coverage state is INCOMPLETE/UNKNOWN — missing
data is never silently interpreted as an inactive token.

Every observed creation is preserved, including mints that never traded,
never graduated, or failed. No keys, no signing, no transaction submission.
Outputs are permanently nonpromotable until independent coverage
verification succeeds.
"""

import argparse
import base64
import hashlib
import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

PUMP_PROGRAM_ID = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
MAINNET_GENESIS = "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d"
DISC_CREATE_EVENT = bytes([0x1B, 0x72, 0xA9, 0x4D, 0xDE, 0xEB, 0x63, 0x76])
EVIDENCE_CLASS = "MAINNET_READ_ONLY_CAPTURE"
MAX_WINDOW_SLOTS = 10_000
DEFAULT_PUBLIC_RPC = "https://api.mainnet-beta.solana.com"

_B58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


class EnvRefused(Exception):
    """Unsafe environment (live flag or key material) for a read-only capture."""


class WrongCluster(Exception):
    """Endpoint genesis does not match pinned Mainnet-beta identity."""

    def __init__(self, actual, expected):
        super().__init__(f"endpoint genesis {actual} != mainnet {expected}")
        self.actual = actual
        self.expected = expected


class RpcError(Exception):
    def __init__(self, message, code=None):
        super().__init__(message)
        self.code = code


def require_safe_env():
    live = os.environ.get("LIVE_TRADING_ENABLED", "false").lower()
    if live not in ("", "false", "0"):
        raise EnvRefused("read-only capture refuses LIVE_TRADING_ENABLED")
    for key in ("PRIVATE_KEY", "SOLANA_PRIVATE_KEY", "MNEMONIC", "SEED_PHRASE"):
        if os.environ.get(key):
            raise EnvRefused(f"read-only capture refuses key material in env ({key})")


def b58encode(data: bytes) -> str:
    num = int.from_bytes(data, "big")
    out = []
    while num > 0:
        num, rem = divmod(num, 58)
        out.append(_B58_ALPHABET[rem])
    for byte in data:
        if byte == 0:
            out.append("1")
        else:
            break
    return "".join(reversed(out))


def sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class _Cursor:
    """Borsh-style prefix-tolerant reader (mirrors the Rust decoder)."""

    def __init__(self, data):
        self.data = data
        self.pos = 0

    def _take(self, n):
        if self.pos + n > len(self.data):
            return None
        chunk = self.data[self.pos:self.pos + n]
        self.pos += n
        return chunk

    def u32(self):
        b = self._take(4)
        return None if b is None else int.from_bytes(b, "little")

    def i64(self):
        b = self._take(8)
        return None if b is None else int.from_bytes(b, "little", signed=True)

    def u64(self):
        b = self._take(8)
        return None if b is None else int.from_bytes(b, "little")

    def boolean(self):
        b = self._take(1)
        return None if b is None else bool(b[0])

    def string(self):
        n = self.u32()
        if n is None or n > 10_000:
            return None
        b = self._take(n)
        return None if b is None else b.decode("utf-8", errors="replace")

    def pubkey(self):
        b = self._take(32)
        return None if b is None else b58encode(b)


def decode_create_event(body: bytes):
    """Decode a CreateEvent body (after the 8-byte discriminator).

    Mirrors sniper-protocols `decode_create`: prefix-tolerant — a field that
    hits EOF ends the decode with the fields decoded so far. Prefixes shorter
    than the identity fields are not usable events (None).
    """
    r = _Cursor(body)
    name = r.string()
    symbol = r.string()
    uri = r.string()
    mint = r.pubkey()
    bonding_curve = r.pubkey()
    user = r.pubkey()
    creator = r.pubkey()
    timestamp = r.i64()
    if any(v is None for v in (name, symbol, uri, mint, bonding_curve, user, creator, timestamp)):
        return None
    event = {
        "name": name, "symbol": symbol, "uri": uri,
        "mint": mint, "bonding_curve": bonding_curve, "user": user,
        "creator": creator, "timestamp": timestamp,
        "virtual_token_reserves": 0, "virtual_sol_reserves": 0,
        "real_token_reserves": 0, "token_total_supply": 0,
        "token_program": None, "is_mayhem_mode": False,
        "is_cashback_enabled": False, "quote_mint": None,
        "virtual_quote_reserves": 0, "creator_fee_bps": 0,
        "is_holder_reward": False,
        "decoded_fields": 8,
    }
    fields = (
        ("virtual_token_reserves", r.u64()),
        ("virtual_sol_reserves", r.u64()),
        ("real_token_reserves", r.u64()),
        ("token_total_supply", r.u64()),
        ("token_program", r.pubkey()),
        ("is_mayhem_mode", r.boolean()),
        ("is_cashback_enabled", r.boolean()),
        ("quote_mint", r.pubkey()),
        ("virtual_quote_reserves", r.u64()),
        ("creator_fee_bps", r.u64()),
        ("is_holder_reward", r.boolean()),
    )
    n = 8
    for key, value in fields:
        if value is None:
            event["decoded_fields"] = n
            break
        event[key] = value
        n += 1
    else:
        event["decoded_fields"] = n
    event["is_truncated"] = event["decoded_fields"] < 19
    return event


def extract_creates_from_logs(logs, program_id):
    """Extract decoded create events from one transaction's log messages.

    The create gate matches whole `Instruction: Create` / `Instruction:
    CreateV2` lines (other programs' create-named instructions cannot pass).
    `Program data:` lines are attributed to the most recent instruction
    marker; non-create instructions clear the attribution.
    """
    creates = []
    variant = None
    ordinal = 0
    prefix_data = "Program data: "
    for line in logs:
        if line == "Program log: Instruction: Create":
            variant = "create"
        elif line == "Program log: Instruction: CreateV2":
            variant = "create_v2"
        elif line.startswith("Program log: Instruction: "):
            variant = None
        elif line.startswith(prefix_data) and variant is not None:
            try:
                payload = base64.b64decode(line[len(prefix_data):])
            except Exception:
                continue
            if len(payload) < 8 or bytes(payload[:8]) != DISC_CREATE_EVENT:
                continue
            event = decode_create_event(payload[8:])
            if event is None:
                continue
            event["instruction_variant"] = variant
            event["event_ordinal"] = ordinal
            event["payload_sha256"] = sha256_hex(payload)
            ordinal += 1
            creates.append(event)
    return creates


class JsonRpc:
    """Minimal JSON-RPC client with an injectable transport for offline tests."""

    def __init__(self, url=DEFAULT_PUBLIC_RPC, transport=None, timeout=30):
        self.url = url
        self.timeout = timeout
        self._transport = transport or self._http_transport

    def _http_transport(self, body):
        req = urllib.request.Request(
            self.url,
            data=json.dumps(body).encode(),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                return json.loads(resp.read().decode())
        except urllib.error.HTTPError as exc:
            # HTTP-level failures (429 rate limits, 5xx) must surface as
            # RpcError so bounded retry / gap recording handles them instead
            # of crashing the capture.
            raise RpcError(f"HTTP {exc.code}: {exc.reason}", code=exc.code) from exc

    def _call(self, method, params):
        body = {"jsonrpc": "2.0", "id": 1, "method": method, "params": params}
        payload = self._transport(body)
        if "error" in payload:
            err = payload["error"]
            raise RpcError(err.get("message", "rpc error"), code=err.get("code"))
        return payload.get("result")

    def get_genesis_hash(self):
        return self._call("getGenesisHash", [])

    def get_slot(self, commitment="confirmed"):
        return self._call("getSlot", [{"commitment": commitment}])

    def get_block(self, slot):
        return self._call("getBlock", [slot, {
            "encoding": "json",
            "transactionDetails": "full",
            "rewards": False,
            "maxSupportedTransactionVersion": 0,
        }])

    def get_signatures_for_address(self, address, before=None, limit=1000):
        params = [address, {"limit": min(int(limit), 1000)}]
        if before:
            params[1]["before"] = before
        return self._call("getSignaturesForAddress", params)

    def get_account_info(self, address):
        return self._call("getAccountInfo", [address, {"encoding": "base64"}])

    def get_transaction(self, signature):
        return self._call("getTransaction", [signature, {
            "encoding": "json",
            "maxSupportedTransactionVersion": 0,
        }])


def with_retry(fn, retries, sleep, sleep_base=0.5):
    """Bounded, deterministic retry: initial attempt + `retries` retries."""
    attempt = 0
    while True:
        try:
            return fn()
        except RpcError as exc:
            if attempt >= retries:
                raise exc
            attempt += 1
            sleep(sleep_base * attempt)


def write_once(path: Path, text: str):
    """Exclusive-create, fsync, chmod 0444. Refuses to overwrite."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "x", encoding="utf-8") as handle:
        handle.write(text)
        handle.flush()
        os.fsync(handle.fileno())
    os.chmod(path, 0o444)


def _validate_window(from_slot, to_slot):
    if from_slot < 1 or to_slot < from_slot:
        raise ValueError(f"invalid window [{from_slot}, {to_slot}]")
    if to_slot - from_slot + 1 > MAX_WINDOW_SLOTS:
        raise ValueError(
            f"window spans {to_slot - from_slot + 1} slots > hard cap {MAX_WINDOW_SLOTS}"
        )


def _capture_window(rpc, genesis, from_slot, to_slot, out_dir, *, max_pages,
                    retries, sleep, sleep_base):
    """Collection core for an already-validated window (identity checked)."""
    tip = rpc.get_slot()
    right_censored = None
    collect_to = to_slot
    if to_slot > tip:
        right_censored = (
            f"window extends past chain tip ({to_slot} > {tip}); "
            "slots beyond the tip were not collected"
        )
        collect_to = tip
    if from_slot > tip:
        right_censored = right_censored or f"window starts past tip ({from_slot} > {tip})"

    gaps = []

    # --- signature sweep (candidate discovery only; NEVER a completeness proof)
    in_window_sigs = set()
    cursor = None
    pages_fetched = 0
    for _ in range(max_pages):
        page = rpc.get_signatures_for_address(PUMP_PROGRAM_ID, before=cursor)
        pages_fetched += 1
        if not page:
            break
        oldest = None
        for item in page:
            slot = item.get("slot") or 0
            sig = item["signature"]
            if from_slot <= slot <= collect_to:
                in_window_sigs.add(sig)
            oldest = item
        if oldest is None or (oldest.get("slot") or 0) < from_slot:
            break
        cursor = oldest["signature"]

    # --- block sweep (the only completeness evidence)
    creates = []
    verified_blocks = []
    block_sigs = set()
    failed_transactions = 0
    for slot in range(from_slot, collect_to + 1):
        try:
            block = with_retry(lambda s=slot: rpc.get_block(s), retries, sleep, sleep_base)
        except RpcError as exc:
            kind = "RateLimited" if exc.code == 429 else "ProviderUnavailable"
            gaps.append({"kind": kind, "slot": slot,
                         "evidence": f"getBlock failed after {retries + 1} attempts: {exc}"})
            continue
        if block is None:
            gaps.append({"kind": "ProviderUnavailable", "slot": slot,
                         "evidence": "getBlock returned null (skipped or unavailable)"})
            continue
        verified_blocks.append(slot)
        for tx_index, tx in enumerate(block.get("transactions") or []):
            sig = tx["transaction"]["signatures"][0]
            block_sigs.add(sig)
            meta = tx.get("meta") or {}
            if meta.get("err") is not None:
                failed_transactions += 1
                continue
            keys = list((tx["transaction"]["message"].get("accountKeys") or []))
            keys += list((meta.get("loadedAddresses") or []))
            if PUMP_PROGRAM_ID not in keys:
                continue
            for event in extract_creates_from_logs(meta.get("logMessages") or [], PUMP_PROGRAM_ID):
                record = dict(event)
                record.update({
                    "signature": sig,
                    "slot": slot,
                    "transaction_index": tx_index,
                    "block_evidence": True,
                })
                creates.append(record)

    # --- reconciliation: feed signatures vs block evidence
    signatures_without_block_evidence = []
    for sig in sorted(in_window_sigs - block_sigs):
        signatures_without_block_evidence.append(sig)
        try:
            tx = with_retry(lambda s=sig: rpc.get_transaction(s), retries, sleep, sleep_base)
        except RpcError as exc:
            gaps.append({"kind": f"MissingMetadata:{sig}",
                         "evidence": f"getTransaction failed: {exc}"})
            continue
        if tx is None:
            gaps.append({"kind": f"MissingMetadata:{sig}",
                         "evidence": "getTransaction returned null (pruned or dropped)"})
            continue
        # Transaction recovered WITHOUT block evidence: decode any creates
        # from its logs, flagged block_evidence=False and transaction_index
        # unknown (never fabricated). The interval still cannot be proven
        # complete without block coverage, so a NoBlockEvidence gap remains.
        gaps.append({"kind": f"NoBlockEvidence:{sig}",
                     "evidence": f"transaction recovered at slot {tx.get('slot')} "
                                 "without block evidence; interval completeness unprovable"})
        meta = tx.get("meta") or {}
        if meta.get("err") is not None:
            continue
        for event in extract_creates_from_logs(meta.get("logMessages") or [], PUMP_PROGRAM_ID):
            record = dict(event)
            record.update({
                "signature": sig,
                "slot": tx.get("slot") or 0,
                "transaction_index": None,
                "block_evidence": False,
            })
            creates.append(record)

    if not verified_blocks and not creates and not in_window_sigs:
        coverage_state = "UNKNOWN"
    elif right_censored:
        coverage_state = "CENSORED"
    elif gaps:
        coverage_state = "INCOMPLETE"
    elif len(verified_blocks) == collect_to - from_slot + 1:
        coverage_state = "COMPLETE"
    else:
        coverage_state = "INCOMPLETE"

    creates.sort(key=lambda c: (c["slot"],
                                c["transaction_index"]
                                if c["transaction_index"] is not None else 1 << 62,
                                c["event_ordinal"]))

    stamp = time.strftime("%Y%m%dT%H%M%S", time.gmtime()) + f"-{os.getpid()}"
    raw_path = Path(out_dir) / f"raw_creates-{stamp}.jsonl"
    raw_text = "".join(json.dumps(c, sort_keys=True) + "\n" for c in creates)
    write_once(raw_path, raw_text)
    digest = sha256_hex(raw_path.read_bytes())

    manifest = {
        "schema_version": 1,
        "evidence_class": EVIDENCE_CLASS,
        "nonpromotable": True,
        "live_trading_enabled": False,
        "program_id": PUMP_PROGRAM_ID,
        "mainnet_genesis_verified": genesis,
        "window": {"from_slot": from_slot, "to_slot": to_slot,
                   "collected_to_slot": collect_to, "tip_slot": tip},
        "coverage_state": coverage_state,
        "right_censor_reason": right_censored,
        "verified_blocks": verified_blocks,
        "gaps": gaps,
        "dataset_sha256": digest,
        "raw_creates_count": len(creates),
        "unique_mints": len({c["mint"] for c in creates}),
        "in_window_signatures": len(in_window_sigs),
        "signatures_without_block_evidence": signatures_without_block_evidence,
        "failed_transactions_in_blocks": failed_transactions,
        "pages_fetched": pages_fetched,
        "completeness_note": (
            "coverage derives from getBlock evidence over every slot plus "
            "signature reconciliation; getSignaturesForAddress alone never "
            "proves a complete creation cohort"
        ),
    }
    manifest_path = Path(out_dir) / f"manifest-{stamp}-{digest[:16]}.json"
    write_once(manifest_path, json.dumps(manifest, indent=2, sort_keys=True) + "\n")

    return {
        "coverage_state": coverage_state,
        "creates": creates,
        "gaps": gaps,
        "verified_blocks": verified_blocks,
        "reconciliation": {"signatures_without_block_evidence": signatures_without_block_evidence},
        "blocks_fetched": len(verified_blocks),
        "pages_fetched": pages_fetched,
        "raw_path": str(raw_path),
        "manifest_path": str(manifest_path),
        "manifest": manifest,
        "nonpromotable": True,
        "evidence_class": EVIDENCE_CLASS,
    }


def _checked_identity(rpc):
    require_safe_env()
    genesis = rpc.get_genesis_hash()
    if genesis != MAINNET_GENESIS:
        raise WrongCluster(genesis, MAINNET_GENESIS)
    return genesis


def capture(rpc, from_slot, to_slot, out_dir, *, max_pages=20, retries=3,
            sleep=time.sleep, sleep_base=0.5):
    """Explicit-window capture: the caller names the slot interval."""
    genesis = _checked_identity(rpc)
    _validate_window(from_slot, to_slot)
    return _capture_window(rpc, genesis, from_slot, to_slot, out_dir,
                           max_pages=max_pages, retries=retries,
                           sleep=sleep, sleep_base=sleep_base)


def capture_recent_slots(rpc, lookback_slots, out_dir, *, max_pages=20, retries=3,
                         sleep=time.sleep, sleep_base=0.5):
    """Tip-relative capture: window = [tip - lookback, tip] with the tip
    fetched immediately before the sweep, avoiding the stale-window race
    when high program throughput makes a pre-computed window drift out of
    the signature feed's first pages."""
    if lookback_slots < 1 or lookback_slots > MAX_WINDOW_SLOTS - 1:
        raise ValueError(f"lookback_slots must be in 1..{MAX_WINDOW_SLOTS - 1}")
    genesis = _checked_identity(rpc)
    tip = rpc.get_slot()
    from_slot = tip - lookback_slots
    return _capture_window(rpc, genesis, from_slot, tip, out_dir,
                           max_pages=max_pages, retries=retries,
                           sleep=sleep, sleep_base=sleep_base)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", default=os.environ.get("SNF_MAINNET_RPC_URL", DEFAULT_PUBLIC_RPC))
    window = parser.add_mutually_exclusive_group(required=True)
    window.add_argument("--from-slot", type=int, help="explicit window start (with --to-slot)")
    window.add_argument("--recent-slots", type=int,
                        help="tip-relative window [tip - N, tip], race-free")
    parser.add_argument("--to-slot", type=int)
    parser.add_argument("--out-dir", default="harness/runs/launch-cohort")
    parser.add_argument("--max-pages", type=int, default=20)
    parser.add_argument("--retries", type=int, default=3)
    args = parser.parse_args(argv)
    rpc = JsonRpc(url=args.url)
    if args.recent_slots is not None:
        report = capture_recent_slots(rpc, args.recent_slots, args.out_dir,
                                      max_pages=args.max_pages, retries=args.retries)
    else:
        if args.to_slot is None:
            parser.error("--from-slot requires --to-slot")
        report = capture(rpc, args.from_slot, args.to_slot, args.out_dir,
                         max_pages=args.max_pages, retries=args.retries)
    summary = {k: report[k] for k in ("coverage_state", "blocks_fetched", "pages_fetched",
                                      "raw_path", "manifest_path", "nonpromotable")}
    summary["creates"] = len(report["creates"])
    summary["gaps"] = len(report["gaps"])
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
