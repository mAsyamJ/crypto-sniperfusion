"""Read-only cluster-probe negative controls (no network)."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from cluster_probe import GENESIS, PROGRAMS, probe, safe_endpoint


class ClusterProbeTests(unittest.TestCase):
    def test_devnet_program_gate(self):
        def call(endpoint, method, params=None):
            if method == "getGenesisHash":
                return GENESIS["devnet"]
            if method == "getSlot":
                return 1234
            if method == "getAccountInfo":
                return {"value": {"executable": True}}
            raise AssertionError(method)
        result = probe("devnet", "https://api.devnet.solana.com", call=call)
        self.assertTrue(result["pump_trade_pilot_supported"])
        self.assertEqual(result["tx_submitted"], 0)
        self.assertEqual(len(result["programs"]), len(PROGRAMS))

    def test_wrong_genesis_fails_closed_without_program_queries(self):
        methods = []
        def call(_endpoint, method, _params=None):
            methods.append(method)
            return GENESIS["testnet"]
        with self.assertRaisesRegex(ValueError, "WRONG_CLUSTER"):
            probe("devnet", "https://api.devnet.solana.com", call=call)
        self.assertEqual(methods, ["getGenesisHash"])

    def test_testnet_never_claims_pump_trading_supported(self):
        def call(_endpoint, method, _params=None):
            if method == "getGenesisHash":
                return GENESIS["testnet"]
            if method == "getSlot":
                return 1234
            return {"value": {"executable": True}}
        result = probe("testnet", "https://api.testnet.solana.com", call=call)
        self.assertFalse(result["pump_trade_pilot_supported"])

    def test_missing_program_is_not_trading_ready(self):
        def call(_endpoint, method, params=None):
            if method == "getGenesisHash":
                return GENESIS["devnet"]
            if method == "getSlot":
                return 1234
            if params[0] == PROGRAMS["pumpswap"]:
                return {"value": None}
            return {"value": {"executable": True}}
        result = probe("devnet", "https://api.devnet.solana.com", call=call)
        self.assertFalse(result["pump_trade_pilot_supported"])

    def test_url_rejects_embedded_secrets_and_http(self):
        for endpoint in ("http://api.devnet.solana.com", "https://user:pass@example.com",
                         "https://api.devnet.solana.com#private"):
            with self.assertRaises(ValueError):
                safe_endpoint(endpoint)


if __name__ == "__main__":
    unittest.main()
