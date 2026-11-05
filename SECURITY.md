# Security Policy — SniperFusion

**Version:** 1.0 (2026-11-05) · **Status:** ACTIVE

This is a public research repository. There is **no funded deployment, no funded key, and no authorized live trading** — the system is structurally read-only toward Mainnet. This document defines what is protected, what is prohibited, and how to report problems.

## Scope

- In scope: this repository's code, documentation, CI workflows, agent tooling (`harness/`), and the operating discipline around them.
- Out of scope: any running funded trading service — none exists. `LIVE_TRADING_ENABLED=false` is the default and live mode requires four independent gates (cargo `live` feature ∧ operator authorization token ∧ validated budget config ∧ kill-switch check).

## Reporting a vulnerability

Do **not** open a public issue for anything secret- or safety-related. Use GitHub's private vulnerability reporting (Security → Report a vulnerability) on this repository, or contact the owner directly. Include: affected file/command, reproduction steps, and impact. Public disclosure only after a fix lands.

## Key and credential policy (hard rules)

1. **Env-only, outside the repo.** RPC endpoints with keys live in a 0600 env file outside the working tree; `.env*` is gitignored; keys are never committed, logged, echoed, or embedded in errors.
2. **Never in prompts or chat.** Keys, endpoint URLs with tokens, and signing material never enter LLM prompts, agent transcripts, CI logs, or run manifests. This has been violated once before (a key pasted into chat); the standing consequence is **rotation** — an exposed key is a dead key, rotated by the operator, never by an agent.
3. **Agents never touch credentials.** No AI job may read, show, rotate, or change RPC secrets or wallet key material, run `--submit`, change repository visibility, or spend funds (`AGENTS.md` §2).
4. **Fixtures use fake keys only.** Any test key in the tree must be demonstrably fake and listed in `.github/secret-allowlist.txt` with a justification comment.

## Enforcement in CI

- History-aware secret scan on every push/PR (H0.4), with the audited exact-substring allowlist above.
- `cargo-deny` (pinned 0.20.2) against `deny.toml`: advisories denied, yanked denied, license allowlist, unknown registries/git sources denied.
- Credential-safe output by construction: `sniper rpc-probe` redacts URLs (`redact_url` strips path/query) — keys are unprintable, not merely filtered.
- Tracing redaction filter from the first log line; no key material in error messages (Threat model T11).

## Supply chain

- `Cargo.lock` committed; no git dependencies; version bumps are regression events.
- Upstream reference repos are read-only, pinned, and never built into this tree (`docs/UPSTREAM_MANIFEST.md`); no-license upstreams are clean-room facts only (ADR-0002).
- Google Harness² is a pinned checkout under gitignored `.external/` with a project-local venv (ADR-0012); its child processes run under the SniperFusion env scrubber and never receive RPC keys.

## Trading-safety invariants

- Default builds cannot construct a broadcast-capable object; observer/replay/paper runtimes are type-enforced non-broadcasters.
- No hardcoded fee rates, no float money math, no unchecked overflow.
- Unknown or stale mandatory safety fact ⇒ never ELIGIBLE; no score overrides a BLOCK (`docs/THREAT_MODEL.md` invariants).
- Synthetic results are permanently `promotable=false`; promoting them to live execution is prohibited everywhere.

## Threat model

Full adversary classes (A1–A5), threat catalog (T1–T16), and test mapping: `docs/THREAT_MODEL.md`.
