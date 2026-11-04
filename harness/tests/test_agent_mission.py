"""H5-H8 agent mission negative controls. No network, signer or fork."""
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from agent_mission import execute_research, plan, python_gate, write_once
from research_batch import EVIDENCE_CLASS

FIXTURE = ROOT / "fixtures/research/synthetic_lifecycle.jsonl"


class AgentMissionTests(unittest.TestCase):
    def test_plan_explicitly_cannot_claim_live_or_profit(self):
        p = plan()
        self.assertEqual(p["safety"], "OFFLINE_RESEARCH_ONLY")
        self.assertEqual(len(p["phases"]), 4)
        self.assertEqual(p["gates"]["G0_full_launch_dataset"], "BLOCKED_NO_COMPLETE_MAINNET_COHORT")
        self.assertIn("RUST_STRATEGY_SDK", p["phases"][2]["current"])
        self.assertNotEqual(p["phases"][1]["current"], "SAMPLE_COLLECTOR_ONLY_NO_COMPLETENESS_PROOF")
        self.assertEqual(p["gates"]["G7_paper_only_promotion"], "DISABLED")
        self.assertTrue(any("mainnet" in a for a in p["prohibited_actions"]))

    def test_two_cycles_produce_evidence_with_gates_not_alpha(self):
        gate = lambda: {"status": "PASS", "exit_code": 0}
        report = execute_research(FIXTURE, 2, gate)
        self.assertEqual(report["status"], "SYNTHETIC_RESEARCH_COMPLETE")
        self.assertEqual(report["evidence_class"], EVIDENCE_CLASS)
        self.assertFalse(report["promotable"])
        self.assertFalse(report["mainnet_execution"])
        self.assertFalse(report["devnet_execution"])
        self.assertEqual(report["signatures"], [])
        self.assertEqual(len(report["experiment"]["cycles"]), 2)
        self.assertEqual(report["experiment"]["total_trials_recorded"], 66)
        self.assertEqual(report["gates"]["G5_mainnet_read_only_shadow"], "NOT_STARTED")
        self.assertEqual(report["gates"]["G3_real_temporal_cluster_oos_alpha"], "NOT_DEMONSTRATED")
        self.assertIn("receipt_sha256", report)

    def test_repeatability_under_identical_inputs(self):
        gate = lambda: {"status": "PASS", "exit_code": 0}
        self.assertEqual(
            execute_research(FIXTURE, 1, gate),
            execute_research(FIXTURE, 1, gate),
        )

    def test_failed_preflight_never_runs_experiment(self):
        gate = lambda: {"status": "FAIL", "exit_code": 1}
        r = execute_research(FIXTURE, 2, gate)
        self.assertEqual(r["status"], "BLOCKED")
        self.assertIsNone(r["experiment"])
        self.assertEqual(r["gates"]["G1_python_gate"], "FAIL")

    def test_rejects_unbounded_cycles(self):
        for cycles in (0, 21, -1, "5", True):
            with self.assertRaisesRegex(ValueError, "cycles"):
                execute_research(FIXTURE, cycles, lambda: {"status": "PASS"})

    def test_rejects_live_environment(self):
        with patch.dict(os.environ, {"LIVE_TRADING_ENABLED": "true"}):
            with self.assertRaisesRegex(ValueError, "offline research"):
                execute_research(FIXTURE, 1, lambda: {"status": "PASS"})

    def test_child_test_runner_does_not_inherit_agent_keys(self):
        fake = type("Completed", (), {"returncode": 0, "stderr": ""})()
        with patch.dict(os.environ, {
            "SNF_DEVNET_RPC": "https://secret.example/rpc",
            "OPENAI_API_KEY": "private-key-placeholder",
            "PRIVATE_KEY": "private-key-placeholder",
        }):
            with patch("agent_mission.subprocess.run", return_value=fake) as proc:
                receipt = python_gate()
                kwargs = proc.call_args.kwargs
        self.assertEqual(receipt["status"], "PASS")
        self.assertEqual(kwargs["env"]["LIVE_TRADING_ENABLED"], "false")
        self.assertNotIn("SNF_DEVNET_RPC", kwargs["env"])
        self.assertNotIn("OPENAI_API_KEY", kwargs["env"])
        self.assertNotIn("PRIVATE_KEY", kwargs["env"])
        self.assertEqual(kwargs["timeout"], 120)

    def test_unknown_gate_result_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "invalid status"):
            execute_research(FIXTURE, 1, lambda: {"status": "SKIPPED"})

    def test_receipts_are_write_once_and_unchanged(self):
        with tempfile.TemporaryDirectory() as root:
            target = Path(root) / "receipt.json"
            write_once(target, {"evidence": "SYNTHETIC", "promotable": False})
            with self.assertRaises(FileExistsError):
                write_once(target, {"promotable": True})
            self.assertFalse(json.loads(target.read_text())["promotable"])


if __name__ == "__main__":
    unittest.main()
