# ADRs — Architecture Decision Records

**Status:** ACTIVE · **Update rule:** new decisions get new numbers; accepted ADRs are immutable (supersede, never edit).

An ADR records a **choice with lasting consequences** — one that is expensive to reverse and governs future work: framework/SDK selection, licensing posture, safety architecture, integration methods. Execution proof does not belong here (that is `docs/evidence/`); day-to-day rules do not belong here (that is `AGENTS.md`).

## Index

| # | Decision | Status | Date |
|---|---|---|---|
| 0001 | Rust-native hot path, Python as frozen offline oracle | ACCEPTED | 2026-11-02 |
| 0002 | Clean-room licensing policy for upstream references | ACCEPTED | 2026-11-02 |
| 0003 | Solana SDK generation selection (Gen-2, compile-spike driven) | ACCEPTED | 2026-11-02 |
| 0012 | Native Google Harness² integration method | ACCEPTED | 2026-11-05 |
| 0013 | Control Tower dashboard read-only mirror architecture | ACCEPTED (operationalizes the owner-approved dashboard mission files, PR #12) | 2026-11-05 |

**Numbering note:** 0004–0011 are unused — reserved by an interim plan and never written. Do not renumber 0012 (it is referenced by commits and evidence); **the next ADR takes 0014**.

## Candidates not yet written (backfill when touched next)

Decisions currently living only in evidence/status docs that deserve their own ADR when their area is next worked on:

- Read-only mainnet + four-gate live activation (currently `AGENTS.md` §2, `docs/ARCHITECTURE.md`)
- Evidence-class system and `promotable=false` for synthetic data (currently `SOUL.md` §3, `docs/evidence/`)
- S00 NO_TRADE as champion baseline (currently `SOUL.md` §2/§5)
- Public repository visibility trade-off (currently `docs/PROJECT_STATUS.md` header, ADR-0002 context)
- H8 hash-chained preregistration + one-use holdout design (currently `docs/evidence/G0-H8-session.md`)

## Process

1. Copy `_template.md` to `00NN-slug.md` (next free number; see index).
2. Status starts `proposed`; the owner accepts or rejects it.
3. Accepted ADRs never change — a reversal is a **new** ADR that marks the old one `superseded by NNNN`.
4. Link the new ADR from `docs/DOC_INDEX.md` and from the code/docs it governs.
