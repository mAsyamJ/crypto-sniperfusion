# SniperFusion research cycles v1 — bounded, repeatable, critical evidence separation

## Scope

\`harness/research_cycle.py\` implements an **actual finite automated research experiment controller** on top of the V0 synthetic-only surrogate \`harness/research_batch.py\`.

Its output is always:

\`\`\`json
{
  "evidence_class": "SYNTHETIC_SURROGATE_UNCALIBRATED",
  "promotable": false,
  "mainnet_execution": false
}
\`\`\`

The controller does NOT include an LLM agent, external data collector, production-ready order router, real Pump.fun fee model, measured landing delay, clean creator-cluster temporal holdout or an automatic funded signer. It is valuable today as proof of an experiment and improvement **control system**, not evidence of economic edge.

## Execute now

\`\`\`bash
python3 -m unittest discover -s harness/tests -v

python3 harness/research_cycle.py \
  --events harness/fixtures/research/synthetic_lifecycle.jsonl \
  --out harness/runs/synthetic-three-cycles.json \
  --cycles 3

# Inspect each preregistration SHA, development results,
# selection, critic, single sealed toy split and NOT_DEMONSTRATED verdict.
python3 -m json.tool harness/runs/synthetic-three-cycles.json
\`\`\`

Outputs use an exclusive create: the same filename cannot be overwritten. For a new experiment use a new path and a new independently captured dataset.

## Loops

**Continuous technical research loop (current V1):**

\`\`\`text
SYNTHETIC OBSERVATIONS -> strict point-in-time validation
-> MINT-DISJOINT DEMO SPLIT (one sealed mint, NOT temporal/creator cluster OOS)
-> for cycle = 1..N (N <= 20):
     freeze min buyers + liquidity multiple proposal and preregistration SHA
     -> eleven policies S00..S10 in three execution scenarios
     -> compute net equity/drawdown/open losses
     -> deterministic candidate preference and static critic
     -> receipt hash and count 33 tests/cycle
-> choose final candidate using DEVELOPMENT data ONLY
-> freeze candidate hash
-> open toy sealed split once; benchmark vs S00 in 3 scenarios
-> report NOT_DEMONSTRATED and archive immutable receipt
\`\`\`

This genuinely changes bounded research parameters between cycles (\`min_buyers\` and \`liquidity_multiple\`), but does not edit Rust strategy code or train an ML model. It is intentionally not an endless hyperparameter lottery.

**Future production science loop (still blocked):**

\`\`\`text
read-only MAINNET source -> all launches, raw+canonical event ledger and gaps
-> as-of features and separate labels
-> preregister experiment / embargo overlapping outcome horizons
-> pure deterministic Rust strategy engine + independent portfolio ledger
-> chronological and observation-order replays with precise fee/landing models
-> repeat on development with trial counts
-> independent analyst falsification
-> SINGLE untouched creator/time holdout + multiple-test controls
-> Pump.fun Surfpool real program parity -> >=7 days real shadow
-> manual approval to paper-only champion
\`\`\`

LLMs should only propose hypotheses or code patches through the eight existing repository Agent Skills and independent review; they never get terminal access to dedicated test signer keys, mainnet funded keys, or a live broadcaster.

## Quant promotion goals (G0–G7)

| Gate | Objective | Evidence | This PR |
|---|---|---|---|
| G0 | Complete unbiased creation denominator | raw provider coverage, slot gaps, dual time | BLOCKED |
| G1 | Rust integrity and secret-safe tests | fmt/clippy/test/deny + audit | CI validates code; network untested |
| G2 | Chronological/independent accounting | replay, lot, open inventory, actual fees | P0 fixes in main; real costs pending |
| G3 | Credible OOS edge after all costs | temporal/creator-cluster holds, multiplicity, adverse tails | BLOCKED |
| G4 | Deployed program parity | actual M3c Surfpool + new Devnet receipt | M3c prior; pilot opt-in |
| G5 | Live read-only shadow | >=7 continuous days plus coverage | NOT STARTED |
| G6 | Operational chaos | unknown, restart, reorg, gap, kill | PARTIAL |
| G7 | Paper-only promotion | independent deterministic reviewer | BLOCKED |

A target outcome is **consistent positive after-cost *portfolio* value**, not 100% trade winners. A bot that abstains can outperform indiscriminate sniping. Never select solely on top gross ROI or the highest future candle.

## Next goal ordered by real dependency

1. Run the opt-in *Devnet* pilot on an actual supported Devnet mint and independently reconcile its account deltas. A Devnet receipt is proof of program execution, not profit.
2. Build a read-only Mainnet collector with full launch denominator and append-only dual-order events, starting with S00/instant/delayed baselines.
3. Wire actual Pump.fun/PumpSwap on-chain quote/reserve and fee-config models plus measured latency/fill data into a deterministic Rust shared-capital backtester.
4. Implement immutable offline agent hypothesis registry, independent skeptical evaluator, time/creator-cluster split, and sealed holdout access control.
5. Collect the full read-only shadow observation window and decide **whether any strategy survives all adverse after-cost gates**. NO_TRADE is a legitimate outcome.

This roadmap does not authorize live mainnet, direct agent signing or automatic production strategy promotion.
