"""Research harness negative controls. All fixtures are synthetic."""
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from research_batch import (EVIDENCE_CLASS, STRATEGIES, diagnose, load_events,
                            run_batch, simulate)

SAMPLE = ROOT / "fixtures" / "research" / "synthetic_lifecycle.jsonl"


class ResearchHarnessTests(unittest.TestCase):
    def test_batch_has_ten_trade_candidates_plus_no_trade(self):
        report = run_batch(SAMPLE)
        self.assertEqual(report["evidence_class"], EVIDENCE_CLASS)
        self.assertFalse(report["promotable"])
        self.assertEqual(len(STRATEGIES), 11)
        self.assertEqual(len(report["results"]), 11)
        self.assertEqual(report["results"][0]["strategy"], "S00")
        self.assertEqual(report["results"][0]["net_pnl_lamports"], 0)
        self.assertEqual(report["results"][0]["attempted_entries"], 0)
        self.assertEqual(report["event_count"], len(load_events(SAMPLE)))

    def test_repeated_runs_have_identical_economic_results(self):
        a = run_batch(SAMPLE)
        b = run_batch(SAMPLE)
        self.assertEqual(json.dumps(a, sort_keys=True), json.dumps(b, sort_keys=True))

    def test_future_feature_rejected(self):
        records = load_events(SAMPLE)
        records[0]["feature_available_at_ms"] = records[0]["observed_at_ms"] - 1
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "bad.jsonl"
            p.write_text("\n".join(json.dumps(r) for r in records) + "\n")
            with self.assertRaisesRegex(ValueError, "lookahead"):
                load_events(p)

    def test_observation_reorder_rejected_not_sorted(self):
        records = load_events(SAMPLE)
        records[0], records[1] = records[1], records[0]
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "bad.jsonl"
            p.write_text("\n".join(json.dumps(r) for r in records) + "\n")
            with self.assertRaisesRegex(ValueError, "chronology"):
                load_events(p)

    def test_duplicate_event_id_rejected(self):
        records = load_events(SAMPLE)
        records[1]["event_id"] = records[0]["event_id"]
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "bad.jsonl"
            p.write_text("\n".join(json.dumps(r) for r in records) + "\n")
            with self.assertRaisesRegex(ValueError, "duplicate"):
                load_events(p)

    def test_safety_block_and_missing_coverage_never_enter(self):
        events = load_events(SAMPLE)
        for e in events:
            e["safety"] = "BLOCK"
        result = simulate(events, "S01", 100_000_000, 1_000_000,
                          100, 10000, 1_500_000, 20000)
        self.assertEqual(result["attempted_entries"], 0)
        for e in events:
            e["safety"] = "ELIGIBLE"
            e["coverage_ok"] = False
        result = simulate(events, "S01", 100_000_000, 1_000_000,
                          100, 10000, 1_500_000, 20000)
        self.assertEqual(result["attempted_entries"], 0)

    def test_unexitable_open_loss_reflected_in_equity(self):
        events = load_events(SAMPLE)
        for e in events:
            e["mint"] = "SYNTHETIC_ONLY_ONE"
            if e["event_id"] != "SYN-001":
                e["safety"] = "WATCH"
                e["exit_liquidity_lamports"] = 0
            if e["event_id"] == "SYN-001":
                e["exit_liquidity_lamports"] = 10_000_000
        result = simulate(events, "S01", 100_000_000, 1_000_000,
                          100, 10000, 1_500_000, 20000)
        self.assertEqual(result["filled_entries"], 1)
        self.assertEqual(result["open_positions"], 1)
        self.assertLess(result["ending_equity_lamports"], 100_000_000)
        self.assertLess(result["net_pnl_lamports"], 0)

    def test_diagnose_always_refuses_promotion(self):
        report = run_batch(SAMPLE)
        diagnosis = diagnose(report)
        self.assertEqual(diagnosis["status"], "RESEARCH_ONLY_NOT_PROMOTABLE")
        report["promotable"] = True
        with self.assertRaisesRegex(ValueError, "refuses"):
            diagnose(report)


if __name__ == "__main__":
    unittest.main()
