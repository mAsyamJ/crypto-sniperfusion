"""No-broadcast cycle control tests. Synthetic only."""
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from research_batch import load_events, hash_file
from research_cycle import cycle_report, partition, write_once

FIXTURE = ROOT / "fixtures/research/synthetic_lifecycle.jsonl"


class ResearchCycleTests(unittest.TestCase):
    def test_cycle_is_deterministic_and_nonpromotable(self):
        events = load_events(FIXTURE)
        x = cycle_report(events, hash_file(FIXTURE), 3)
        y = cycle_report(events, hash_file(FIXTURE), 3)
        self.assertEqual(x, y)
        self.assertEqual(len(x["cycles"]), 3)
        self.assertEqual(x["total_trials_recorded"], 99)
        self.assertEqual(x["sealed_check_read_count"], 1)
        self.assertFalse(x["promotable"])
        self.assertEqual(x["independent_review"]["decision"], "NOT_DEMONSTRATED")
        self.assertEqual(x["evidence_class"], "SYNTHETIC_SURROGATE_UNCALIBRATED")

    def test_sealed_split_never_reuses_development_mint(self):
        a, b = partition(load_events(FIXTURE))
        self.assertFalse({x["mint"] for x in a} & {x["mint"] for x in b})

    def test_unbounded_and_invalid_budget_rejected(self):
        events = load_events(FIXTURE)
        for count in (0, 21):
            with self.assertRaisesRegex(ValueError, "cycles"):
                cycle_report(events, hash_file(FIXTURE), count)
        with self.assertRaisesRegex(ValueError, "budget"):
            cycle_report(events, hash_file(FIXTURE), 1, cash=1, spend=2)

    def test_write_once_never_overwrites_evidence(self):
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "receipt.json"
            write_once(path, {"status": "demo"})
            self.assertEqual(json.loads(path.read_text())["status"], "demo")
            with self.assertRaises(FileExistsError):
                write_once(path, {"status": "tampered"})
            self.assertEqual(json.loads(path.read_text())["status"], "demo")


if __name__ == "__main__":
    unittest.main()
