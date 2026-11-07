"""Guard the single canonical SniperFusion agent mission (documentation-only invariant).

Run: python3 -m unittest harness.tests.test_single_mission_authority
No credentials, network, or trading execution required.
"""
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class SingleMissionAuthority(unittest.TestCase):
    def test_only_one_project_wide_goal_plan_and_launcher_at_root(self):
        self.assertTrue((ROOT / "GOAL_PLAN_PROMPT.md").is_file())
        self.assertTrue((ROOT / "SESSION_LAUNCHER_PROMPT.md").is_file())
        self.assertEqual(
            sorted(p.name for p in ROOT.glob("*GOAL_PLAN_PROMPT.md")),
            ["GOAL_PLAN_PROMPT.md"],
        )
        self.assertEqual(
            sorted(p.name for p in ROOT.glob("*SESSION_LAUNCHER*")),
            ["SESSION_LAUNCHER_PROMPT.md"],
        )
        self.assertFalse((ROOT / "DASHBOARD_EXECUTE_PROMPT.md").exists())

    def test_historical_dashboard_workflows_are_preserved(self):
        for name in (
            "SNIPERFUSION_DASHBOARD_ARCHITECTURE_PLAN.md",
            "SNIPERFUSION_DASHBOARD_GOAL_PLAN_PROMPT.md",
            "SNIPERFUSION_DASHBOARD_SESSION_LAUNCHER.md",
            "DASHBOARD_EXECUTE_PROMPT.md",
        ):
            path = ROOT / "docs" / "history" / "dashboard" / name
            self.assertTrue(path.is_file(), str(path))
            self.assertIn("HISTORICAL / NOT AN ACTIVE SESSION", path.read_text())

    def test_current_agent_router_never_points_to_old_g0(self):
        agentctl = (ROOT / "harness" / "agentctl.py").read_text()
        self.assertIn('"next": "docs/GOALS.md"', agentctl)
        self.assertIn('"current_session_launcher": "cat SESSION_LAUNCHER_PROMPT.md"', agentctl)
        self.assertNotIn('"next_goal": "cat docs/NEXT_GOAL_H8_G0.md"', agentctl)
        for path in ("GOAL_PLAN_PROMPT.md", "SESSION_LAUNCHER_PROMPT.md", "AGENTS.md"):
            self.assertIn("docs/NEXT_GOAL_24H_FILTERED_PAPER_RESEARCH.md", (ROOT / path).read_text())

    def test_readme_and_index_authority(self):
        readme = (ROOT / "README.md").read_text()
        index = (ROOT / "docs" / "DOC_INDEX.md").read_text()
        self.assertIn("One active agent entrypoint", readme)
        self.assertIn("Sole active mission and no-shadow rule", index)
        self.assertNotIn("| `SNIPERFUSION_DASHBOARD_GOAL_PLAN_PROMPT.md` |", index)


if __name__ == "__main__":
    unittest.main()
