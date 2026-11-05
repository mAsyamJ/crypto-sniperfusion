# SniperFusion Continuous Research Loop — executable V0 + target integration

**Archive status (2026-11-05):** LIVING — command reference; on conflict `AGENTS.md` §7 wins.

Date: 2026-11-03. Branch `codex/research-harness-agent-skills-20261103`.

**Status:** V0 synthetic surrogate + P0 correctness fixes + eight agent skills + CI. **NOT** production alpha, unbiased historical backtest, validated mainnet prices, read-only seven-day shadow, or live execution. No strategy has a demonstrated 100% win rate or verified positive live expected value. The attached deep-research report is the analysis basis; its code assertions were independently checked against this repository before patching.

## 1. Architectural boundary

```text
REAL MAINNET (future read-only collector)
  -> immutable raw feeds + source/commitment/gap receipts (NOT BUILT)
  -> canonical observation/chain-order timelines (NOT BUILT)
  -> protocol lifecycle reconstructor (NOT BUILT)
  -> point-in-time feature store and isolated labels (NOT BUILT)
  -> deterministic 10-strategy Rust batch runner (NOT BUILT)
  -> independent, portfolio-level economic evaluator (NOT BUILT)
  -> offline AI hypothesis council (SKILLS PROVIDED, AUTOMATION NOT BUILT)
  -> statistical holdout/fork/shadow gate (NOT BUILT)
  -> paper-only champion config (NEVER LIVE ACTIVATION)

V0 IMPLEMENTED: synthetic JSONL observations
  -> strict validate/causal-availability checks
  -> S00 no-trade + S01..S10 independent surrogate virtual portfolios
  -> mark-to-conservative-exit portfolio results
  -> deterministic diagnostic hypothesis prompts, NOT AI inference
  -> non-promotable synthetic batch receipt
```

The existing real Rust execution engine remains architecturally independent: `sniper-core` authorization; `sniper-protocols` official protocol maths/builders; `sniper-runtime` H2 broker, journal, lifecycle and replay; `sniper-app` CLI. Never call LLMs in the Rust hot path, share a signer with research, or change live risk parameters from research outputs.

## 2. Run now on any Python 3 machine

```bash
python3 -m unittest discover -s harness/tests -v
python3 harness/research_batch.py validate \
  --events harness/fixtures/research/synthetic_lifecycle.jsonl
python3 harness/research_batch.py batch \
  --events harness/fixtures/research/synthetic_lifecycle.jsonl \
  --out harness/runs/demo-batch-v0.json
python3 harness/research_batch.py diagnose \
  --report harness/runs/demo-batch-v0.json

# Rust regression gates, where rust toolchain is installed:
CARGO_BUILD_JOBS=2 cargo fmt --all -- --check
CARGO_BUILD_JOBS=2 cargo clippy --workspace --all-targets --all-features -- -D warnings
CARGO_BUILD_JOBS=2 cargo test --workspace
```

`harness/runs/` is ignored by git. The batch CLI refuses overwriting existing receipts. Re-run with a new file name. All outputs are deterministic for the same file/config, but are **scientifically non-promotable**. This separate V0 interface **does not** replace `harness/snf-harness` H4 `promote`, which continues to require a real shadow completion receipt.

## 3. Strategy specification and first ablation

All strategies share the same input stream, fee/latency surrogate, virtual cash, spend, safety veto and exit horizon. Strategies are evaluated in separate independent wallets; shared capital remains future work. The simple price/quote rule is deliberately uncalibrated; it is a data-contract and negative-control smoke test, not on-chain curve math.

| ID | Entry policy | Experimental question |
|---|---|---|
| S00 | No trade | Is abstaining better than all demonstrated alternatives? |
| S01 | Immediate eligible | Does information-free speed help? |
| S02 | Age 5s | Is five seconds of observation useful? |
| S03 | Age 15s | Is a longer buffer worth late entry? |
| S04 | Active + buyer breadth/net flow | Is independent trading demand informative? |
| S05 | Dip then rebound | Can early reversals be selected without hindsight? |
| S06 | S04 + no creator sell on observation | Does a creator-flow veto reduce losses? |
| S07 | Active curve at 70–90% progress | Does reserve regime affect expectation? |
| S08 | Near completion at >=90% | Is graduation anticipation viable? |
| S09 | Canonical pool phase with buyers | Is post-migration entry better? |
| S10 | High exit-liquidity + positive flow | Do high sellability requirements reduce tails? |

No strategy is assumed to win. No unverified profitability should be marketed. `safety!=ELIGIBLE` or missing coverage disables all new buys. All open inventory is marked at a conservative exit, not ignored as zero-cost unclosed trades; no exit liquidity => zero exit value.

## 4. Specialist skills and agent ownership

Repository-local Agent Skills are in `.agents/skills/*/SKILL.md`. Each owns an independently verifiable research artifact.

| Skill | Input responsibility | Output |
|---|---|---|
| pump-protocol-forensics | current official docs, on-chain program/IDL, local fork | versioned protocol truth table |
| memecoin-lifecycle-research | all creation events, transition observations, gaps | per-mint lifecycle coverage |
| sniper-microstructure | launch cohorts, flow, liquidity, creator behavior | falsifiable trading hypothesis |
| sniper-quant-research | preregistered candidate and unchanged cohort | economic + statistical evaluation |
| sniper-security-redteam | account permissions, malformed feeds, prompt injection | reproducible severity/findings |
| solana-execution-reliability | blockhash/priority/Jito/slot delay | landing and capital safety evidence |
| research-harness-conductor | data hashes, agent outputs, experiment registry | controlled offline research cycle |
| independent-strategy-critic | frozen candidate, withheld evidence | independent accept/reject/inconclusive |

Agent role assignment is static; an independent critic must not be the proposer. All agent-generated snippets, web citations, token metadata and LLM outputs are untrusted.

## 5. Repeating loops

### Per-event runtime loop (future real collector, deterministic only)
OBSERVE -> VALIDATE SOURCE/FRESHNESS -> DECODE -> UPDATE LIFECYCLE -> TECHNICAL SAFETY -> STRATEGY ROUTER -> RISK AUTHORIZATION -> (operator-only funded mode) ORDER BUILD/SUBMIT -> RECONCILE -> POSITION/EXIT. Any unknown safety/data/reconciliation state => NO_TRADE. No LLM allowed here.

### Per-batch experiment loop (V0 demo runnable; real evidence future)
FREEZE dataset+SHA+cohort and strategy configs -> RUN S00-S10 on exactly the same input -> EVALUATE full virtual equity and rejected/exited positions -> DIAGNOSE costs, liquidity and category failure -> independent audit -> archive immutable receipt. V0 exposes validate/batch/diagnose only.

### AI hypothesis loop (agent skills ready; automatic orchestration future)
Select a **development** receipt -> Evidence Curator summarizes loss decomposition -> Quant/Protocol/Security agents propose independently falsifiable improvement -> Skeptic defines negative controls -> freeze hypothesis before seeing holdout -> coding agent proposes a small Rust/config patch -> run tests, local fork, sealed holdout and shadow -> deterministic paper-only promotion. When a holdout is opened, retire it and use fresh data for revised hypotheses; maintain an experiment count.

### Daily data quality loop (future)
Record *all* Pump.fun create events including untraded and failed launches, observe provider slot completeness, track reorg/commitment upgrades, compare second provider parity and close any gap before research. No collector exists yet, so do not claim daily coverage.

### Weekly strategy review (future)
Summarize all candidates incl. rejected/no-trade, tail loss, fee/tip dependence, creator/venue/regime drift, negative controls, outlier dominance, and confidence intervals. Approve research hypotheses, never automatically activate funded trading.

## 6. Acceptance gates before any alpha claim

- G0: full creation denominator, chain + observed ordering, gap/censor receipts.
- G1: Rust fmt, Clippy, tests, dependency+secrets scan and no-broadcast proof.
- G2: replay chronology, mint-specific lot accounting, future-data rejection, independent portfolio reconciler.
- G3: untouched temporal + creator-cluster holdout, multiplicity correction, positive adverse after-cost expected value with bounded tails.
- G4: local fork economic balance reconciliation for supported Pump/PumpSwap paths (not historical execution).
- G5: >=7 consecutive days real read-only shadow with timestamp and coverage receipts; longer forward period for strategy claims.
- G6: fault injection and defensive stale state / kill switch behavior.
- G7: paper-only immutable promotion, independently approved. Mainnet funding separately operator-authorized.

The V0 synthetic workflow **cannot** satisfy G0, G3, G4, G5, G6 or G7 by itself. Statistical claims remain `NOT_DEMONSTRATED`.

## 7. PR sequence to reach the true end-to-end system

- **PR-A (this branch):** replay input-order fix + cross-mint partial lot accounting; V0 surrogate batch, fixture, tests, agent skills, documentation, CI.
- **PR-B:** read-only Solana observation collector + full pump lifecycle, dual time/order, raw append-only data, provider parity/gap backfill, quote/fee config versioning.
- **PR-C:** typed pure Rust strategy SDK, real curve/PumpSwap fill evaluator, independent accounting, shared-capital portfolio, 10-strategy historical batch from complete cohorts.
- **PR-D:** registered hypothesis+trials, sealed holdouts, clustered bootstrap, negative controls, falsification and report/diagnose engine.
- **PR-E:** isolated offline LLM research orchestration, fork parity for all supported paths, >=7-day shadow (cannot be fabricated or expedited), deterministic paper-only gate.

Explicit upstream reference: https://github.com/pump-fun/pump-public-docs ; Solana landing: https://solana.com/docs/tools/transaction-landing ; Jito: https://docs.jito.wtf/lowlatencytxnsend/ .

## 8. First engineering issue/next handoff

Implement real `RawObservation` and `CanonicalChainEvent` as distinct types with provider received monotonic + chain slot/tx index/instruction coordinate; require complete source denominators and source hashes. Preserve commitment evolution. Decode latest V3 crossing events and fee config, and supply no-lookahead real snapshots to a pure strategy interface. This is a blocking dependency for any genuine after-cost OOS conclusion.

Never advertise 100% win rates. The project's target is constrained expected-value research, not impossibly certain predictions.
