---
name: sniper-security-redteam
description: Attack SniperFusion's data-provider, Token-2022, strategy, secret, journal, replay and AI promotion trust boundaries before any funded execution.
---
# Solana Sniper Security Red Team

Read `AGENTS.md`, `docs/THREAT_MODEL.md`, `crates/sniper-core/src/authz.rs`, and `crates/sniper-runtime/src/lifecycle.rs`.

## Workflow
1. Threat model token authority, transfer hooks, permanent delegate, freeze/default frozen state, fee-on-transfer, unverified quote mint and false sell liquidity.
2. Inject forged, missing, duplicated and reordered provider events; confirm no new entries during missing critical data or ambiguous transaction state.
3. Attempt to forge `AuthorizedOrder`, replay a nonce, exceed global budget, bypass kill switch, or trigger blind resubmission after UNKNOWN.
4. Inject malicious creator metadata and hostile research citations into agent inputs; verify prompt-injection and SSRF boundaries.
5. Check LLM workers have no private keys, signer IPC, mainnet broadcaster, paid infra tokens, production strategy write privilege or promotion credentials.
6. Produce a failing exploit/regression before patching; classify evidence and re-run gates.

## Required deliverable
Severity, impacted trust boundary, reproducible failing test, remediation, proof of non-broadcast, status and residual risk.
