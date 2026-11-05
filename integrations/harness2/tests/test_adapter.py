"""Contract tests: the SniperFusion adapter implements the REAL upstream
BenchmarkAdapter protocol and the deterministic recursion works end-to-end.

These tests run under the PROJECT python3 (offline parts) and, when the
harness2 venv exists, additionally drive the real upstream recursion via
`.venv-harness2/bin/python` subprocess. They skip gracefully (with the
reason printed) when the venv is absent — mirroring the CI posture.
"""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "harness"))

VENV_PY = ROOT / ".venv-harness2" / "bin" / "python"


def _h2_env_available():
    return VENV_PY.is_file() and (ROOT / ".external" / "harness2" / ".git").is_dir()


class AdapterContractOffline(unittest.TestCase):
    """Protocol-shape checks that need only the source file (no venv)."""

    def test_adapter_source_implements_the_13_protocol_methods(self):
        source = (ROOT / "integrations" / "harness2" / "adapter.py").read_text()
        required = [
            "def tasks(self", "def domain_of(self", "def task_prompt(self",
            "def task_files(", "def execute(", "def delivered(self",
            "def trajectory(", "def deliverable_files(self",
            "def deliverable_text(self", "def base_harness(self",
            "def validate_tree(self", "def score(self",
            "self.spec = BenchSpec(", "self.substrate = OpencodeSubstrate()",
            "leak_globs=(\"reference_answer.txt\",)",
        ]
        for needle in required:
            self.assertIn(needle, source, f"adapter missing: {needle}")

    def test_fixture_truth_is_oracle_derived(self):
        # wallet-A: buy -1_000_000_000 (fee 5k) + sell +950_000_000 (fee 5k,
        # venue fee already net) => -50_010_000 exactly.
        self.assertEqual(
            -1_000_000_000 + 950_000_000 - 2 * 5_000, -50_010_000
        )
        # wallet-B: real round trip only; transfer and LP flow excluded.
        self.assertEqual(
            -400_000_000 + 300_000_000 - 2 * 5_000, -100_010_000
        )

    def test_validate_tree_rejects_subprocess_harness_scripts(self):
        source = (ROOT / "integrations" / "harness2" / "adapter.py").read_text()
        self.assertIn("must not spawn subprocesses", source)


@unittest.skipUnless(_h2_env_available(), "harness2 venv + pinned checkout absent")
class AdapterRecursionLive(unittest.TestCase):
    """Drive the REAL upstream recursion through our adapter."""

    def test_protocol_methods_on_real_classes(self):
        code = (
            "import sys; sys.path.insert(0, 'integrations/harness2')\n"
            "from pathlib import Path\n"
            "import tempfile\n"
            "from adapter import SniperFusionAdapter\n"
            "required = ['tasks','domain_of','task_prompt','task_files',"
            "'execute','delivered','trajectory','deliverable_files',"
            "'deliverable_text','base_harness','validate_tree','score']\n"
            "with tempfile.TemporaryDirectory() as tmp:\n"
            "    a = SniperFusionAdapter(Path(tmp))\n"
            "    missing = [m for m in required if not callable(getattr(a, m, None))]\n"
            "    assert not missing, missing\n"
            "    assert a.tasks() == ['wallet-A','wallet-B']\n"
            "    assert a.domain_of('wallet-A') == 'wallet-fee-reconciliation'\n"
            "    assert (a.task_files('wallet-A') / 'events.json').is_file()\n"
            "    base = a.base_harness()\n"
            "    assert (base / 'scripts' / 'reconcile.py').is_file()\n"
            "    run = a.execute('wallet-A', None, 'contract-run')\n"
            "    assert a.delivered(run)\n"
            "    assert 'NET_PNL_LAMPORTS' in a.deliverable_text(run)\n"
            "    sc = a.score(run, 'wallet-A')\n"
            "    assert sc is not None and 'expected_lamports' in sc\n"
            "print('CONTRACT_OK')\n"
        )
        out = subprocess.run(
            [str(VENV_PY), "-c", code], cwd=ROOT, capture_output=True, text=True,
            timeout=180,
        )
        self.assertEqual(out.returncode, 0, out.stderr)
        self.assertIn("CONTRACT_OK", out.stdout)

    def test_sequential_recursion_end_to_end(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = subprocess.run(
                [
                    str(VENV_PY), "-c",
                    "import sys; sys.path.insert(0, 'integrations/harness2')\n"
                    "from pathlib import Path\n"
                    "from adapter import run_research\n"
                    f"r = run_research(Path(r'{tmp}').resolve(), k=1, mode='sequential')\n"
                    "assert r.get('failed') == 0, r\n"
                    "assert r.get('done') == 2, r\n"
                    "assert r['execution_count'] > 0 and r['editor_calls'] > 0\n"
                    "print('RECURSION_OK', r['done'], r['failed'])\n",
                ],
                cwd=ROOT, capture_output=True, text=True, timeout=300,
                env={k: v for k, v in os.environ.items()
                     if k in ("PATH", "HOME", "LANG", "LIVE_TRADING_ENABLED")
                     or k == "LIVE_TRADING_ENABLED"},
            )
            self.assertEqual(out.returncode, 0, out.stderr)
            self.assertIn("RECURSION_OK", out.stdout)

    def test_experience_library_persists_transferable_lesson(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = subprocess.run(
                [
                    str(VENV_PY), "-c",
                    "import sys; sys.path.insert(0, 'integrations/harness2')\n"
                    "from pathlib import Path\n"
                    "from adapter import run_research\n"
                    f"run_research(Path(r'{tmp}').resolve(), k=1, mode='sequential')\n"
                    f"lib = sorted(Path(r'{tmp}').glob('library/k1/seq/*/harnesses/*/EXPERIENCE.md'))\n"
                    "assert lib, 'no experience entries'\n"
                    "text = lib[0].read_text()\n"
                    "assert 'Transferable lesson' in text\n"
                    "assert 'venue fee' in text.lower()\n"
                    "print('LIBRARY_OK', len(lib))\n",
                ],
                cwd=ROOT, capture_output=True, text=True, timeout=300,
            )
            self.assertEqual(out.returncode, 0, out.stderr)
            self.assertIn("LIBRARY_OK", out.stdout)

    def test_lock_matches_installed_checkout(self):
        lock = json.loads(
            (ROOT / "integrations" / "harness2" / "upstream.lock.json").read_text()
        )
        current = subprocess.run(
            ["git", "-C", str(ROOT / ".external" / "harness2"), "rev-parse", "HEAD"],
            capture_output=True, text=True,
        ).stdout.strip()
        self.assertEqual(lock["commit"], current)
        self.assertEqual(
            lock["commit"], "6868817518876de8fd170c6a8354daca273b6191"
        )
        self.assertTrue(lock["verification"]["credential_free_demo_exit_0"])


if __name__ == "__main__":
    unittest.main()
