# ADR 0002 — Clean-room licensing policy for upstream references

Status: accepted (M0) · Date: 2026-11-02

## Context

Four upstreams with three licensing situations (see `docs/UPSTREAM_MANIFEST.md`): chainstack Apache-2.0; yuto no license at all; m8s MIT *claimed* with no license text; mev-bot Ms-PL file contradicting its MIT README. We need their engineering knowledge without creating licensing or taint risk.

## Decision

1. **chainstack (Apache-2.0): reference-and-reimplement.** We may read its code to learn verified behavior (it is our parity oracle by design) and re-implement that behavior in original Rust. We do not copy source files. Apache-2.0 attribution is recorded in the manifest; derived artifacts carry a NOTICE pointer if any nontrivial expression is unavoidable (goal: none).
2. **yuto, m8s, mev-bot: strict clean-room.** Our code must be original. Interaction with their repos is limited to (a) reading for architectural understanding, (b) documenting their behavior/defects in our docs, (c) extracting *facts* (program IDs, published endpoints, observed thresholds) which are not copyrightable expression. **Never**: copying functions/structs/tables of code, transliterated control flow, or their fixture data into our tree.
3. **Protocol truth comes from official sources** (`pump-docs` IDLs/docs, on-chain fixtures, vendor API docs), not from any upstream repo — so clean-room constraints never block correctness.
4. `pump-docs` (pump.fun's public docs) is treated as authoritative documentation; quoting factual tables (account orders, error codes) into our protocol modules with citation is fine — facts and interfaces, not creative expression.

## Rationale

- Ms-PL §3(D) would virally condition source distribution of anything derived from mev-bot's code; MIT-without-text and no-license both fail to grant the permissions we'd need. Only Apache-2.0 is compatible — and even there, independent expression avoids notice-burden ambiguity.
- The audit showed upstream code we'd *want* is entangled with defects (hardcoded 1% fees, dead endpoints, stub handlers), so copying was a bad engineering trade even before licensing.

## Consequences

- Slightly slower feature migration; mitigated by official IDLs being complete (V3/PumpSwap-V2 are documented better upstream-official than in any of the four repos).
- Any contributor must not diff-copy from yuto/m8s/mev-bot; review checklist item added (milestone REVIEW step).
- mev-bot's Ms-PL does not taint us by mere local reading; the risk would be copying — prohibited.
