# Threat Model — Solana Sniper Fusion

Scope: self-hosted sniper trading Solana memecoels. Assets: operator capital (SOL), signing keys, infrastructure integrity, decision correctness. Trust boundaries: (1) chain+providers → us; (2) token creators/adversaries → our parser/risk engine; (3) our config/secrets → logs/artifacts; (4) our runtime → network endpoints.

## Adversary classes

A1 **Malicious token creator** (primary): crafts launch txs, metadata, holder distributions, authorities to bait snipers. A2 **MEV/co-snipers**: front-run our buys, sandwich exits. A3 **Compromised/malicious provider**: forged/duplicated/reordered events, withheld fills. A4 **Supply-chain**: crate/npm/pip compromise, typosquats. A5 **Local attacker**: secret exfiltration via logs, artifacts, caches.

## Attack surface & mitigations

| ID | Threat | Vector | Mitigation | Verdict-bearing |
|---|---|---|---|---|
| T1 | Malformed ix/event data crashes decoder | A1 | Decoders are total functions: unknown discriminator/short buffer/absurd lengths ⇒ typed error event, never panic; fuzz + property tests on all parse boundaries (M2) | — |
| T2 | Metadata SSRF/RCE-bait | A1 | URI allowlist (https/ipfs), DNS-rebind-safe fetcher, content-length cap, timeout, no redirect-to-internal, cache; socials are reputation-only — never safety input (M5) | — |
| T3 | Token-2022 hostile extensions (permanent delegate, transfer hook, transfer fee, freeze/mint authority) | A1 | Decode extensions from mint state; denylist unsafe; venue-aware authority interpretation (protocol-owned ≠ renounced) (M5) | BLOCK |
| T4 | Honeypot / unsellable token | A1 | Sell-route validity check (accounts, hooks, liquidity, slippage); LiteSVM dry-run where feasible; inability to prove sellable ⇒ fail closed (M5) | BLOCK |
| T5 | Creator-linked concentration + bundled buys | A1 | Creator initial buy exact math; clustered holdings (confidence-labeled); fresh-buyer anomaly; top-1/10/20 thresholds (M5) | BLOCK/WATCH |
| T6 | Dev dump post-entry | A1 | Event-driven DevFlowMonitor: creator-linked net selling pressure **relative to executable liquidity** → protective exit (M5) | exit trigger |
| T7 | Forged/duplicated provider events → double buy | A3 | ChainEventId dedup; commitment upgrade; signature verification of observations where available; idempotent submission (M2/M3) | — |
| T8 | Sandwich/front-run at entry/exit | A2 | Slippage+impact bounds in RiskAuthorization enforced at build; tips capped; venue-specific fee modeling; never widen bounds at runtime (M3/M7) | BLOCK on econ |
| T9 | Provider ack-lies / bundle never lands | A3 | Ack≠landed: OBSERVED state only on signature+balance verification; UNKNOWN→reconcile; no blind re-buy (M7) | — |
| T10 | Stream gap hides rug-in-progress | A3 | Critical-gap ⇒ halt entries + reconcile; staleness ceilings on all facts (M2) | WATCH/BLOCK |
| T11 | Secret leakage to logs/artifacts/commits | A5 | Tracing redaction filter from first log line; no key material in error messages; secret-scan in CI; fake keys in all fixtures; `.env*` gitignored | — |
| T12 | Supply-chain (crates) | A4 | `Cargo.lock` committed; `cargo audit`/`cargo deny` in CI when available; no git deps; upstream code never built into our tree; upgrade = regression event | — |
| T13 | Config error moves real money | human | Typed config, `deny_unknown_fields`, unit-checked fields; money-moving defaults off; four-gate live activation; startup validation refuses unsafe combos | — |
| T14 | Stale fee/price/reserve data trades at wrong economics | A1/A3 | fee_config + curve/pool state cached with as_of_slot+TTL; stale ⇒ WATCH/BLOCK; all math i128/checked (M3) | WATCH/BLOCK |
| T15 | Crash mid-trade → duplicate/lost position | ops | Durable journal pre-submission; stage-exact recovery; dedup by intent nonce+signature across restarts (M3/M7) | — |
| T16 | Kill-switch bypass | A1/A2/ops | Kill switch gates RiskAuthorization issuance (not just submission); live gates are compile-time + runtime independent | — |

## Safety invariants (must hold in every mode)

1. Default builds cannot construct a broadcast-capable object.
2. Observer/replay/paper produce zero signed spend orders (tested).
3. No hardcoded fee rates; no float money math; no unchecked overflow.
4. Unknown/stale mandatory safety fact ⇒ not ELIGIBLE, ever; no score overrides a BLOCK.
5. ELIGIBLE means "criteria met", never "safe" or "profitable".
6. Timeout ≠ failure; failure classification precedes retry; retries never mint fresh buys.

## Residual risks (accepted, tracked)

- Providers can withhold events entirely (nothing detects silence beyond gap heuristics) — mitigated by multi-source subscription (M2) and shadow-mode parity checks.
- LiteSVM diverges from live runtime behavior (feature-set drift) — fixtures + eventual shadow trading quantify; never claimed as mainnet verification.
- Social/enrichment APIs (GoPlus/RugCheck, M7+) are optional and untrusted; cached results carry source+freshness.

## Test mapping

Every threat ID maps to at least one test class in `docs/TEST_PLAN.md` (fuzz=T1/T2, property=T14, fault-injection=T9/T15, safety-mode=T13/T16, intelligence-fixtures=T3–T6).
