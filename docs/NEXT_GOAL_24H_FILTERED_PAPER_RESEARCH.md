# Next Goal — Universal Filtering, Multi-Source Coverage, 24/7 Deterministic Paper Profitability

**Role:** Supporting engineering acceptance specification only; NOT an alternate `GOAL_PLAN_PROMPT.md` and not a second agent entrypoint. `docs/GOALS.md` owns current milestone state; `SESSION_LAUNCHER_PROMPT.md` is the sole entrypoint.

**Date:** 2026-11-07 · **Status:** POINT-IN-TIME IMPLEMENTATION CONTRACT · **Reviewed baseline:** main at 314399959ab5f7573a25df77eed25991d9cae907.  
**Research:** `docs/research/2026-11-07-multisource-token-reach-and-filtering.md`. **Execution mission:** `GOAL_PLAN_PROMPT.md` and `SESSION_LAUNCHER_PROMPT.md`.  
**Security authority:** `AGENTS.md` §2, `SOUL.md` and ADR-0002/0012/0013. All mainnet observations are READ-ONLY; all trades here mean historical or prospective PAPER trades. The owner has not authorized funded orders or mainnet broadcasts.

## Objective

Make SniperFusion an honest 24/7 Solana research and paper trading service that can find diverse new tokens without trusting a single screener, deterministically **filter every input and decision boundary**, survive restarts/outages with no AI dependency, and evaluate preregistered strategies against S00 with net liquidation economics and immutable evidence. The objective is *proven positive after-cost risk-bounded expectancy*, not a guaranteed profit, invented win rate, or infinite agent loop.

## What is proven / what is missing (at baseline)

D1a verified 232 on-chain `create_v2` launches across 3,001 complete slots (~11 minutes, 181 SOL quotes, 51 alternative quotes). No strategy has been priced against that real cohort yet. Engine B demands point-in-time reserve and fee state, SOL-only quote handling. Read-only dashboard represents receipts but is not a broker. The H8 registry, paper math, PF1 oracle, and Google Harness² deterministic channel exist; model-backed Harness² and 7-day forward shadow are not enabled. S00 remains champion: `EDGE_NOT_DEMONSTRATED`.

**First critical path:** D2A → D2B → PR2. Source diversification and AI features must not displace these outcomes.

## Architecture zones (separate trust boundaries)

```text
Chain raw (RPC / WSS / Yellowstone)                  Third-party documented public APIs
          |                                                        |
 Verified program ID, slot, account owner                Allowlisted host, schema, quota
          |                                                        |
          +------------------ Provider Admission ------------------+
                                     |
                         Deduped append-only event journal
                                     |
                          Causal as-of MarketBook
                                     |
             G1 Source → G2 Token2022 → G3 Wallet/creator provenance
                 → G4 Liquidity/exit → G5 Organic quality
                     → G6 Costs → G7 Risk authorization
                                     |
              ALLOW_PAPER             WATCH              BLOCK
                    |                   |                  |
      PaperCapability + PaperBroker  observe only      immutable veto
                    |
         PF1 independent NAV / all-in lamports
                    |
               H8 registry / critic / OOS holdout
                    |
               read-only Control Tower
          -------- AIR GAP / NO LIVE KEYS --------
              isolated optional Harness² lab
          (may propose research, never control daemon)
```

Canonical identities: `(chain_genesis, mint, venue, pool)`, chain slot/commitment, source ID/version, first-seen, provider timestamp, locally known-at timestamp, payload hash, data completeness, policy version, evidence class. At decision time use only observations then available; a historical provider response fetched later is NOT a valid contemporaneous signal. Every filtered-out candidate remains counted to prevent survivor bias.

## Universal filter rules (every subsystem)

Required verdict: `ALLOW_PAPER | WATCH | BLOCK` + stable reasons, immutable inputs, evidence hashes, TTL, policy version and optional uncertainty. Pure deterministic code, no model/network inside the filter.

**G1** provider/origin: valid network/genesis, mint identity, payload size, schema, timestamps/finality, duplicate and poison checks.  
**G2** Token-2022: recognized mint account owner, mint/freeze authorities, transfer hook program and metas, transfer fee, permanent delegate, non-transferable, default account frozen, unknown extensions; *no unrecognized token passes*.  
**G3** creator/wallet: insider concentration, shared funding graph, bot/wash volume, cost basis, historical serial-rug pattern; unknown provenance never equals verified-alpha.  
**G4** exit feasibility: as-of vaults, reserves, account ownership, supported quote, executable SELL route, price impact and depth, bounded risk of liquidity withdrawal; no exit means NO new entry.  
**G5** market quality: non-sponsored flow, unique independent buyers, bidirectional activity, organic momentum/volume, manipulation checks. Boosts/ads and external scores alone cannot approve a token.  
**G6** real economics: fees, Token-2022 transfer tax, priority fee, failed-transaction burn, rent, latency, slippage, no-fill and exit proceeds under stress.  
**G7** portfolio authority: exposure, creator and mint limits, drawdown, cooldown, duplicates, kill switch, `RiskEngine::authorize`; cannot bypass typed approval.

Beyond tokens, **every subsystem** has admission: API adapter (network/schema/auth), on-chain IDL/venue (fixture/invariant), wallet signals (cost-basis/funding provenance), research datasets (completeness/causality), strategy registry (preregistration and sample budget), Harness² proposals (sandbox/safe-diff and benchmark), paper broker (G1–G7), dashboard artifacts (hash/shape). Missing optional enrichment may degrade quality; missing mandatory state halts NEW entries. Existing open paper positions preserve a separately limited risk-reducing exit state machine.

## Proposed module ownership (validate with code review before creating files)

- `crates/sniper-core/src/source.rs`: `ProviderId`, `ObservedFact`, `KnownAt`, `CandidateKey` and provenance schema.
- `crates/sniper-core/src/filter.rs`: pure, versioned `FilterPolicy`, `AdmissionVerdict`, `RiskReason`, attestation types; NO network or signing dependencies.
- `crates/sniper-runtime/src/ingest/`: existing provider/supervisor reused; new provider adapters (DexScreener REST/WS and Jupiter Tokens V2 first), bounded queues, rate budgets, reconnect, quotas and gap receipts. No untrusted metadata URL fetch.
- `crates/sniper-runtime/src/market/` or `harness/`: D2 reserve/fee/config state collection with on-chain provenance and quote-mint validation. Preserve existing Engine B and PF1 math.
- `crates/sniper-runtime/src/security/`: typed SPL/Token-2022 authority and transfer-hook checks, creator/holder risk and simulated sellability.
- `crates/sniper-runtime/src/service/`: supervised single-writer append-only journal, restart-safe paper mode, health, deterministic catchup, and safe no-new-entry degraded state. Rust hot path never depends on AI uptime.
- `crates/sniper-strategy/`: preregistered strategies S11–S14 and EX exits, cost and liquidation oracle. `harness/`: H8 and model-independent evaluator. `integrations/harness2/`: isolated research improvement only.
- `crates/sniper-observability` and `crates/sniper-dashboard-api`: unchanged read-only trust boundary per ADR-0013.

## Work packages — one bounded local-agent session each

| Package | Dependency | Falsifiable acceptance / what to implement |
|---|---|---|
| **D2A** | D1a complete | Verify locally present 232-line raw cohort and immutable manifest/hash (if absent: `BLOCKED_MISSING_RAW_DATA`). Build red-first tests and causal snapshot contract for creation reserves + actual decision-time state. Classify 181 SOL and 51 alt-quote. No guessed price. |
| **D2B** | D2A | Slot-proven account/vault/reserve/fee/config collector; futures cannot leak; missing or unsupported Token-2022/fee/quote data yields explicit abstention, not optimistic fill. |
| **PR2** | D2B | Run Engine A/B on identical raw real cohort with PF1 independent NAV, every cost/failure/unexitable exposure counted and S00 baseline. Real-data *historical paper* receipt, NOT live execution or forward shadow. |
| **F0** | PR2 or safe independent code task | Add canonical source admission and G1/G2 pure policy + Token-2022 negative controls; insert a single mandatory entry authorization gate, no bypass. |
| **F1** | F0 | G3–G7: wallet/creator provenance, liquidity and exit, organic quality, all-in costs, portfolio risk. Produce deterministic adversarial tests and reason-coded WATCH/BLOCK. |
| **API0** | F0 | Implement DexScreener REST/WS + Jupiter Tokens V2 candidate adapters using official docs, allowlisted domains, schema validation and offline fixtures. No live trading, no unrestricted URL fetch. |
| **API1** | F0 + API0 | Expand **first-party chain decoders** to Raydium LaunchLab / LetsBonk and Meteora DBC; official version/pins, differential fixtures, separate per-venue coverage receipts. |
| **API2** | API1 | Measure incremental coverage of Birdeye, Bitquery, CoinGecko/GeckoTerminal, Shyft, GoPlus, RugCheck, Orca and/or Helius; integrate ONLY when auth, terms, fees, quota and positive novelty/safety value are verified and authorized. |
| **OPS24** | PR2 + F1 + API0 | 24h deterministic NO-LLM paper daemon test and fault injection: 429, gap, reorg, 1000-event burst, stale state, disk-full, SIGKILL/restart, replay hash equality, secret leak checks. No signer linked. |
| **S11EX** | PR2 + F1 | First real-cohort S11 causal quality-entry + EX exit preregistration and H8 evaluation; compare S00/S02/RX, full net costs, clustered OOS tests and stress. Fail honestly if sample insufficient. |
| **S12/S13/S14** | sufficient independent cohorts | Graduations, genuinely multi-day survivors, proven leader wallet alpha; no future features or re-entry of previously rejected naive following. |
| **H2SAFE** | existing deterministic Harness² | Harden benchmark proposal/critic sandbox, negative-controls and isolated worker boundary; model-backed remains operator-BLOCKED. RL only offline with rewards from PF1 oracle, OPE diagnostics and protected holdout. |
| **SHADOW7** | gates G-A through G-E | ≥7 consecutive days *prospective* read-only strategy shadow with immutable daily receipts and independent after-cost P&L; G-G human paper-only review. |

**D2A first.** Later stages are *not* done simply because a provider adapter compiled or a dashboard rendered. A safe blocked status can be the correct result.

## 24/7 deterministic operation

Service states: `BOOTING → BACKFILLING → OBSERVING → PAPER_READY → DEGRADED_NO_NEW_ENTRY → RECONCILING → STOPPED`. Reconnect and slot-gap backfill are independent of AI; evidence and policy hashes survive process restart. Single-writer WAL or journal; idempotent replay; duplicate events do not create second orders. Stale provider or missing mandatory fee/holdings/reserve forces no new paper entries; independently authorized exits retain their own conservative validity guards. Supervisor (systemd or equivalent) is least-privilege with memory/disk/file/network budgets and kill/watchdog. No paid infra, credential deployment or mainnet broadcast from an agent.

24h fault-injected operational soak is **not** the same as ≥7d prospective strategy shadow, and may not be reported as such.

## Security acceptance and attack fixtures

Test all Token-2022 variations: TransferHook with malicious extra accounts, PermanentDelegate, TransferFeeConfig cap, mint/freeze, NonTransferable, default frozen, unknown extensions; unknown quote, fake pool/vault, liquidity drain, sell only partly possible, wash volume, funding-linked creator wallets, misleading metadata, malicious social URL, prompt injection, provider output substitution/429, reorg/duplicate/clock skew, MEV/failed-tx inclusion, nonce/replay, uncontrolled model worker. At least one negative test for each class before exposing any strategy to it. **No external risk score alone licenses a buy.**

Protect developer/operator machines: never read keys, no model in signing path, no unreviewed upstream code in hot path, no executable scripts from API responses; clean-room and dependency pins apply. Any new security bypass is an urgent package and takes priority.

## Strategy and Harness² gates

S11: under-6h causal organic selection with *currently* observable curve slope, holder growth and developer provenance; never use 6–24h eventual fill as a feature for an earlier entry. S12: quality-filtered migration; S13: 24h–14d survivor (requires multi-day data); S14: verified-real wallet leaders rather than W01 naive follow. EX: volume death/dev dump/liquidity drain/staged exits mandatory for each policy. All P&L modeled at actual decision-time achievable liquidity with failure-burn, slippage, fees, and uncertain no-fill.

H8 hash-chain preregistration FIRST; ≤20 trials per bounded run, multiple-testing correction, cluster unseen creator/day cohorts. SOUL G-A: ≥30 preregistered trials or ≥100 paper trades across ≥3 complete cohorts; G-B CI excludes zero and net edge > S00; G-C bounded CVaR95 and drawdown; G-D +100% fees/+2 slots/50% no-fill; G-E one-use sealed holdout; G-F ≥7-day prospective shadow; G-G human paper review. Synthetic/fork permanently non-promotable. `EDGE_NOT_DEMONSTRATED` remains valid outcome.

Google Harness² pinned genuine upstream optimizes research instructions, skills and tests, not production buy decisions. The model-backed isolated worker is not operational or authorized until separately provisioned; offline bandit/RL is only research with real causal reward/outcome constraints, not a promise to maximize live profit.

## Per-session discipline and receipts

One agent-invoked session runs L0 ORIENT → L1 SELECT → L2 ARCHITECT → L3 RED-FIRST BUILD → L4 VALIDATE → L5 RUN → L6 INDEPENDENT EVALUATION → L7 DECIDE+RECORD. Record exact exit codes, evidence class, rejected tokens, missing data, fee/fill uncertainty, source caps, negative-control and security outcomes, and “what is NOT claimed”. Update living `docs/PROJECT_STATUS.md` + `docs/GOALS.md` only with supported evidence; write-once `docs/evidence/` or `harness/runs/` receipts and local conventional commits. **Never push**, change credentials/visibility, buy paid APIs, sign/send funded transactions or create unbounded autonomous research loops without separate explicit operator authorization.
