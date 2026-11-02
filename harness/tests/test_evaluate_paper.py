import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
from evaluate_paper import evaluate, read_trades

SAMPLE = ROOT / "fixtures" / "example_paper_trades.jsonl"

class TestAccounting(unittest.TestCase):
    def test_synthetic_sample_costs_and_pnl(self):
        result = evaluate(read_trades(SAMPLE))
        self.assertEqual(result["trades"], 3)
        self.assertEqual(result["net_pnl_lamports"], -1800000)
        self.assertEqual(result["network_plus_tip_lamports"], 800000)
        self.assertEqual(result["win_rate"], round(2 / 3, 6))

    def test_lookahead_rejected(self):
        data = json.loads(SAMPLE.read_text().splitlines()[0])
        data["features_observed_at_ms"] = 999999
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "bad.jsonl"
            path.write_text(json.dumps(data) + "\n")
            with self.assertRaisesRegex(ValueError, "lookahead"):
                read_trades(path)

    def test_duplicate_trade_rejected(self):
        line = SAMPLE.read_text().splitlines()[0]
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "duplicates.jsonl"
            path.write_text(line + "\n" + line + "\n")
            with self.assertRaisesRegex(ValueError, "duplicate"):
                read_trades(path)

if __name__ == "__main__":
    unittest.main()
