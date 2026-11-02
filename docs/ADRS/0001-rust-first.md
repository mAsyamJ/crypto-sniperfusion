# ADR 0001 — Rust-native hot path, Python as frozen offline oracle

Status: accepted (M0) · Date: 2026-11-02

## Context

Chainstack (Python, Apache-2.0) is the verified behavioral baseline for Solana memecoin sniping: protocol decoding, listeners, exits, and 38 offline regression verifiers, all green on pinned commit `9e22464`. The other three reference repos (yuto Rust; m8s, mev-bot TypeScript) contribute designs but carry either no license or Ms-PL, and each embeds correctness gaps (10-min risk polling; stubbed buys/sells; unverified Jito bundles). Our objective is correct, economically measurable execution with risk controls none of the upstreams implement.

## Decision

Build ONE Rust Cargo workspace as the production system (per amended architecture: four functional crates first). Keep the chainstack checkout pinned and frozen **exclusively** as a differential test oracle and fixture source; yuto/m8s/mev-bot are read-only design references under clean-room rules (ADR-0002). No Python at runtime, ever.

## Rationale

- Latency-sensitive plane (decode→screen→sign) benefits from Rust's zero-copy parsing, checked integer math, and absence of GIL/polling patterns; the audit's failure catalog (stale wallet polling, in-memory-only position state, ack==success) is exactly what a typed state machine + journal fixes.
- Correctness: money math needs u64/i128 + typed units; authorization needs type-level non-bypassability — both are Rust type-system work, hard to retrofit in Python.
- The oracle pattern preserves Chainstack's tested behavior as ground truth for decoder/builder parity without inheriting its architecture.

## Consequences

- (+) Single deployable daemon; memory-safe parsing of adversarial input; property-testable invariants; LiteSVM-integrated instruction tests.
- (−) We re-implement (clean-room) what upstreams already coded — accepted cost for licensing and correctness control.
- (−) Python skill only needed for oracle maintenance (pinned, frozen).
- Rust generation selection deferred to the M0 compilation spike — see ADR-0003.

## Compliance

Apache-2.0 notices for chainstack-derived *behavioral* reference preserved in `docs/UPSTREAM_MANIFEST.md`; no chainstack source is copied into Rust (clean-room rule applies to code; behavior is re-specified from official docs + fixtures, which is also why G-1/G-2 official-only gaps get implemented at all).
