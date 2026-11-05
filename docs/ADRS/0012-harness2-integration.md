# ADR-0012: Native Google Harness² Integration Method

Date: 2026-11-05. Status: accepted.

## Context

Google Research **Harness²** (`google-research/harness2`, Apache-2.0, pin
`6868817518876de8fd170c6a8354daca273b6191`) is a recursive agent-harness
optimization framework: Propose–Probe–Compose recursion over benchmark
adapters with a persistent per-domain experience library. We want it to
optimize SniperFusion's *research harness* (prompts, skills, workflows) —
never trading strategies directly.

## Decision

Install as a **pinned upstream git checkout + project-local uv venv with
editable install**:

- Checkout under `.external/harness2` (gitignored, never committed),
  detached at the pinned commit.
- venv `.venv-harness2` (uv, Python 3.12), `uv pip install -e '.external/harness2[render]'`.
- `scripts/install-harness2.sh` is the single idempotent entry point;
  `integrations/harness2/upstream.lock.json` records the pin + resolved deps.
- SniperFusion integrates through the REAL upstream Python API
  (`BenchmarkAdapter` protocol, `core/recursion.py`, `core/library.py`) —
  no Google code is copied into this repo.

Rationale for checkout (vs PyPI/wheel): harness² resolves resources
relative to its source root, the pinned revision is exact, and upgrades
are a one-line lock change with a fresh audit. Zero base runtime deps +
`[render]` extras are pure-Python — no transitive risk explosion.

## Alternatives rejected

- **PyPI install**: not verified to exist for this pin; loses source-root
  resource resolution guarantees.
- **Git submodule**: couples our history to upstream availability; the
  repo is already gitignored-by-policy for reference checkouts.
- **Vendoring source into the repo**: prohibited duplication of Google
  internals (mission constraint) and license-noise in a public repo.

## Security posture

- Credential-free deterministic channel only (upstream `DemoAdapter`
  pattern: `OpencodeSubstrate` + `ChannelSpec("opencode",
  "local/deterministic")`).
- Model-backed substrates (Codex: `HARNESS2_CODEX_*`, runs unsandboxed
  with approvals bypassed by design) stay **BLOCKED** until the prepared
  isolated-worker configuration (`integrations/harness2/worker/`) is
  reviewed and enabled by the operator.
- All child processes run under SniperFusion's env scrubber; no RPC/keys
  ever forwarded; outputs write-once 0444 under `harness/runs/harness2/`.

## Consequences

- `harness/agentctl.py harness2 …` becomes the operator surface;
  H8 registry accounting is bridged (rollouts ≠ trials; preregistration
  before evaluation; holdout labels structurally absent from candidate
  context).
- CI runs deterministic harness² checks only, skipping gracefully when
  `.external/` is absent.
