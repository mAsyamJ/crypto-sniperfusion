"""Offline wallet-alpha evaluation tests: all fixtures synthetic, no RPC."""
import importlib.util
import json
import tempfile
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
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


class WalletRosterTests(unittest.TestCase):
    def test_discovery_excludes_infrastructure_and_failed(self):
        import wallet_roster as wr
        import json as _json
        from pathlib import Path as _P
        pump = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
        w = "Wa11etWa11etWa11etWa11etWa11etWa11etWa11etWa"
        tx_ok = {
            "transaction": {"signatures": ["S1"], "message": {"accountKeys": [w, pump]}},
            "meta": {"err": None,
                     "preTokenBalances": [], "postTokenBalances": [
                         {"owner": w, "mint": "M", "accountIndex": 2,
                          "uiTokenAmount": {"amount": "5", "decimals": 6}}]},
        }
        tx_failed = {
            "transaction": {"signatures": ["S2"], "message": {"accountKeys": [pump, pump]}},
            "meta": {"err": {"InstructionError": [0, "Custom(1)"]}, "preTokenBalances": [], "postTokenBalances": []},
        }
        wallets = wr.discover_wallets([tx_ok, tx_failed])
        self.assertIn(w, wallets)
        self.assertNotIn(pump, wallets, "infrastructure never ranks as a wallet")

    def test_ranking_tiers_and_uncertainty(self):
        import wallet_roster as wr
        receipts = {
            "profitableObserved": {"net_pnl_lamports": 1_000, "records": 5, "skipped": 0,
                                   "incomplete": 0, "unobserved_opening_inventory_events": 0,
                                   "unobserved_deposits_lamports": 0},
            "loserObserved": {"net_pnl_lamports": -4_961_316_798, "records": 3, "skipped": 0,
                              "incomplete": 0, "unobserved_opening_inventory_events": 0,
                              "unobserved_deposits_lamports": 4_961_316_798},
            "unknownHistory": {"net_pnl_lamports": 0, "records": 3, "skipped": 0,
                               "incomplete": 0, "unobserved_opening_inventory_events": 3,
                               "unobserved_deposits_lamports": 0},
            "noTrades": {"net_pnl_lamports": 0, "records": 0, "skipped": 2,
                         "incomplete": 0, "unobserved_opening_inventory_events": 0,
                         "unobserved_deposits_lamports": 0},
        }
        rows = wr.rank(receipts)
        tiers = {r["wallet"]: r["tier"] for r in rows}
        self.assertEqual(tiers["profitableObserved"], "CANDIDATE")
        self.assertEqual(tiers["loserObserved"], "REJECTED_LOSS")
        self.assertEqual(tiers["unknownHistory"], "LOW_CONFIDENCE")
        self.assertEqual(tiers["noTrades"], "UNRANKABLE")
        self.assertEqual(rows[0]["wallet"], "profitableObserved", "net P&L dominates rank")
        for row in rows:
            self.assertIn("uncertainty", row, "uncertainty column mandatory")
