# Solana Dependency Compatibility (M0 spike evidence)

Date: 2026-11-02 · Host: WSL2 linux 6.18, 16 CPU, 7.6 Gi RAM · rustc via rustup.

## Decision

**Generation 2 classic line, workspace-pinned:**

| Crate | Version | Role |
|---|---|---|
| `solana-sdk` | `=2.3.1` | Transaction/message/signature/keypair types (modular crates underneath at 2.2/2.3) |
| `litesvm` | `=0.7.0` | Local SVM execution for offline instruction tests |
| `spl-token-2022` | `=9.0.0` | Token-2022 state/extensions decoding |
| `yellowstone-grpc-client` / `-proto` | `16.0.1` / `14.0.1` (or 17.x) | Geyser streaming — **zero solana-crate coupling** (tonic + proto only), composes with any generation |
| `pump-rust-client` (optional, M3 evaluation) | `0.5.0` (docs reference 0.4.1) | Requires `solana-sdk >=2.1.21, <3` — **only fits Generation 2** |

Toolchain: rustc **1.99.1** (stable, updated from 1.98.0 during this spike); MSRV floor for the chosen set is satisfied by 1.98.x.

## Spike method

Two probe crates in `/tmp` (never the workspace), each: ephemeral `Keypair` → LiteSVM airdrop → **legacy** transaction send → **v0 versioned** transaction build/sign/send → balance assertion (validated fill) → pump.fun program-id parse → anchor discriminator table check. Verdicts come from actual `cargo build`/`cargo run` exit codes, not metadata alone.

### Generation A (chosen): solana-sdk 2.3.1 + litesvm 0.7.0 + spl-token-2022 9.0.0

- **Result: GREEN — probe ran to completion, all assertions passed, exit 0.**
- Resolution: full modular family at 2.2.x/2.3.x (`solana-pubkey 2.2/2.3`, `solana-transaction 2.2.3`, …). No MSRV conflicts on rustc 1.98.0 or 1.99.1.
- API notes captured for M1/M3: litesvm 0.7 uses `latest_blockhash()` (not `get_latest_blockhash`), `send_transaction(impl Into<VersionedTransaction>)` (pass owned), `VersionedTransaction::try_new(msg, &[&kp])` for v0 signing, and **the blockhash must be re-fetched between sends** (stale-blockhash send fails `BlockhashNotFound`).
- Uniquely compatible with `pump-rust-client 0.5.0` (`>=2.1.21, <3`).

### Generation B (rejected today): solana-sdk 4.1.0 + litesvm 0.18.1 + spl-token-2022 12.0.0

- **Result: RED — two independent blockers, both evidence-backed:**
  1. *MSRV:* resolution refused on rustc 1.98.0 — 19 agave/svm crates at 4.4.0 (`agave-feature-set`, `solana-syscalls`, …) require `rustc >= 1.98.1`. Mitigated by updating the toolchain.
  2. *Source-incompatibility:* after updating to 1.99.1, **litesvm 0.18.1 itself fails to compile** against the agave 4.4 family it resolves to (`^4.3.0` requirements resolve to 4.4.0): `ProgramCacheEntry::new_builtin` arity changed, `TransactionContext::new` removed, `SVMStaticMessage` unsatisfied, ALT `lookup()` signature drift, `AccountSharedData` mutability — errors inside `litesvm-0.18.1/src`.
  3. *Downgrade impossible:* pinning the 19 crates to 4.3.0 fails because the agave family is version-locked (`=4.4.0` cross-requirements) and `agave-transaction-view` has no pre-7.0.0 release satisfying the graph. litesvm 0.16/0.17 declare `^4.2.1`/`^4.3.0` and would resolve to the same broken 4.4 family.
- Conclusion: the Generation-4 SVM-testing ecosystem is mid-break right now. **Revisit trigger:** a litesvm release compatible with the agave ≥4.3 family, or mollusk maturing (0.1.0, no deps — too early).

### Generation C (rejected): solana-sdk 3.0.0 / 5.0.0 / solana-transaction 6.0.0

Single transitional releases (3.0.0: n=1 in the 3.x line; 5.0.0/6.0.0: n=1 each), no litesvm/pump-client ecosystem validation, higher-churn API surface (solana-sdk 3.0.0 already drops ~20 modules the 2.x facade carried). Not viable for a system that needs LiteSVM tests + pump-rust-client + Yellowstone today.

## Version landscape observed (index.crates.io, 2026-11-02)

| Crate | Latest | Last 2.x | Notes |
|---|---|---|---|
| solana-sdk | 5.0.0 | 2.3.1 (50 releases in 2.x) | 3.0.0/4.1.0/5.0.0 all post-2.x singles-or-short-lines |
| solana-pubkey | 4.4.0 | 2.4.0 | modular line diverged from sdk majors |
| solana-transaction | 6.0.0 | (via sdk 2.2.3) | 3.1/4.3/5.1/6.0 |
| yellowstone-grpc-client | 17.0.0 | — | tonic 0.14, proto ^14.0.1, **no solana deps** |
| yellowstone-grpc-proto | 14.0.1 | — | client 16.0.1 ↔ proto 14.0.1 pairing used |
| litesvm | 0.18.1 | 0.7.0 ↔ sdk 2.x | 0.8+ tracks agave 3/4 families (broken at 0.18.1) |
| spl-token-2022 | 12.0.0 | 9.0.0 ↔ sdk 2.2/2.3 | 10/11/12 track modular 3.x/4.x lines |
| pump-rust-client | 0.5.0 | — | requires sdk `>=2.1.21,<3` |
| mollusk | 0.1.0 | — | dependency-free SVM harness; watch, not adopt |

## Transaction version support (both generations, verified)

- Legacy and v0 (versioned) message construction, ephemeral signing, and LiteSVM execution **pass in Generation A**. v0-with-ALT (address-lookup-table) construction is exercised at M2/M3 against Geyser fixtures (Chainstack's shreds listener already demonstrates ALT key resolution, `loaded_writable/readonly`, which we mirror).
- Chainstack regression `verify_transaction_v1.py` documents mainnet's v0/v1 envelope behaviors (SIMD-0385 inline config) — mirrored in our parser tests.

## Lock policy

- `Cargo.lock` committed from day one; workspace `[workspace.dependencies]` carries the exact pins above.
- `cargo update` of any solana/agave/svm-family crate is a **regression-triggering event** (re-run full test matrix per `docs/TEST_PLAN.md`).
- RAM control: `CARGO_BUILD_JOBS=2` default (host has 7.6 Gi total, ~2–3.6 Gi free during M0); drop to 1 on swap pressure.
