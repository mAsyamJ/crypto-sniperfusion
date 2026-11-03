"""Read-only sample capture with explicit incomplete coverage."""
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from observation_capture import GENESIS, PUMP, capture, write_once


class CaptureTests(unittest.TestCase):
    def test_raw_sample_marks_missing_and_incomplete(self):
        calls = []
        def fake(_endpoint, method, params=None):
            calls.append(method)
            if method == "getGenesisHash":
                return GENESIS["mainnet"]
            if method == "getSignaturesForAddress":
                self.assertEqual(params[0], PUMP)
                return [
                    {"signature": "SIG-A", "slot": 1234,
                     "blockTime": 1700000000, "confirmationStatus": "confirmed",
                     "err": None},
                    {"signature": "SIG-B", "slot": 1235,
                     "blockTime": 1700000001, "confirmationStatus": "confirmed",
                     "err": None},
                ]
            if method == "getTransaction":
                return {"meta": {"logMessages": ["Program log: Create"]}} if params[0] == "SIG-A" else None
            raise AssertionError(method)
        report = capture("mainnet", "https://api.mainnet-beta.solana.com",
                         limit=2, call=fake)
        self.assertEqual(report["observed_signatures"], 2)
        self.assertEqual(report["missing_transactions"], 1)
        self.assertFalse(report["all_launches_captured"])
        self.assertFalse(report["transaction_index_reconstructed"])
        self.assertEqual(report["submitted_transactions"], 0)
        self.assertEqual(calls.count("getTransaction"), 2)
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "raw.json"
            write_once(p, report)
            self.assertEqual(json.loads(p.read_text())["missing_transactions"], 1)
            with self.assertRaises(FileExistsError):
                write_once(p, {"tamper": True})

    def test_source_genesis_mismatch_rejected(self):
        def fake(_endpoint, method, _params=None):
            self.assertEqual(method, "getGenesisHash")
            return GENESIS["devnet"]
        with self.assertRaisesRegex(ValueError, "WRONG_CLUSTER"):
            capture("mainnet", "https://api.mainnet-beta.solana.com", call=fake)

    def test_limits_and_https(self):
        for limit in (0, 101):
            with self.assertRaisesRegex(ValueError, "limit"):
                capture("mainnet", "https://api.mainnet-beta.solana.com", limit=limit)
        with self.assertRaisesRegex(ValueError, "HTTPS"):
            capture("mainnet", "http://api.mainnet-beta.solana.com")


if __name__ == "__main__":
    unittest.main()
