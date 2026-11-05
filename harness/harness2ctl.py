#!/usr/bin/env python3
"""SniperFusion harness2ctl — first-class operator surface for the REAL
Google Harness² integration (ADR-0012).

Subcommands: doctor | demo | research | status | report.
All model-backed substrates are BLOCKED until the isolated worker
(integrations/harness2/worker/) is operator-enabled; only the
credential-free deterministic channel runs. Offline; no keys; outputs
write-once 0444 under harness/runs/harness2/.
"""

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "harness"))

from launch_cohort_capture import write_once  # noqa: E402

VENV_PY = ROOT / ".venv-harness2" / "bin" / "python"
EXT = ROOT / ".external" / "harness2"
RUNS = ROOT / "harness" / "runs" / "harness2"
LOCK = ROOT / "integrations" / "harness2" / "upstream.lock.json"

SAFE_CHILD_ENV_KEYS = ("PATH", "HOME", "LANG", "LC_ALL", "TZ", "SYSTEMROOT")


def safe_env():
    env = {k: os.environ.get(k) for k in SAFE_CHILD_ENV_KEYS if os.environ.get(k)}
    env["LIVE_TRADING_ENABLED"] = "false"
    # Never forward RPC/keys/model credentials to any child.
    for key in list(os.environ):
        if key.startswith(("SNF_", "HARNESS2_", "OPENAI", "ANTHROPIC", "GITHUB_")) \
                or "KEY" in key or "TOKEN" in key or "SECRET" in key:
            env.pop(key, None)
    return env


def require_safe_host_env():
    if os.environ.get("LIVE_TRADING_ENABLED", "false").lower() not in ("", "false", "0"):
        raise SystemExit("harness2ctl refuses LIVE_TRADING_ENABLED")
    for key in os.environ:
        if key.startswith(("HARNESS2_CODEX_", "SNF_MAINNET_RPC_URL")) and os.environ[key]:
            # Refuse to even run while operator secrets are exported.
            raise SystemExit(f"refusing to run with {key.split('_')[0]}* credentials in env")


def installed():
    return VENV_PY.is_file() and (EXT / ".git").is_dir()


def cmd_doctor(_args):
    report = {
        "tool": "harness2ctl",
        "purpose": "REAL Google Harness2 integration (ADR-0012)",
        "installed": installed(),
        "model_backed": "BLOCKED_NO_ISOLATED_WORKER",
        "deterministic_channel": "AVAILABLE" if installed() else "NOT_INSTALLED",
    }
    if installed():
        lock = json.loads(LOCK.read_text())
        current = subprocess.run(
            ["git", "-C", str(EXT), "rev-parse", "HEAD"],
            capture_output=True, text=True, env=safe_env(),
        ).stdout.strip()
        report["lock_commit"] = lock["commit"]
        report["checkout_commit"] = current
        report["pin_ok"] = current == lock["commit"]
        imp = subprocess.run(
            [str(VENV_PY), "-c",
             "import harness2, os; print(os.path.dirname(harness2.__file__))"],
            capture_output=True, text=True, env=safe_env(), timeout=60,
        )
        report["import_resolves_to_checkout"] = imp.returncode == 0 and imp.stdout.strip() == str(EXT / "harness2")
        report["license"] = "Apache-2.0 (Google LLC)"
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report.get("installed") else 2


def cmd_demo(args):
    require_safe_host_env()
    if not installed():
        print(json.dumps({"error": "not installed; run scripts/install-harness2.sh"}))
        return 2
    out = RUNS / f"demo-{time.strftime('%Y%m%dT%H%M%S')}-{os.getpid()}"
    proc = subprocess.run(
        [str(VENV_PY), "-m", "harness2.cli.demo", "--output", str(out), "--k", str(args.k)],
        cwd=ROOT, capture_output=True, text=True, env=safe_env(), timeout=300,
    )
    print(proc.stdout.strip())
    return proc.returncode


def cmd_research(args):
    require_safe_host_env()
    if not installed():
        print(json.dumps({"error": "not installed; run scripts/install-harness2.sh"}))
        return 2
    if args.mode not in ("sequential", "parallel"):
        raise SystemExit("--mode must be sequential|parallel")
    out = RUNS / f"research-{time.strftime('%Y%m%dT%H%M%S')}-{os.getpid()}"
    code = (
        "import sys; sys.path.insert(0, 'integrations/harness2')\n"
        "from pathlib import Path\n"
        "from adapter import run_research\n"
        f"r = run_research(Path(r'{out}').resolve(), k={args.k}, mode='{args.mode}')\n"
        "import json; print(json.dumps({'done': r.get('done'), 'failed': r.get('failed'),\n"
        "  'execution_count': r['execution_count'], 'editor_calls': r['editor_calls'],\n"
        f"  'result_json': r'{out}' + '/result.json'}}))\n"
    )
    proc = subprocess.run(
        [str(VENV_PY), "-c", code], cwd=ROOT, capture_output=True, text=True,
        env=safe_env(), timeout=600,
    )
    if proc.returncode != 0:
        print(proc.stderr.strip() or "research failed", file=sys.stderr)
        return proc.returncode
    summary = json.loads(proc.stdout.strip().splitlines()[-1])
    summary.update({
        "promotable": False,
        "evidence_class": "DETERMINISTIC_RESEARCH_HARNESS",
        "model_backed": False,
        "task": args.task,
        "mode": args.mode,
        "k": args.k,
    })
    write_once(out / "summary.json", json.dumps(summary, indent=2, sort_keys=True) + "\n")
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


def cmd_status(_args):
    rows = []
    if RUNS.is_dir():
        for run in sorted(RUNS.iterdir()):
            result = run / "result.json"
            summary = run / "summary.json"
            if summary.is_file():
                rows.append({"run": run.name, **json.loads(summary.read_text())})
            elif result.is_file():
                rows.append({"run": run.name, "raw": True})
    print(json.dumps({"runs": rows, "model_backed": "BLOCKED_NO_ISOLATED_WORKER"},
                     indent=2, sort_keys=True))
    return 0


def cmd_report(args):
    run = RUNS / args.run
    result = run / "result.json"
    if not result.is_file():
        print(json.dumps({"error": f"no result.json under {run}"}))
        return 2
    data = json.loads(result.read_text())
    experiences = sorted(run.glob("library/**/harnesses/*/EXPERIENCE.md"))
    data["experience_entries"] = [str(p.relative_to(run)) for p in experiences]
    print(json.dumps(data, indent=2, sort_keys=True))
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("doctor").set_defaults(fn=cmd_doctor)
    demo = sub.add_parser("demo")
    demo.add_argument("--k", type=int, default=1)
    demo.set_defaults(fn=cmd_demo)
    research = sub.add_parser("research")
    research.add_argument("--task", default="wallet-fee-reconciliation")
    research.add_argument("--mode", default="sequential",
                          choices=("sequential", "parallel"))
    research.add_argument("--k", type=int, default=1)
    research.set_defaults(fn=cmd_research)
    sub.add_parser("status").set_defaults(fn=cmd_status)
    report = sub.add_parser("report")
    report.add_argument("--run", required=True)
    report.set_defaults(fn=cmd_report)
    args = parser.parse_args(argv)
    require_safe_host_env()
    return args.fn(args)


if __name__ == "__main__":
    sys.exit(main())
