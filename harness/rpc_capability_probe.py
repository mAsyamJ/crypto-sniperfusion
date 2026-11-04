#!/usr/bin/env python3
"""Safe, bounded, read-only RPC provider capability probe for H6/G0.

No mainnet transaction methods, no private key loading, no raw endpoint/error
printing. A successful sample block is NOT a COMPLETE creation-cohort proof.
"""
import argparse
import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNS = ROOT / "harness" / "runs"
MAINNET_GENESIS = "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d"
ALLOWED_METHODS = {"getGenesisHash", "getSlot", "getBlock"}
MAX_REQUESTS = 5
MAX_BLOCK_RESPONSE_BYTES = 32_000_000


class ProbeError(Exception):
    def __init__(self, code):
        super().__init__(code)
        self.code = code


def endpoint_from_env(env=None):
    env = os.environ if env is None else env
    if env.get("LIVE_TRADING_ENABLED", "false").lower() != "false":
        raise ProbeError("LIVE_TRADING_MUST_BE_DISABLED")
    url = env.get("SNF_MAINNET_RPC_URL", "")
    if not url:
        raise ProbeError("RPC_URL_NOT_CONFIGURED")
    u = urllib.parse.urlsplit(url)
    if not u.hostname or u.username or u.password:
        raise ProbeError("INVALID_RPC_ENDPOINT")
    local = u.hostname in {"127.0.0.1", "localhost", "::1"}
    if u.scheme != "https" and not (u.scheme == "http" and local):
        raise ProbeError("RPC_MUST_USE_HTTPS_OR_LOOPBACK")
    return url


class RpcClient:
    def __init__(self, url, timeout=5):
        self._url = url
        self._timeout = timeout
        self.calls = 0

    def call(self, method, params=None):
        if method not in ALLOWED_METHODS:
            raise ProbeError("METHOD_NOT_ALLOWED")
        if self.calls >= MAX_REQUESTS:
            raise ProbeError("REQUEST_BUDGET_EXHAUSTED")
        self.calls += 1
        payload = json.dumps({
            "jsonrpc": "2.0", "id": self.calls,
            "method": method, "params": params if params is not None else [],
        }).encode("utf-8")
        req = urllib.request.Request(
            self._url, data=payload, headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=self._timeout) as res:
                raw = res.read(MAX_BLOCK_RESPONSE_BYTES + 1)
                if len(raw) > MAX_BLOCK_RESPONSE_BYTES:
                    raise ProbeError("BLOCK_RESPONSE_TOO_LARGE")
        except urllib.error.HTTPError as exc:
            # Intentionally never include exc string (could contain secret URL).
            if exc.code == 429:
                raise ProbeError("HTTP_429_RATE_LIMITED") from None
            if exc.code == 403:
                raise ProbeError("HTTP_403_FORBIDDEN") from None
            raise ProbeError(f"HTTP_{exc.code}") from None
        except (urllib.error.URLError, TimeoutError, OSError):
            raise ProbeError("TRANSPORT_UNAVAILABLE") from None
        try:
            obj = json.loads(raw)
        except (ValueError, UnicodeDecodeError):
            raise ProbeError("BAD_JSON_RESPONSE") from None
        if not isinstance(obj, dict):
            raise ProbeError("BAD_RPC_RESPONSE")
        if "error" in obj:
            detail = str(obj["error"].get("message", "")) if isinstance(obj["error"], dict) else ""
            if "version" in detail.lower() and ("not support" in detail.lower() or "unsupported" in detail.lower()):
                raise ProbeError("TX_VERSION_UNSUPPORTED")
            raise ProbeError("RPC_RETURNED_ERROR")
        if "result" not in obj:
            raise ProbeError("MISSING_RESULT")
        return obj["result"]


def probe(client):
    result = {
        "status": "UNKNOWN",
        "rpc_source": "OPERATOR_ENV_ONLY",
        "read_only_methods": sorted(ALLOWED_METHODS),
        "coverage_state": "UNKNOWN",
        "promotable": False,
        "sampled_block_count": 0,
        "transaction_version_max_requested": 1,
        "source_url_or_credentials_recorded": False,
    }
    try:
        genesis = client.call("getGenesisHash")
        if genesis != MAINNET_GENESIS:
            result["status"] = "WRONG_CLUSTER"
            return result
        result["genesis_matches_mainnet"] = True
        finalized = client.call("getSlot", [{"commitment": "finalized"}])
        if not isinstance(finalized, int) or finalized < 64:
            raise ProbeError("INVALID_FINALIZED_SLOT")
        result["finalized_slot"] = finalized
        samples = []
        for delta in (8, 16, 32):
            slot = finalized - delta
            # Full transaction metadata, not signatures-only, required to prove
            # an ordered block can be reconstructed in a later cohort capture.
            config = {
                "encoding": "json", "transactionDetails": "full",
                "rewards": False, "maxSupportedTransactionVersion": 1,
                "commitment": "finalized",
            }
            block = client.call("getBlock", [slot, config])
            if block is None:  # legitimate skipped slot; not proof of provider support
                samples.append({"slot": slot, "kind": "SKIPPED_OR_NULL"})
                continue
            if not isinstance(block, dict) or not isinstance(block.get("transactions"), list):
                raise ProbeError("NO_FULL_BLOCK_TRANSACTIONS")
            sample = {
                "slot": slot, "kind": "FULL_BLOCK",
                "transactions": len(block["transactions"]),
            }
            samples.append(sample)
            result["status"] = "FULL_BLOCK_SAMPLE_AVAILABLE"
            result["sampled_block_count"] = 1
            break
        result["samples"] = samples
        if result["sampled_block_count"] == 0:
            result["status"] = "NO_SAMPLE_BLOCK_AVAILABLE"
    except ProbeError as exc:
        result["status"] = "BLOCKED"
        result["error_code"] = exc.code
    finally:
        result["requests_made"] = client.calls
    return result


def write_once(path, result):
    path = Path(path)
    if not path.is_absolute():
        path = ROOT / path
    path = path.resolve()
    if not path.is_relative_to(RUNS.resolve()):
        raise ProbeError("RECEIPT_PATH_OUTSIDE_HARNESS_RUNS")
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x", encoding="utf-8") as f:
        json.dump(result, f, indent=2, sort_keys=True)
        f.write("\n")
        f.flush()
        os.fsync(f.fileno())
    path.chmod(0o444)
    return path


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out", required=True, help="New write-once receipt under harness/runs/")
    args = ap.parse_args(argv)
    try:
        client = RpcClient(endpoint_from_env())
        report = probe(client)
        path = write_once(args.out, report)
        print(json.dumps({"status": report["status"], "receipt": str(path),
                          "requests_made": client.calls, "coverage_state": "UNKNOWN"}, indent=2))
        return 0 if report["status"] == "FULL_BLOCK_SAMPLE_AVAILABLE" else 2
    except (ProbeError, FileExistsError, OSError) as exc:
        # Never print remote exceptions, which may contain credential-bearing URLs.
        code = exc.code if isinstance(exc, ProbeError) else "RECEIPT_WRITE_FAILED"
        print(json.dumps({"status": "BLOCKED", "error_code": code}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
