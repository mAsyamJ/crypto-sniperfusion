"""Offline tests for the H8 research registry (Mission C / PF5): hash-chained
hypothesis/trial ledger, structural 20-trial bound, cross-process one-use
holdout, deterministic splits, multiple-comparison correction."""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import research_registry as rr


def hypothesis(hid="H-0001", **over):
    payload = {
        "hypothesis_id": hid,
        "claim": "organic-flow entries beat S00 after observed fees",
        "causal_mechanism": "breadth of unique buyers predicts continuation",
        "decision_time_evidence": ["unique_buyers", "net_flow_lamports"],
        "abstention_conditions": ["coverage_ok false", "safety != ELIGIBLE"],
        "cost_model": "observed 125bps curve fee + priority + slippage",
        "falsification_conditions": ["net edge <= 0 on dev split"],
        "development_dataset_sha256": "a" * 64,
        "primary_metric": "incremental_net_edge_vs_no_trade",
        "risk_constraints": {"spend_lamports": 1_000_000},
    }
    payload.update(over)
    return payload


class RegistryChain(unittest.TestCase):
    def test_register_and_verify(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "reg.jsonl"
            reg = rr.Registry(path)
            reg.register_hypothesis(hypothesis())
            reg.record_trial({"hypothesis_id": "H-0001", "trial_index": 1,
                              "result": "REJECTED", "dev_metrics": {}})
            reloaded = rr.Registry(path)
            self.assertTrue(reloaded.verify())
            self.assertEqual(len(reloaded.hypotheses()), 1)
            self.assertEqual(len(reloaded.trials()), 1)

    def test_tampering_is_detected(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "reg.jsonl"
            reg = rr.Registry(path)
            reg.register_hypothesis(hypothesis())
            lines = path.read_text().splitlines()
            record = json.loads(lines[0])
            record["payload"]["claim"] = "edited after the fact"
            lines[0] = json.dumps(record, sort_keys=True, separators=(",", ":"))
            path.write_text("\n".join(lines) + "\n")
            with self.assertRaises(rr.RegistryError):
                rr.Registry(path).verify()

    def test_duplicate_hypothesis_and_missing_fields_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            reg = rr.Registry(Path(tmp) / "reg.jsonl")
            reg.register_hypothesis(hypothesis())
            with self.assertRaises(rr.RegistryError):
                reg.register_hypothesis(hypothesis())
            bad = hypothesis("H-0002")
            del bad["falsification_conditions"]
            with self.assertRaises(rr.RegistryError):
                reg.register_hypothesis(bad)

    def test_trial_budget_is_structural_at_20(self):
        with tempfile.TemporaryDirectory() as tmp:
            reg = rr.Registry(Path(tmp) / "reg.jsonl")
            reg.register_hypothesis(hypothesis())
            for index in range(rr.MAX_TRIALS):
                reg.record_trial({"hypothesis_id": "H-0001",
                                  "trial_index": index + 1,
                                  "result": "RUN", "dev_metrics": {}})
            with self.assertRaises(rr.RegistryError) as ctx:
                reg.record_trial({"hypothesis_id": "H-0001",
                                  "trial_index": 21, "result": "RUN",
                                  "dev_metrics": {}})
            self.assertIn("budget", str(ctx.exception))


class HoldoutOneUse(unittest.TestCase):
    def test_second_open_denied_in_same_process_and_audited(self):
        with tempfile.TemporaryDirectory() as tmp:
            guard = rr.HoldoutGuard(tmp)
            guard.seal({"mint": "M", "graduated": True})
            labels = guard.open("first evaluation")
            self.assertEqual(labels["mint"], "M")
            with self.assertRaises(rr.HoldoutError):
                guard.open("second attempt")
            attempts = guard.attempts()
            self.assertEqual(len(attempts), 2)
            self.assertEqual([a["granted"] for a in attempts], [True, False])

    def test_one_use_enforced_across_processes(self):
        # A REAL second process opens the holdout; this process must then be
        # denied — the marker and counter persist on disk, surviving crashes.
        with tempfile.TemporaryDirectory() as tmp:
            guard = rr.HoldoutGuard(tmp)
            guard.seal({"answer": 42})
            script = (
                "import sys, json;"
                f"sys.path.insert(0, {json.dumps(str(ROOT))});"
                "import research_registry as rr;"
                f"g = rr.HoldoutGuard({json.dumps(tmp)});"
                "labels = g.open('subprocess open');"
                "print(json.dumps(labels))"
            )
            out = subprocess.run([sys.executable, "-c", script],
                                 capture_output=True, text=True)
            self.assertEqual(out.returncode, 0, out.stderr)
            self.assertEqual(json.loads(out.stdout)["answer"], 42)
            with self.assertRaises(rr.HoldoutError):
                guard.open("main process after subprocess opened it")
            # And a second subprocess is denied too (crash-survival).
            out2 = subprocess.run([sys.executable, "-c", script],
                                  capture_output=True, text=True)
            self.assertNotEqual(out2.returncode, 0)
            self.assertIn("already opened", out2.stderr)

    def test_unsealed_holdout_refuses_open(self):
        with tempfile.TemporaryDirectory() as tmp:
            guard = rr.HoldoutGuard(tmp)
            with self.assertRaises(rr.HoldoutError):
                guard.open()


class SplitsAndCorrections(unittest.TestCase):
    def test_temporal_split_is_deterministic_and_disjoint(self):
        items = list(range(100))
        a = rr.temporal_split(items, 0.8, "d" * 64)
        b = rr.temporal_split(items, 0.8, "d" * 64)
        self.assertEqual(a, b)
        self.assertEqual(len(a["development"]), 80)
        self.assertEqual(len(a["holdout"]), 20)
        self.assertFalse(set(a["development"]) & set(a["holdout"]))

    def test_creator_cluster_split_keeps_creators_on_one_side(self):
        records = [{"creator": f"c{i % 5}", "v": i} for i in range(50)]
        split = rr.creator_cluster_split(records, 0.6, "d" * 64)
        dev_creators = set(split["dev_creators"])
        for record in split["holdout"]:
            self.assertNotIn(record["creator"], dev_creators)
        self.assertEqual(len(split["development"]) + len(split["holdout"]), 50)

    def test_bonferroni(self):
        self.assertEqual(rr.bonferroni([0.01, 0.05]), [0.02, 0.1])
        self.assertEqual(rr.bonferroni([0.9]), [0.9])

    def test_benjamini_hochberg_matches_reference(self):
        # Classic reference: p=[0.01,0.04,0.03,0.005] -> BH=[0.02,0.04,0.04,0.02]
        adjusted = rr.benjamini_hochberg([0.01, 0.04, 0.03, 0.005])
        self.assertEqual([round(x, 6) for x in adjusted],
                         [0.02, 0.04, 0.04, 0.02])

    def test_registry_refuses_live_env(self):
        with mock.patch.dict("os.environ", {"LIVE_TRADING_ENABLED": "true"}):
            with self.assertRaises(SystemExit):
                rr.require_safe_env()


if __name__ == "__main__":
    unittest.main()
