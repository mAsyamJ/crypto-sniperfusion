# Contributing to SniperFusion

**Version:** 1.0 (2026-11-05) · **Status:** ACTIVE

This repository is operated by the owner with AI coding agents. Human contributions are welcome; the rules below bind everyone and mirror the agent rules — on any conflict, `SOUL.md` and `AGENTS.md` win.

## Read before touching anything

1. `SOUL.md` — doctrine: what winning is, the Ten Laws, the gates.
2. `AGENTS.md` — operating rules: safety, licensing, engineering loop, research loop.
3. `docs/PROJECT_STATUS.md` — what actually exists at HEAD (docs can lag; code and evidence do not lie).
4. `docs/DOC_INDEX.md` — where everything lives.

## Ground rules

- **No push, no merge to main without owner approval.** Work happens on branches; the engineering loop ends with a *local* commit and a PR.
- **Conventional commits** (`feat:`, `fix:`, `docs:`, `chore:` …) — see `git log` for the house style.
- **Secrets never enter the repository** — no keys, endpoint URLs with tokens, signing material, private datasets, or market-data agreements. See `SECURITY.md`.
- **Every milestone leaves a write-once evidence receipt** in `docs/evidence/{ID}.md` (commands, exit codes, measurements, open defects) and updates `docs/PROJECT_STATUS.md`. A change without a receipt is unfinished.
- **Markdown discipline:** a new or moved document is indexed in `docs/DOC_INDEX.md` in the same PR. One job per document; no new "master prompt" files.

## Engineering loop (per `AGENTS.md` §4)

DISCOVER → SPECIFY ACCEPTANCE → WRITE FAILING TEST → IMPLEMENT → `cargo fmt` → `cargo clippy --all-targets --all-features -- -D warnings` → unit/integration tests → SVM/replay tests where applicable → fault-injection where applicable → REVIEW (diff, secrets, licenses) → EVIDENCE → update status docs → local commit.

A failing gate blocks the milestone; root-cause and fix — never mark unexecuted tests passed, never edit fixtures green without documented protocol-source justification.

## Licensing (ADR-0002 — read it before reusing anything)

- **chainstack** (Apache-2.0): read for behavior, reimplement originally, keep attribution in `docs/UPSTREAM_MANIFEST.md`.
- **yuto / m8s / mev-bot / handi-cat** (no explicit license): **clean-room only** — facts (IDs, endpoints, thresholds) may inform; code, structures, and flow may not be copied or transliterated. Never build or run them.
- **million / dorado / harness²** (MIT / Apache-2.0): behavioral references via their pinned checkouts; never vendored into this tree.
- Protocol truth comes from `upstream/pump-docs` (official IDLs) and on-chain fixtures — never from another sniper repo.
- The repository itself grants no general redistribution license beyond explicit files (see `README.md` §License); do not add one without an owner decision.

## Research contributions

Strategy hypotheses enter through the H8 registry (`harness/research_registry.py`), preregistered before evaluation — never through ad-hoc backtests in issues or PRs. Intake template: `.github/ISSUE_TEMPLATE/research-hypothesis.md`. Synthetic results are permanently non-promotable; claims inherit the lowest evidence class they support (`docs/GLOSSARY.md`).

## Bugs

Open an issue using `.github/ISSUE_TEMPLATE/bug.md`: HEAD SHA, exact command and exit code, expected vs actual, and a link to the evidence receipt if one exists. Never paste secrets or endpoint URLs.
