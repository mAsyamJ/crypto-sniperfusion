"""Security + bridge tests for the harness² integration."""

import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "integrations" / "harness2"))
sys.path.insert(0, str(ROOT / "harness"))

import security  # noqa: E402
from evidence_bridge import assert_no_holdout_references, bridge_run  # noqa: E402


class ScrubbedEnvTests(unittest.TestCase):
    def test_scrubs_rpc_model_and_key_material(self):
        base = {
            "PATH": "/usr/bin", "HOME": "/home/x",
            "SNF_MAINNET_RPC_URL": "https://secret",
            "HARNESS2_CODEX_API_KEY": "sk-123",
            "OPENAI_API_KEY": "sk-456",
            "SOLANA_PRIVATE_KEY": "abc",
            "GITHUB_TOKEN": "gh",
            "MY_APP_SECRET": "s",
            "LIVE_TRADING_ENABLED": "true",
        }
        env = security.scrubbed_env(base)
        for banned in ("SNF_MAINNET_RPC_URL", "HARNESS2_CODEX_API_KEY",
                       "OPENAI_API_KEY", "SOLANA_PRIVATE_KEY",
                       "GITHUB_TOKEN", "MY_APP_SECRET"):
            self.assertNotIn(banned, env, f"{banned} leaked")
        # forced safe, never forwarded as-is
        self.assertEqual(env["LIVE_TRADING_ENABLED"], "false")
        self.assertEqual(env["PATH"], "/usr/bin")

    def test_model_backing_blocked_without_worker_marker(self):
        with mock.patch.object(security, "WORKER_ENABLED_MARKER",
                               Path(tempfile.gettempdir()) / "nope-missing"):
            with self.assertRaises(security.BlockedModelError):
                security.assert_model_backing_allowed(True)
            # deterministic mode is always allowed
            security.assert_model_backing_allowed(False)


class PathJailTests(unittest.TestCase):
    def test_escape_refused(self):
        with self.assertRaises(ValueError):
            security.path_jail("/etc/passwd")
        inside = ROOT / "harness" / "runs" / "harness2" / "x" / "y"
        self.assertEqual(security.path_jail(inside), inside)


class BridgeTests(unittest.TestCase):
    def _run_summary(self):
        return {
            "hypothesis_id": "H-9001",
            "claim": "deterministic recursion fixes the reconcile harness",
            "run_dir": "/tmp/run-x",
            "done": 2, "failed": 0,
            "execution_count": 8, "editor_calls": 8,
        }

    def test_bridge_preregisters_then_records_one_trial(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "reg.jsonl"
            out = bridge_run(path, self._run_summary())
            self.assertEqual(out["hypothesis"], "H-9001")
            # Second run of the SAME hypothesis: no duplicate registration,
            # exactly one more trial (rollouts are sub-units).
            out2 = bridge_run(path, self._run_summary())
            self.assertEqual(out2["trial"], 2)  # seq of the 3rd record (0,1,2)
            import research_registry as rr
            reg = rr.Registry(path)
            self.assertEqual(len(reg.hypotheses()), 1)
            self.assertEqual(len(reg.trials()), 2)

    def test_bridge_counts_into_the_20_trial_budget(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "reg.jsonl"
            from research_registry import RegistryError
            with self.assertRaises(RegistryError):
                for i in range(21):
                    bridge_run(path, {**self._run_summary(),
                                      "hypothesis_id": f"H-90{i:02d}"})

    def test_holdout_references_refused_in_candidate_context(self):
        with self.assertRaises(Exception):
            assert_no_holdout_references({"context": "see holdout labels"})
        self.assertTrue(assert_no_holdout_references({"context": "clean"}))


if __name__ == "__main__":
    unittest.main()
