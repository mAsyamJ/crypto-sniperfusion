# Evidence — Harness² Native Integration + Full-Repo Upstream Integration + W01 Strategy Run

Date: 2026-11-05. Branch `codex/wallet-alpha-upstream-integration-20261104`,
commits `9521aec → (this doc)`. Local only, NOT pushed. `LIVE_TRADING_ENABLED=false` throughout; mainnet strictly read-only; operator RPC env never printed.

## A. Installation (Harness²)

| Item | Value |
|---|---|
| Upstream | https://github.com/google-research/harness2 @ `6868817518876de8fd170c6a8354daca273b6191` (Apache-2.0, Google LLC, “not an officially supported Google product”) |
| Install | `.external/harness2` (pinned detached checkout) + `.venv-harness2` (uv, Python 3.12.3), `uv pip install -e '.external/harness2[render]'` — resolved deps: openpyxl 3.1.5, python-pptx 1.0.2, pillow, lxml, et-xmlfile (pure-Python only) |
| Installer | `scripts/install-harness2.sh` (idempotent, fail-fast, no pipe-to-shell); lock: `integrations/harness2/upstream.lock.json` |
| Verification | import resolves to the pinned checkout; all 7 CLI entrypoints present; **credential-free `harness2-demo --k 1` executed: 2 tasks ok, 0 failed, exit 0** (receipt `harness/runs/harness2-install-smoke/demo.json`, chmod 444) |

## B. Architecture (what is connected)

REAL upstream Python API only — no Google code vendored:
- **Adapter** (`integrations/harness2/adapter.py`): the full 13-method
  `BenchmarkAdapter` protocol (`tasks/domain_of/task_prompt/task_files/
  execute/delivered/trajectory/deliverable_files/deliverable_text/
  base_harness/validate_tree/score` + `spec`/`substrate`) on the
  deterministic channel (`OpencodeSubstrate` +
  `ChannelSpec("opencode","local/deterministic")`). Domain
  `wallet-fee-reconciliation`: base harness embeds the PF0 CE-1
  fee-double-count defect; deterministic proposer applies the real fix;
  `score()` = lamport-exactness vs sealed oracle truth (`leak_globs`).
- **CLI** (`harness/harness2ctl.py` + `agentctl.py harness2` group with
  SAFE_CHILD_ENV): doctor | demo | research | status | report; write-once
  0444 summaries; `promotable=false`.
- **H8 bridge** (`evidence_bridge.py`): preregistration BEFORE
  evaluation; one research command = one trial (rollouts = sub-units);
  holdout references refused.
- **Security** (`security.py` + `worker/`): env scrubber (drops
  SNF_*/HARNESS2_*/OPENAI*/tokens/secrets), `BlockedModelError`,
  run-dir path jail; isolated-worker spec + operator checklist PREPARED,
  NOT enabled — model-backed = `BLOCKED_NO_ISOLATED_WORKER`.

## C. Demonstration (real upstream recursion, deterministic)

`agentctl harness2 research --mode sequential --k 1` → **2/2 tasks ok, 0
failed, 8 real Python subprocess executions, 8 editor calls**; parallel
mode also 2/2. The upstream experience LIBRARY persisted a transferable
lesson per task (`library/k1/seq/.../EXPERIENCE.md`): *“When observed
sell quotes are already net of venue fees — never subtract them a second
time.”* — the exact lesson our PF0 evidence recorded.

## D. Full-repo upstream integrations

- `upstream/handi-cat` @ `f154c670` (NO LICENSE — clean-room only,
  never executed; concepts already reimplemented in wallet_watch.py).
- `upstream/dorado` @ `1aaeda86` (MIT): balance-delta semantics → 3
  differential Rust tests vs our normalizer (consumed-mint −pre;
  per-mint map diffing; inclusive-SOL view reconciles with our
  fee-excluded quote + separate fee).
- `upstream/million` @ `c89ad07c` (MIT): owner graph + clustering
  (funding/fee-payer/co-entry; co-entry-never-bridges; hub>6 exclusion),
  consensus with net-flow gate, ladder-on-cumulative-SOL
  (`harness/wallet_signals.py`, 5 tests incl. a real sign-convention bug
  found+fixed); oracle worked-examples (rotation/dust/no-proceeds);
  feature register `docs/WALLET_ALPHA_MILLION_FEATURES.md` (external-API
  items BLOCKED pending operator terms review).

## E. Strategy run W01 (evidence-gated, honest)

Fresh bounded live capture (1000-sig page at tip 465,586,468: **270
successful** — calmer than the earlier 63/1000 storm; 10 txs fetched),
plus the earlier capture; owner-based buy signals (3 real buys across 3
wallets, mints priced from the trades' own observed amounts):

**Follower (W01 naive follow, all 6 delays): 3 entries each, net −250,308
lamports per arm. S00 NO_TRADE: 0. S00 dominates.** Receipt sha
`1ea8bf20…` (archived `harness/runs/wallet-alpha/wa7/`). H8
preregistration H-0001 + trial 1 recorded BEFORE the verdict via the
bridge (`h8-registry.jsonl`, hash-chained). Leader P&L same window: one
wallet −357,890,743 lamports (real observed loss), others 0/no-trade.
**Verdict: EDGE_NOT_DEMONSTRATED; W01 rejected on real data.**

## F. Tests (exit codes)

| Suite | Command | Result |
|---|---|---|
| Rust workspace | `cargo test --workspace` | 312 passed, 0 failed (both feature sets) |
| fmt/clippy | `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |
| Python harness | `python3 -m unittest discover -s harness/tests` | OK (26 tests incl. owner-graph/consensus/ladder) |
| Harness² integration | `python3 -m unittest discover -s integrations/harness2/tests` | 13 OK |
| Skill lint | `python3 harness/agentctl.py lint-skills` | PASS (19 skills) |

## G. Limitations

- Model-backed Harness² recursion: **BLOCKED** (no credentials here; the
  Codex substrate is unsafe by design without the isolated worker).
- W01 ran on 3 real buy signals with prices derived from the trades'
  own observed amounts (not independent as-of reserve quotes) — a
  bounded honest smoke, not a shadow-grade evaluation.
- External market-data APIs (Helius/DexScreener/GeckoTerminal/Jupiter/
  Rugcheck) remain unreviewed/unactivated by owner decision.
- Live shadow (H3/WA6, ≥7 days) still not started.

## H. Next milestone

1. Operator reviews `integrations/harness2/worker/OPERATOR_CHECKLIST.md`
   and provisions the isolated worker for model-backed recursion.
2. WA4 token-safety layer + conviction cohort-vs-control snapshots once a
   calm multi-hour live cohort is collected (checkpointed sweeps).
3. First real H8 tournament (W02 copyability-ranked vs S00/S02/RX) on a
   multi-hour cohort; only then is any holdout opening justified.
