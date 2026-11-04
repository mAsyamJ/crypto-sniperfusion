"""Offline wallet-alpha evaluation tests: all fixtures synthetic, no RPC."""
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("wallet_alpha_lab", ROOT / "harness" / "wallet_alpha_lab.py")
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)
FIXTURE = ROOT / "harness" / "fixtures" / "wallet_alpha_synthetic.jsonl"

class WalletAlphaLabTests(unittest.TestCase):
    def test_net_after_costs_and_no_trade_control(self):
        r = lab.metrics(lab.load_records(FIXTURE), min_trades=2)
        self.assertFalse(r["promotable"])
        self.assertFalse(r["portfolio_nav_reconciled"])
        self.assertIsNone(r["realized_mainnet_pnl"])
        self.assertEqual(r["excluded_incomplete_rows"]["INCOMPLETE"], 1)
        alpha = next(x for x in r["wallets"] if x["wallet"] == "SYNTHETIC_WALLET_ALPHA")
        self.assertEqual(alpha["complete_roundtrips"], 2)
        self.assertEqual(alpha["follower_net_pnl_lamports"], -11000)
        self.assertEqual(alpha["net_edge_vs_no_trade_lamports"], -11000)
        self.assertEqual(alpha["verdict"], "NEGATIVE_NET")

    def test_tiny_sample_not_called_profitable(self):
        r = lab.metrics(lab.load_records(FIXTURE))
        self.assertTrue(all(x["verdict"] == "INSUFFICIENT_SAMPLE" for x in r["wallets"]))

    def test_future_signal_or_ranking_rejected(self):
        item = lab.load_records(FIXTURE)[0]
        item["ranking_asof_ms"] = item["leader_buy_ms"] + 1
        with self.assertRaisesRegex(ValueError, "lookahead"):
            lab.validate_row(item, 1)

    def test_future_quote_used_before_signal_rejected(self):
        item = lab.load_records(FIXTURE)[0]
        item["follower_entry_quote_at_ms"] = item["signal_observed_ms"] - 1
        with self.assertRaisesRegex(ValueError, "lookahead"):
            lab.validate_row(item, 1)

    def test_transfer_not_a_trade_and_wrong_proof_rejected(self):
        item = lab.load_records(FIXTURE)[0]
        item["proof_state"] = "MAINNET_EXECUTED"
        with self.assertRaisesRegex(ValueError, "proof_state"):
            lab.validate_row(item, 1)

    def test_duplicate_trade_rejected(self):
        row = FIXTURE.read_text(encoding="utf-8").splitlines()[0]
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / "dupe.jsonl"
            p.write_text(row+"\n"+row+"\n")
            with self.assertRaisesRegex(ValueError, "duplicate"):
                lab.load_records(p)

    def test_boolean_amount_and_negative_amount_rejected(self):
        item = lab.load_records(FIXTURE)[0]
        item["tip_lamports"] = True
        with self.assertRaisesRegex(ValueError, "tip_lamports"):
            lab.validate_row(item, 1)
        item["tip_lamports"] = -1
        with self.assertRaisesRegex(ValueError, "tip_lamports"):
            lab.validate_row(item, 1)

    def test_stable_replay_hash(self):
        a = lab.metrics(lab.load_records(FIXTURE))
        b = lab.metrics(lab.load_records(FIXTURE))
        self.assertEqual(a, b)

    def test_no_unverified_rows_count_as_profit(self):
        rows = lab.load_records(FIXTURE)
        last = rows[-1]
        self.assertEqual(last["coverage"], "INCOMPLETE")
        result = lab.metrics(rows)
        self.assertNotIn(last["wallet"], [w["wallet"] for w in result["wallets"]])

if __name__ == "__main__":
    unittest.main()
