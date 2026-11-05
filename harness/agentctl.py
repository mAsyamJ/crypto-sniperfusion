#!/usr/bin/env python3
"""Safe SniperFusion agent dispatch: introspection and bounded synthetic demo.

This is NOT a runtime/LLM orchestrator, remote collector, credential manager,
testnet pilot, fund signer, or replacement for harness/snf-harness.
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SKILLS = ROOT / ".agents" / "skills"
SYNTHETIC = ROOT / "harness" / "fixtures" / "research" / "synthetic_lifecycle.jsonl"
RUNS = ROOT / "harness" / "runs"
MAX_CYCLES = 20
SAFE_CHILD_ENV = ("PATH", "HOME", "LANG", "LC_ALL", "TZ", "SYSTEMROOT", "WINDIR")


def safe_child_env(source=None):
    """Whitelisted environment; never forwards RPC tokens, private keys or API URLs."""
    source = os.environ if source is None else source
    env = {key: source[key] for key in SAFE_CHILD_ENV if key in source}
    env["LIVE_TRADING_ENABLED"] = "false"
    return env


def parse_skill(path):
    text = path.read_text(encoding="utf-8")
    match = re.match(r"\A---\r?\n(.*?)\r?\n---(?:\r?\n|\Z)", text, re.S)
    if match is None:
        raise ValueError(f"missing frontmatter: {path.parent.name}")
    meta = {}
    for line in match.group(1).splitlines():
        if ":" in line:
            k, v = line.split(":", 1)
            meta[k.strip()] = v.strip().strip('"').strip("'")
    name = meta.get("name", "")
    desc = meta.get("description", "")
    if name != path.parent.name or not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", name):
        raise ValueError(f"invalid skill name/frontmatter: {path.parent.name}")
    if len(desc) < 25:
        raise ValueError(f"missing/short description: {name}")
    if not text[match.end():].strip():
        raise ValueError(f"empty skill body: {name}")
    return {"name": name, "description": desc, "path": str(path.relative_to(ROOT))}


def list_skills():
    paths = sorted(SKILLS.glob("*/SKILL.md"))
    if not paths:
        raise ValueError("no repository-local skills discovered")
    return [parse_skill(p) for p in paths]


def git_head():
    p = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT,
        capture_output=True, text=True, check=False, timeout=5,
        env=safe_child_env(),
    )
    return p.stdout.strip() if p.returncode == 0 else "UNKNOWN"


def doctor():
    required = ["Cargo.toml", "README.md", "AGENTS.md",
                "docs/PROJECT_STATUS.md", "docs/evidence/H6.md",
                "docs/evidence/H7.md", "harness/launch_cohort_capture.py",
                "harness/agent_mission.py", "harness/snf-harness"]
    missing = [p for p in required if not (ROOT / p).is_file()]
    skills = list_skills()
    report = {
        "tool": "sniperfusion-agentctl",
        "purpose": "OFFLINE_AGENT_DISPATCH_ONLY",
        "git_sha": git_head(),
        "required_files_present": not missing,
        "missing": missing,
        "skills_discovered": len(skills),
        "toolchain_found": {k: bool(shutil.which(k)) for k in ("git", "python3", "cargo", "rustc")},
        "rpc_env_configured": bool(os.environ.get("SNF_MAINNET_RPC_URL")),
        "rpc_url_value_in_output": False,
        "live_trading_enabled": False,
        "mainnet_broadcast_possible_from_this_tool": False,
        "real_cohort_coverage": "NOT_VERIFIED_BY_DOCTOR",
        "alpha": "NOT_DEMONSTRATED",
        "next": "docs/NEXT_GOAL_H8_G0.md",
    }
    print(json.dumps(report, indent=2))
    return 0 if not missing else 2


def commands():
    rows = {
        "doctor": "python3 harness/agentctl.py doctor",
        "skills": "python3 harness/agentctl.py skills",
        "skills_lint": "python3 harness/agentctl.py lint-skills",
        "safe_tests": "python3 -m unittest discover -s harness/tests -v",
        "rust_fmt": "CARGO_BUILD_JOBS=2 cargo fmt --all -- --check",
        "rust_test": "CARGO_BUILD_JOBS=2 cargo test --workspace --locked",
        "mission_plan_STALE": "python3 harness/agent_mission.py plan",
        "synthetic_demo": "python3 harness/agentctl.py research-demo --cycles 3 --out harness/runs/agent-demo-unique.json",
        "existing_harness_help": "python3 harness/snf-harness --help",
        "collector_help_READ_ONLY": "python3 harness/launch_cohort_capture.py --help",
        "rpc_capability_READ_ONLY": "python3 harness/rpc_capability_probe.py --out harness/runs/rpc-sample-unique.json",
        "next_goal": "cat docs/NEXT_GOAL_H8_G0.md",
    }
    print(json.dumps(rows, indent=2))
    return 0


def research_demo(cycles, outfile):
    if not 1 <= cycles <= MAX_CYCLES:
        raise ValueError("cycles must be between 1 and 20")
    target = Path(outfile)
    if not target.is_absolute():
        target = ROOT / target
    target = target.resolve()
    if not target.is_relative_to(RUNS.resolve()):
        raise ValueError("demo output must be under harness/runs")
    if target.exists():
        raise ValueError("write-once demo receipt already exists; choose another --out")
    if not SYNTHETIC.is_file():
        raise ValueError("synthetic fixture not present")
    cmd = [sys.executable, str(ROOT / "harness" / "agent_mission.py"),
           "run", "--events", str(SYNTHETIC), "--cycles", str(cycles), "--out", str(target)]
    return subprocess.run(cmd, cwd=ROOT, env=safe_child_env(), check=False).returncode


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("doctor", help="Inspect local files/toolchain only; no RPC and no secrets")
    sub.add_parser("skills", help="List local expert skills with their descriptions")
    sub.add_parser("lint-skills", help="Validate local SKILL.md metadata and bodies")
    sub.add_parser("commands", help="Print verified command map (no commands executed)")
    demo = sub.add_parser("research-demo", help="Bounded offline synthetic demonstration")
    demo.add_argument("--cycles", type=int, default=3)
    demo.add_argument("--out", required=True, help="New receipt under harness/runs/")
    h2 = sub.add_parser("harness2", help="Google Harness2 integration (doctor|demo|research|status|report)")
    h2.add_argument("harness2_command", help="harness2ctl subcommand")
    h2.add_argument("harness2_args", nargs=argparse.REMAINDER, help="forwarded args")
    args = parser.parse_args(argv)
    try:
        if args.command == "harness2":
            # Passthrough to harness/harness2ctl.py with the SAME sanitized
            # child env used elsewhere (never forwards RPC/keys).
            sub_argv = [args.harness2_command] + args.harness2_args
            result = subprocess.run(
                [sys.executable, str(ROOT / "harness" / "harness2ctl.py")] + sub_argv,
                cwd=ROOT, env=safe_child_env(), timeout=900,
            )
            return result.returncode
        if args.command == "doctor":
            return doctor()
        if args.command == "commands":
            return commands()
        if args.command == "skills":
            print(json.dumps(list_skills(), indent=2))
            return 0
        if args.command == "lint-skills":
            found = list_skills()
            print(json.dumps({"validated": len(found), "status": "PASS"}, indent=2))
            return 0
        if args.command == "research-demo":
            return research_demo(args.cycles, args.out)
    except (OSError, ValueError, subprocess.TimeoutExpired) as exc:
        print(f"agentctl: {exc}", file=sys.stderr)
        return 2
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
