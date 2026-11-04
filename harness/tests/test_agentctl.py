"""Regression tests for offline SniperFusion agent skill dispatch."""
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("snf_agentctl", ROOT / "harness" / "agentctl.py")
agentctl = importlib.util.module_from_spec(spec)
spec.loader.exec_module(agentctl)


class AgentDispatchTests(unittest.TestCase):
    def test_all_skills_validate_and_include_required_roles(self):
        skills = agentctl.list_skills()
        names = {s["name"] for s in skills}
        self.assertGreaterEqual(len(skills), 16)
        self.assertTrue({"sniperfusion-maintainer", "memecoin-trader-research",
                         "solana-rust-engineer", "cohort-coverage-engineer",
                         "h8-research-council"}.issubset(names))

    def test_rejects_missing_frontmatter(self):
        with tempfile.TemporaryDirectory() as td:
            p = Path(td) / "bad" / "SKILL.md"
            p.parent.mkdir()
            p.write_text("# Invalid\n")
            with self.assertRaisesRegex(ValueError, "frontmatter"):
                agentctl.parse_skill(p)

    def test_rejects_mismatched_name(self):
        with tempfile.TemporaryDirectory() as td:
            p = Path(td) / "wrong" / "SKILL.md"
            p.parent.mkdir()
            p.write_text("---\nname: else\ndescription: This is a test description of considerable length.\n---\n# Body\n")
            with self.assertRaisesRegex(ValueError, "name/frontmatter"):
                agentctl.parse_skill(p)

    def test_never_forwards_rpc_token_or_private_key(self):
        env = agentctl.safe_child_env({
            "PATH": "/bin",
            "SNF_MAINNET_RPC_URL": "https://rpc/?token=SECRET",
            "PRIVATE_KEY": "SECRET",
            "INFURA_API_KEY": "SECRET",
            "LIVE_TRADING_ENABLED": "true",
        })
        self.assertEqual(env, {"PATH": "/bin", "LIVE_TRADING_ENABLED": "false"})
        self.assertNotIn("SECRET", json.dumps(env))

    def test_rejects_unbounded_cycles_and_external_receipt_path(self):
        with self.assertRaisesRegex(ValueError, "between 1 and 20"):
            agentctl.research_demo(21, "harness/runs/no.json")
        with self.assertRaisesRegex(ValueError, "under harness/runs"):
            agentctl.research_demo(1, "/tmp/unsafe-report.json")

    def test_rejects_output_overwrite(self):
        with tempfile.TemporaryDirectory() as td:
            # Patch RUNS to a temporary directory inside demo's allowed boundary.
            with patch.object(agentctl, "RUNS", Path(td)):
                p = Path(td) / "already.json"
                p.write_text("{}")
                with self.assertRaisesRegex(ValueError, "already exists"):
                    agentctl.research_demo(1, str(p))


if __name__ == "__main__":
    unittest.main()
