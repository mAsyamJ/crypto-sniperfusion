"""Offline provider capability probe tests; NEVER send network calls."""
import importlib.util
import io
import json
import urllib.error
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("snf_rpc_probe", ROOT / "harness" / "rpc_capability_probe.py")
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class Fake:
    def __init__(self, results):
        self.results = iter(results)
        self.calls = 0

    def call(self, method, params=None):
        self.calls += 1
        v = next(self.results)
        if isinstance(v, Exception):
            raise v
        return v


class RpcProbeTests(unittest.TestCase):
    def test_wrong_cluster_short_circuit(self):
        c = Fake(["not-mainnet"])
        r = probe.probe(c)
        self.assertEqual(r["status"], "WRONG_CLUSTER")
        self.assertEqual(c.calls, 1)
        self.assertEqual(r["coverage_state"], "UNKNOWN")

    def test_successful_block_not_false_complete(self):
        c = Fake([probe.MAINNET_GENESIS, 1000,
                  {"transactions": [{"transaction": {}, "meta": {}}], "blockhash": "abc"}])
        r = probe.probe(c)
        self.assertEqual(r["status"], "FULL_BLOCK_SAMPLE_AVAILABLE")
        self.assertEqual(r["coverage_state"], "UNKNOWN")
        self.assertFalse(r["promotable"])
        self.assertEqual(c.calls, 3)

    def test_skipped_slot_then_block(self):
        c = Fake([probe.MAINNET_GENESIS, 1000, None, {"transactions": []}])
        r = probe.probe(c)
        self.assertEqual(r["status"], "FULL_BLOCK_SAMPLE_AVAILABLE")
        self.assertEqual(r["samples"][0]["kind"], "SKIPPED_OR_NULL")
        self.assertEqual(c.calls, 4)

    def test_version_unsupported_is_structured(self):
        c = Fake([probe.MAINNET_GENESIS, 1000, probe.ProbeError("TX_VERSION_UNSUPPORTED")])
        r = probe.probe(c)
        self.assertEqual(r["status"], "BLOCKED")
        self.assertEqual(r["error_code"], "TX_VERSION_UNSUPPORTED")
        self.assertFalse(r["source_url_or_credentials_recorded"])
        self.assertNotIn("SECRET", json.dumps(r))

    def test_no_http_secrets_or_non_http_schemes(self):
        with self.assertRaisesRegex(probe.ProbeError, "HTTPS"):
            probe.endpoint_from_env({"SNF_MAINNET_RPC_URL": "http://rpc.example/secret"})
        with self.assertRaisesRegex(probe.ProbeError, "INVALID"):
            probe.endpoint_from_env({"SNF_MAINNET_RPC_URL": "https://user:secret@rpc.example/rpc"})
        with self.assertRaisesRegex(probe.ProbeError, "LIVE_TRADING"):
            probe.endpoint_from_env({"SNF_MAINNET_RPC_URL": "https://rpc.example/x",
                                     "LIVE_TRADING_ENABLED": "true"})

    def test_rpc_error_does_not_print_secret_url(self):
        client = probe.RpcClient("https://example.com/?key=DO_NOT_PRINT")
        error = urllib.error.HTTPError(client._url, 429, "429", {}, None)
        with patch("urllib.request.urlopen", side_effect=error):
            with self.assertRaises(probe.ProbeError) as got:
                client.call("getGenesisHash")
            self.assertEqual(got.exception.code, "HTTP_429_RATE_LIMITED")
            self.assertNotIn("DO_NOT_PRINT", str(got.exception))

    def test_method_allowlist(self):
        client = probe.RpcClient("https://example.com")
        with self.assertRaisesRegex(probe.ProbeError, "METHOD_NOT_ALLOWED"):
            client.call("sendTransaction", [])
        self.assertEqual(client.calls, 0)


if __name__ == "__main__":
    unittest.main()
