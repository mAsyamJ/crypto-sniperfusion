# G0/H8 Session Evidence — Provider Unblocked, COMPLETE Windows, Research Registry

Date: 2026-11-04. Branch: `codex/profit-first-h8-pnl-goal-20261104`.
Commits: `aaa881c` (Mission A: collector version-1 fix + checkpoint/resume + budgets), `e2ab4e9` (Mission C: research registry). Mainnet strictly read-only; operator RPC env never printed/committed.

## Mission A — G0 provider and coverage report

**Root cause of all prior failures**: the collector pinned `maxSupportedTransactionVersion: 0`; the provider requires 1 for versioned (ALT) blocks. The H6-session conclusion "Infura cannot serve getBlock" was wrong — our parameter was. Fixed in `aaa881c` with a wire-param regression test.

**Capability receipt** (`harness/runs/rpc-capability/capability-20261104.json`): `FULL_BLOCK_SAMPLE_AVAILABLE`, genesis verified, 3 read-only calls, sample block 534 transactions at version 1, `rpc_source: OPERATOR_ENV_ONLY`.

**COMPLETE block-verified windows** (all gaps 0, all signatures reconciled, write-once 0444 manifests):

| Receipt (harness/runs/launch-cohort/) | Window | Blocks | Creates |
|---|---|---|---|
| `manifest-20261104T143209-32491-*` | 465336532–465336534 | 3 | 0 |
| `manifest-20261104T143329-32712-*` | (7 slots) | 7 | 0 |
| `manifest-20261104T143519-32811-*` | (41 slots) | 41 | 0 |

**Zero creates is a market fact, not a capture failure**: diagnostic sample of 3 live blocks (2,094 txs; 268 pump-program txs) found **235/268 (87%) pump transactions FAILED** and zero create instructions among the 33 successful ones — a congestion/bot storm during the capture period; failed transactions correctly yield no events (oracle parity, tested). Launch capture at scale needs a calmer window or a longer (minutes+) sweep with the new budgets/checkpointing.

PR-1 gate: **small COMPLETE window with zero missing block evidence and independent signature reconciliation — ACHIEVED** (three windows). Complete cohort at meaningful scale — open, resumable via `ckpt-*.json` mechanics.

## Mission B — Engine B on real observations: honestly BLOCKED (data)

Real reserve/fee snapshots per decision do not yet exist (snapshots require account reads at as-of slots for every observed launch — the fee side is now proven (PF0 receipts); reserve snapshots are the next collector capability). No synthetic fallback was presented as real: the V2 audit gate refuses non-reconciling runs, `fee_synthetic_fallback` labels propagate, and `is_reference_fixture()` marks the 25 bps reference.

## Mission C — H8 research registry (PF5 machinery)

`harness/research_registry.py` + 12 tests (`e2ab4e9`): hash-chained tamper-evident hypothesis preregistration (all mandated fields enforced); structural 20-trial bound; **cross-process one-use holdout** (flock + O_EXCL marker + persistent counter + full audit trail — verified with real subprocesses: second open denied in-process, cross-process, and after the first process exits); deterministic temporal + creator-cluster-disjoint splits; label embargo (labels only inside the sealed holdout); Bonferroni + Benjamini-Hochberg corrections. Zero signer/network/promotion paths.

## Mission D — strategy research: preregistration scaffolding only

No strategy claims: G0 real cohort at scale is not yet collected and H3 shadow has not started. The registry's H-0001 template (organic-flow vs S00 at observed 125 bps fees) is the first registered falsifiable hypothesis, unexecuted.

## Gates at session close

`cargo fmt --check` exit 0; `cargo clippy --workspace --all-targets --all-features -- -D warnings` exit 0; `cargo test --workspace` 285/285 (and ×all-features 285/285); `python3 -m unittest discover -s harness/tests -q` 103 OK; secret/license scans clean (CI commands).
