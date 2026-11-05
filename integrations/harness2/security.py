"""Security primitives for the Harness² integration (ADR-0012).

The upstream Codex substrate intentionally runs model agents with sandbox
disabled and approvals bypassed. On this host — which has access to
operator RPC credentials and (elsewhere) wallet tooling — that posture is
unacceptable. This module centralizes the boundaries:

- `scrubbed_env()`: allowlist child env; NEVER forwards SNF_*, model
  credentials, or anything key/token/secret-like; forces
  LIVE_TRADING_ENABLED=false.
- `assert_no_isolated_worker_need(mode)`: model-backed modes raise
  BlockedModelError unless the operator has explicitly enabled the
  isolated worker (a file marker created only by following
  integrations/harness2/worker/OPERATOR_CHECKLIST.md by hand).
- `path_jail(...)`: research writes confined under harness/runs/harness2.
"""

import os
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKER_ENABLED_MARKER = ROOT / "integrations" / "harness2" / "worker" / ".operator-enabled"

ALLOWLIST = ("PATH", "HOME", "LANG", "LC_ALL", "TZ", "SYSTEMROOT", "WINDIR")


class BlockedModelError(Exception):
    """Model-backed execution attempted without the isolated worker."""


def scrubbed_env(base=None):
    src = os.environ if base is None else base
    env = {k: src[k] for k in ALLOWLIST if k in src}
    env["LIVE_TRADING_ENABLED"] = "false"
    for key in list(env):
        if (
            key.startswith(("SNF_", "HARNESS2_", "OPENAI", "ANTHROPIC", "GITHUB_"))
            or "KEY" in key or "TOKEN" in key or "SECRET" in key or "PRIVATE" in key
        ):
            env.pop(key, None)
    return env


def isolated_worker_enabled():
    return WORKER_ENABLED_MARKER.is_file()


def assert_model_backing_allowed(model_backed: bool):
    if model_backed and not isolated_worker_enabled():
        raise BlockedModelError(
            "model-backed harness2 execution is BLOCKED: enable the isolated "
            "worker first (integrations/harness2/worker/OPERATOR_CHECKLIST.md); "
            "the Codex substrate runs unsandboxed with approvals bypassed"
        )


def path_jail(candidate: Path) -> Path:
    runs = (ROOT / "harness" / "runs" / "harness2").resolve()
    resolved = Path(candidate).resolve()
    if runs not in resolved.parents and resolved != runs:
        raise ValueError(f"path escapes the harness2 run jail: {resolved}")
    return resolved
