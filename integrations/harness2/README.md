# Harness² Integration (ADR-0012)

**What is installed:** the genuine Google Research Harness² framework
(Apache-2.0), pinned at commit `6868817…` under `.external/harness2`,
installed editable into the project venv `.venv-harness2` (Python 3.12,
extras `[render]`). Install/reinstall: `bash scripts/install-harness2.sh`
(idempotent). Pin + resolved deps: `upstream.lock.json`.

**What is operational:** credential-free deterministic recursion. The
upstream `harness2-demo` runs real Propose–Probe–Compose over
deterministic editors with real Python subprocess execution — verified
(2 tasks ok, exit 0, receipt `harness/runs/harness2-install-smoke/`).
The SniperFusion adapter (`adapter.py`) implements the real upstream
`BenchmarkAdapter` protocol on the same deterministic channel.

**What is BLOCKED:** model-backed substrates (Codex requires
`HARNESS2_CODEX_*` credentials and always runs with sandbox disabled +
approvals bypassed — unsafe on any host with wallet material). The
isolated-worker configuration is PREPARED (`worker/`) but NOT enabled;
`harness2ctl doctor` reports `BLOCKED_NO_ISOLATED_WORKER` until the
operator reviews and enables it.

**Boundary:** Harness² optimizes the research harness (prompts, skills,
workflow guidance). It never evaluates trading strategies — the Rust
P&L V2 oracle and H8 registry remain the financial authority.
