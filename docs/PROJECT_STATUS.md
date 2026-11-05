# Project Status

**Repository: `mAsyamJ/crypto-sniperfusion` (PUBLIC — verified 2026-11-03, owner decision: keep public).** Project name: **SniperFusion**. Chain-agnostic naming on purpose: Solana is the only implemented chain today; every doc claim about capabilities remains Solana-scoped until multi-chain code lands behind its own milestone gates.
**Operational consequence of PUBLIC:** strategy logic, risk thresholds, and evaluation results are world-readable. Enforced invariants that make this acceptable: no keys/secrets/`.env` in history (H0.4 scans enforce), no funded credentials, no infrastructure endpoints in docs. Revisit visibility before any live-trading authorization — see ADR-0002.

Living document; update at every milestone close. Format: state, evidence pointer, next.

## Milestones

| Milestone | State | Evidence |
|---|---|---|
| M0 — Audit, baseline, dependency selection, protocol verification | **COMPLETE** (2026-11-02) | `docs/evidence/M0.md` |
| M1 — Functional Rust core, config, risk authz, safe replay runtime | **COMPLETE** (2026-11-02) | `docs/evidence/M1.md` |
| M2 — Pump.fun event decoding, streaming interfaces, fixture parity | **COMPLETE** (M2a differential 320/320; M2b providers+supervision+cache) | `docs/evidence/M2.md` |
| M3 — V2/V3 + PumpSwap lifecycle | **COMPLETE incl. M3c** (real local execution on Surfpool fork, 2026-11-02) | `docs/evidence/M3.md`, `docs/evidence/M3c.md` |
| H2 — Real paper broker (event-sourced) | **COMPLETE** (2026-11-02) | `docs/evidence/H2.md` |
| H4 — Improvement harness CLI skeleton | **COMPLETE** (2026-11-02; H3 shadow window intentionally not started) | `docs/evidence/H4.md` |
| INFRA — Solana disposable wallets + operator RPC validation | **COMPLETE** (2026-11-04; docs-only, no code change) | `docs/evidence/INFRA-solana-rpc-wallets.md` |
| INFRA-2 — Operator RPCs integrated: `sniper rpc-probe` + endpoint resolution | **COMPLETE** (2026-11-04; full engineering loop, 179/179 tests ×2) | `docs/evidence/INFRA-2-rpc-integration.md` |
| INFRA-3 — Mainnet readiness: unfunded watch wallet + `rpc-probe --address` | **COMPLETE** (2026-11-04; read-only only, 249/249 tests ×2 on merged tree) | `docs/evidence/INFRA-3-mainnet-readiness.md` |
| H6 — Canonical dataset contracts + bounded creation-cohort collector | **COMPLETE (code)** (2026-11-04; 266/266 Rust ×2, 55 Python; live cohort coverage INCOMPLETE — provider cannot serve block evidence; G0 stays BLOCKED) | `docs/evidence/H6.md` |
| H7 — Strategy SDK (S00–S10, two engines) + accounting oracle + evaluator/CLI | **COMPLETE (code)** (2026-11-04; Engine A byte-exact Python parity; verdict EDGE_NOT_DEMONSTRATED — S00 dominates on the synthetic cohort; nothing promotable) | `docs/evidence/H7.md` |
| PF0 — P&L audit + on-chain fee provenance | **COMPLETE** (2026-11-04; 5 counterexamples; curve fees OBSERVED 95+30=125 bps, canonical PumpSwap TIERED 125→30 bps; 25 bps reference is fixture-only) | `docs/evidence/PF0.md`, `docs/PF0_PNL_AUDIT.md` |
| PF1 — P&L V2 (reconciled equity, waterfall, deterministic receipts) | **COMPLETE** (2026-11-04; oracle round-trip lamport-exact; deposits/attribution/slippage; V2 CLI bundle; 9 mandated tests; S00 dominates at observed fees) | `docs/evidence/PF1.md` |
| G0 — block-verified real launch cohort | **COMPLETE WINDOW AT SCALE, ZERO LAUNCHES** (2026-11-05; 3,001/3,001 blocks, 0 gaps, coverage COMPLETE, 0 creates; sampled diagnostic: 0 create attempts = absence not failure, 82% program-tx failure; S11/S12 blocked by market structure — next: launch-active hours + M4 launchpad coverage) | `docs/evidence/G0-H8-session.md`, `docs/evidence/G0-WINDOW-COMPLETE-20261105.md` |
| H8/PF5 — research registry + one-use holdout | **COMPLETE (machinery)** (2026-11-04; hash-chained preregistration; structural 20-trial bound; cross-process one-use holdout; BH/Bonferroni; 12 tests) | `docs/evidence/G0-H8-session.md` |
| WA0 — wallet-alpha audit + offline lab | **COMPLETE** (2026-11-04; upstream license map, architecture, synthetic negative-control evaluator) | `docs/WALLET_ALPHA_UPSTREAM_AUDIT.md` |
| WA1–WA3 — wallet normalization, leader P&L, delayed follower | **COMPLETE (code + live smoke)** (2026-11-04; 306/306 Rust, 114 Python; real observed leader −4.96 SOL, unknown-basis wallets exactly 0; follower loses at all delays vs S00; nothing promotable) | `docs/evidence/WA1-WA3.md` |
| WA-int — million + handi-cat deep integration | **COMPLETE** (2026-11-04; million cloned MIT-pinned @c89ad07c, infra-facts + cross-token-bot heuristic + blocklist adopted with attribution; handi-cat clean-room concepts → offline wallet_watch.py alert layer; fixed-60s copyability replaced by WA3 multi-delay per plan; 120 Python tests) | `docs/WALLET_ALPHA_UPSTREAM_AUDIT.md` §amendment |
| H2 — Google Harness² native integration | **COMPLETE (deterministic channel)** (2026-11-05; pinned .external checkout + uv venv, credential-free demo executed, real BenchmarkAdapter recursion verified 2/2, experience library persisted, agentctl harness2 group, H8 bridge, security scrubber + worker prep; model-backed BLOCKED) | `docs/evidence/HARNESS2-INTEGRATION.md` |
| WA-full — full-repo million features + Dorado differential | **COMPLETE** (2026-11-05; owner graph + consensus/ladder signals, oracle worked-examples, Dorado cross-check tests, feature register; 312/312 Rust) | `docs/evidence/HARNESS2-INTEGRATION.md` §D |
| W01 — naive wallet-follow on real signals | **EXECUTED, REJECTED** (2026-11-05; 3 real buy signals, all 6 delays net −250,308 lamports vs S00 = 0; H-0001 preregistered first; EDGE_NOT_DEMONSTRATED) | `docs/evidence/HARNESS2-INTEGRATION.md` §E |
| M4 — Chainstack parity scope closure | PLANNED | — |
| M5 — Token intelligence + DevFlowMonitor | PLANNED | — |
| M6 — Multi-DEX + copy trading | PLANNED | — |
| M7 — Execution routes + reconciliation + fault injection | PLANNED | — |
| M8 — Replay/backtest/benchmarks | PLANNED | — |
| M9 — Hardening + release readiness | PLANNED | — |

## M0 results (summary — verbatim detail in evidence file)

1. **Security audit**: no funded credentials in workspace; upstream install surfaces safe; m8s ships apparent live credentials (flagged, unused, uncopied).
2. **Chainstack baseline**: 38 verifier scripts discovered, **38 executed, 38 passed, runner exit 0**, isolated `uv sync --frozen` env.
3. **Dependency spike**: Gen-2 (solana-sdk 2.3.1 + litesvm 0.7 + spl-token-2022 9) **GREEN end-to-end**; Gen-4 **RED** (MSRV wall + litesvm 0.18.1 broken vs agave-4.4 lockstep). Decision: Gen-2 (ADR-0003). Toolchain updated 1.98.0 → 1.99.1 to give Gen-4 a fair trial (kept).
4. **Protocol audit** (`pump-docs` @ 639b87f): Chainstack's vendored IDLs lack the official **V3 curve trades** and **PumpSwap V2 trades** (17-account families) + multi-hop — Rust scope therefore exceeds Chainstack parity on these (G-1/G-2). Fee schedules are on-chain (`fee_config`) — no hardcoded rates. Synthetic migration + negative i128 `virtual_quote_reserves` captured with formulas and PDAs.

## M1 results (summary — verbatim detail in evidence file)

Workspace: 4 crates (core/protocols/runtime/app), solana-sdk =2.3.1 per ADR-0003, lockfile committed, rustc 1.99.1 pinned, build jobs=2. Gates all green: fmt, clippy×2 (`-D warnings`), tests×2 (8 suites each, 0 failures). CLI demonstrates offline replay/paper end-to-end (7 events → 2 dupes deduped → 5 decisions) with honest refusals for observe-streaming (M2) and live (4-gate + M7). Dual event identity, 3-tier shedding bus, crash-tolerant journal, base58 fixture serde landed.

## Open items / blockers

- None blocking M2. Watch item: host RAM — build jobs capped at 2 (held throughout M1).
- Deferred: `pump-rust-client` 0.5.0 evaluation (M3); Yellowstone bump headroom (16→17) harmless.
- Gen-4 revisit trigger documented in ADR-0003 (litesvm ≥0.19 or mollusk maturity).

## M2a results (summary — verbatim detail in evidence file)

Event decoding + log attribution + differential harness complete: 320 synthetic fixtures (8 kinds incl. old-format truncated), oracle = chainstack IDLParser; comparator result ALL MATCH (4792/4792 comparable fields, exit 0). Two oracle limitations discovered, documented, and excluded with rationale: D-1 vec<defined> unsupported with silent offset misalignment (oracle post-shareholders fields are garbage), D-2 EOF artifact synthesis. Rust decoders are prefix-tolerant per-field (decoded_fields counts), garbage-proof (property test), and sha256-verified against both IDLs. All gates green (fmt, clippy ×2, tests ×2).

## Open items / blockers

- M2b (live provider adapters: WSS logs/blocks, Yellowstone gRPC, SubscribeDeshred detection, reconnect/gap/reorder against real streams, blockhash cache) — NOT STARTED; requires opt-in read-only provider credentials. First task next session.
- PostCompleteBuyEvent layout (synthetic migration pool leg) — pending M3 with SYNTHETIC_MIGRATION.md quoting work.
- Host RAM watch item unchanged (jobs=2 held).

## M3 results (summary — verbatim detail in evidence file)

PDA module (doc-constants verified), exact integer math (curve constant-product, i128 signed effective reserves, bounded-output invariant), six instruction builders (curve v3×3 @17 accounts, v2 buy 27 / sell 26, PumpSwap sell_v2 @17) with doc-table structural tests, order lifecycle machine with journal write-ahead + full crash-recovery test matrix (UNKNOWN mapping, duplicate-submission rejection, entry gating), paper fill engine with fee-in/out round-trip economics. Gates all green (fmt, clippy×2, tests×2: 89 tests across workspace).

## Open items / blockers

- **M3c (next session, first task)**: LiteSVM execution vs real pump.fun/PumpSwap program binaries — needs opt-in program-ELF fetch or committed fixture; PostCompleteBuyEvent layout from synthetic-migration fixtures.
- M4 then proceeds: LetsBonk/LaunchLab parity scope, exit-strategy parity, CONFIG_MIGRATION.md, runtime-Python-independence proof.

## Next actions (M3c → M4)

1. M3c: opt-in program-ELF fixture fetch script (read-only RPC), LiteSVM harness executing builders end-to-end, PostCompleteBuyEvent decode.
2. M4: Chainstack parity closure per CAPABILITY_MATRIX rows C2–C24; `docs/CONFIG_MIGRATION.md`;LetsBonk/StonkFun adapter scope decision.
3. Carry: `pump-rust-client` 0.5.0 build-helper evaluation.

## H-mission results (2026-11-02, branch codex/harness-m3c-security)

- **H0** (`bd5c3b0`): sealed `AuthorizedOrder` trust boundary, durable
  checksummed single-writer journal with crash-tail repair, backfill-proof
  EntryGate, history-aware secret scan + cargo-deny CI, docs visibility fix.
- **H1/M3c** (`b305106`): Surfpool v1.7.0 pinned (sha256 recorded); real
  local `buy_v3`/`sell_v3` + negative control against the deployed pump.fun
  ELF on a loopback mainnet fork; official-IDL conformance fixes
  (`partial_fill` arg, account mutability, ATA prerequisite); live-PDA
  ground-truth tests; deployed-program economics documented (volume
  accumulator SOL routing, quote-era event semantics open).
- **H2** (`f38f1cc`): event-sourced PaperBroker (fills/fees/rent/slippage,
  no-lookahead, migration, ledger crash-recovery, deterministic export);
  CLI paper mode rewired with reconciled P&L receipt accepted by the
  independent evaluator (broker realized 2,080,057 vs evaluator net
  2,068,057 = exactly the separately-booked 12,000 network+tip).
- **H4** (harness skeleton): `harness/snf-harness` — doctor/verify/replay/
  fork/shadow/evaluate/compare/promote with write-once run manifests;
  promotion requires the full gate chain INCLUDING the ≥7-day shadow window
  and never enables live trading. Tests: 167/167 both feature sets.
- **H3**: NOT STARTED (needs ≥7 days read-only observation; the harness
  `shadow` command refuses to fabricate it).

## Research Harness V0 implementation branch — 2026-11-03

Branch `codex/research-harness-agent-skills-20261103` (requires PR review and CI confirmation): fixes identified P0 replay priority reordering and global-mint trade attribution in the paper broker, adds regression tests, a strict offline 11-policy **synthetic surrogate** research batch CLI, immutable-output rules, eight repository-local Solana/quant/security/critic skills, and an operational runbook. See `harness/docs/RESEARCH_LOOP_RUNBOOK.md` for exact scope and commands.

**Remaining gates (NOT COMPLETE):** genuine all-launch historical dataset, on-chain point-in-time feature reconstruction, shared-capital Rust backtester, independent full-portfolio economic reconciler, actual latency/fee calibration, statistical holdouts, automated AI orchestration, complete fork protocol matrix, and >=7-day read-only shadow. The V0 surrogate has `promotable=false` and cannot activate mainnet trading. Do not claim a positive strategy edge from it.

## INFRA results (2026-11-04, operator-authorized)

Disposable devnet/testnet wallets + operator-provided Infura RPC (mainnet+devnet) validated.
Genesis hashes cross-checked against public clusters (MATCH ×2; mainnet strictly read-only, no
keypair ever combined with it). Live devnet smoke through the private endpoint: `requestAirdrop`
1 SOL confirmed, then signed 0.001 SOL transfer finalized (slot 518298780, fee 5000 lamports,
150 CU) with exact balance deltas and full `getTransaction` receipt. Testnet faucet BLOCKED
(rate-limited, 2 bounded attempts). RPC URLs live only in a 0600 env file outside the repo;
Infura key redacted from all docs. Details: `docs/evidence/INFRA-solana-rpc-wallets.md`.

## INFRA-2 results (2026-11-04, operator-authorized)

Operator RPCs are now integrated in the Rust system: `sniper rpc-probe --cluster
mainnet|devnet|testnet` resolves endpoints (`SNF_{MAINNET,DEVNET,TESTNET}_RPC_URL` env
override → pinned public fallback; invalid env = hard error, never silent fallback),
verifies cluster identity against pinned genesis hashes, and prints a credential-safe JSON
report (`redact_url` strips path/query — Infura keys are unprintable by construction; live
leak grep = 0). Structurally read-only: four getter calls, no signer types, no send path.
Live-verified against both operator endpoints + public testnet. Gates: fmt, clippy ×2,
tests 179/179 ×2 (6 new contract tests incl. offline mock JSON-RPC happy + genesis-mismatch
paths). Faucet status: devnet wallet funded (0.998995 SOL); RPC faucets CLOSED cluster-wide
(HTTP 410, devnet-via-Infura and testnet-public) — testnet funding is operator-manual via
https://faucet.solana.com (address in evidence). `SNF_SOURCE_RPC` for Surfpool forks can now
be fed from `SNF_MAINNET_RPC_URL` (read-only). Details: `docs/evidence/INFRA-2-rpc-integration.md`.

## H5 candidate — Devnet protocol verification and offline research loop (2026-11-04)

Draft PR #6 adds a Devnet-genesis-bound Rust protocol pilot (read-only preflight; explicit transaction test), a separate read-only cluster probe, and bounded synthetic-only research cycles with immutable trial receipts. Full validation is tracked in PR #6. Real Devnet transaction signatures have not been collected by this integration. These demonstrations cannot prove profitability, and mainnet live trading remains disabled. See `docs/DEVNET_EXECUTION_PILOT.md` and `harness/docs/RESEARCH_CYCLE_V1.md`.


## H6+H7 session record (2026-11-04, branch `codex/h6-h7-lifecycle-strategy-20261104`, local only — not pushed)

Executed the agent-executable hand-off PRs (goal plan PR-next-2 and PR-next-3) with the deep-research report's still-open findings folded in (fee truth, independent evaluator, dual replay order, negative controls; its two P0s were already remediated at `204d853`/`b5e266c` and were re-verified by their regression tests at session start).

- H6-1 `8bb39aa`: canonical order (block-sourced transaction_index only, unknown coordinates refuse to sort), provenance (commitment history, fork abandonment), causality (`feature_available <= decision` only), coverage states COMPLETE/INCOMPLETE/CENSORED/UNKNOWN independent of lifecycle, labels physically separated, persistent dedup journal.
- H6-2 `0499b6a` + `92a2b2e`: read-only Mainnet creation-cohort collector; completeness only from getBlock evidence + reconciliation; live attempts classified MAINNET_READ_ONLY_OBSERVED with honest INCOMPLETE/UNKNOWN coverage (public RPC 429s; Infura refuses getBlock for versioned txs). Key handling: operator Infura endpoint used via env only, never committed; owner should rotate the key pasted in chat.
- H7-1 `17410cd`: pure crate `sniper-strategy`; S00–S10; Engine A LegacySurrogateV1 byte-exact vs frozen Python fixture (differential test committed); S00 NO_TRADE dominates all traders on the synthetic cohort.
- H7-2 `cc75b55`: market-cap-tiered curve fees per pinned pump-docs (labeled synthetic fallback when unobserved; retroactive fee states refused), PumpSwap 20+5+creator bps, Engine B on real reserve math with signed reserves, SOL-only routing (others fail closed), latency + failed-landing frictions, full fee attribution in execution records.
- H7-3 `4124839`: independent double-entry oracle (signed balances, per-posting accounting-equation verification, FIFO lots with proportional partial-exit basis, effective-cost inventory, unknown holds excluded from NAV, shared-wallet contention); cross-engine reconstruction test must match engine cash to the lamport.
- H7-4 `77b5a79`: full-portfolio evaluator (CVaR95 with declared 20-trade minimum, profit factor safe on zero denominators, per-phase attribution, outlier dominance, vs S00), the 12 mandated negative controls in one battery, `sniper research` CLI with deterministic write-once receipts (promotable false).

Operator concurrent work preserved: INFRA-3 `0e36fb0` landed on this branch mid-session; further uncommitted rpc_probe edits remain untouched in the working tree.

Next actions: (1) operator RPC tier that serves `getBlock` → rerun collector for a real COMPLETE cohort window (G0); (2) wire Engine B to real snapshots once the cohort exists; (3) H8/PR-next-4 (council sandbox, trial registry, sealed holdout access control, multiple-testing correction); H5-B Devnet round-trip still operator-gated BLOCKED; H3 shadow ≥7 days still NOT_STARTED.


## PF/H8 session record (2026-11-04, branch `codex/profit-first-h8-pnl-goal-20261104`, local only)

PF0 8653d76/ba20ded · PF1 2ed8e8d · G0 aaa881c · H8 e2ab4e9. Fee truth now ON-CHAIN-OBSERVED (95+30 curve; tiered canonical PumpSwap 125→30 bps); oracle reconciles Engine-B cash to the lamport on fee-bearing round trips; PnlRunReceiptV2 bundles deterministic and nonpromotable; G0 collector fixed (root cause: our maxSupportedTransactionVersion=0) with three COMPLETE windows; research registry enforces one-use holdout across processes. All strategies REJECTED on synthetic evidence; S00 NO_TRADE dominates. Next: reserve-snapshot capture for Engine B on real data; longer G0 windows in calmer periods; first registered hypothesis H-0001 execution when a real cohort exists; H3 shadow still NOT_STARTED; no push without owner authorization.


## Wallet Alpha WA1–WA3 session record (2026-11-04, branch `codex/wallet-alpha-upstream-integration-20261104`, local only)

c0270c0 WA1 owner-verified balance-delta normalizer (13-test fixture matrix, dedup, conservation) · 486e20b WA2 `sniper wallet-pnl` via PF1 oracle + roster discovery/ranking with honest unknown-history accounting · 5a54d40 WA3 delayed follower (6 delays, leader-price unreachable, matched S00). Live read-only validation on 12 real mainnet txs (congestion storm: 97% pump-tx failure). All observed leaders lose or are unknown-basis; all follower delay arms lose; S00 dominates everywhere. MIT references (Dorado/Solsonar/million) reimplemented originally. Next: calm-window cohort → W02 copyability-ranked follow vs S00/S02/RX through the H8 preregistration path.


## Wallet Alpha source integration (2026-11-04, same branch)

Deep source-read of Kelows/million (MIT, now cloned under upstream/ pinned c89ad07c) and DracoR22/handi-cat (NO license — temp-clone outside repo, clean-room concepts only). Adopted: million's INFRA_ADDRESSES facts (Raydium AMM v4/CPMM authorities, pump fee recipient, Jupiter referral, quote mints), the find-infra cross-token-top-buyer bot heuristic, operator blocklist; handi-cat's subscription UX reimplemented offline as harness/wallet_watch.py (immutable alert ledger, no network/Telegram). Explicitly rejected-by-design: million's fixed 60-second candle + 6-trade copyability measurement (already superseded by WA3's 500 ms–60 s multi-delay follower). high-win-rate suspicion flag reserved for WA4. Manifest + audit amendment updated.


## Harness² + full-repo integration session (2026-11-05, same branch)

9521aec clones/ADR-0012 · 804e957 REAL harness2 install + demo receipt · 44c87d8 adapter (13-method contract, deterministic recursion 2/2, experience library persisted) · 940b365 contract fixes · b3d27a1 harness2ctl + agentctl + H8 bridge + security + worker prep · 33f3ce6 million owner-graph/consensus/ladder + oracle worked-examples · 4faebcb Dorado differentials + feature register. W01 strategy run executed on REAL fresh capture (calmer window, 270/1000 successful): S00 dominates all delays; H-0001 preregistered before evaluation. Model-backed harness2 BLOCKED behind the prepared isolated worker. All 19 skills lint-pass.


## Documentation architecture + monorepo tidy (2026-11-05)

- `c35e6b3` docs layer: `DOC_INDEX` (single map, status vocabulary, anti-sprawl rules), `GLOSSARY` (full ID system, both evidence ladders), `STRATEGY_REGISTRY` (S00 champion; S01–S10 rejected-synthetic; S11–S14+EX frontier; W01 rejected-real; H-0001 disambiguated — the hash-chained ledger is authoritative), `RPC_PROVIDERS` (G0 critical path, failure catalog incl. the maxSupportedTransactionVersion root cause), `SECURITY.md`, `CONTRIBUTING.md`, ADR index/template (0004–0011 unused; next is 0013), evidence README (receipt contract), PR/issue templates. The earlier ci.yml `ain]` finding was RETRACTED — display artifact; both workflows verified byte-exact (`od -c`).
- The tidy commit (this change): `research.md` → `docs/research/2026-11-02-initial-engineering-design-review.md` (POINT-IN-TIME); `AGENT_MASTER_PROMPT.md` → `docs/history/` (HISTORICAL; all six references updated); archive headers on all 16 `harness/docs/` files; `ARCHITECTURE.md` crate table now includes `sniper-strategy` (existed since H7, was missing) + adjacent-planes note; `AGENTS.md` read order ends at DOC_INDEX and §6 is extended; README pointers added; DOC_INDEX updated. No code moves: `harness/` paths are load-bearing (`agent_mission.py` MASTER_MISSION, CI path filters) and `crates/` is untouched (312/312 ×2 green at HEAD).


## G0 calm-window sweep session 2 (2026-11-05, main)

Owner-authorized read-only continuation of the G0 critical path. Probe `FULL_BLOCK_SAMPLE_AVAILABLE` (3 calls, genesis verified); 4-slot verification window COMPLETE (5 blocks, 0 gaps); 3,000-slot sweep `[465625744, 465628744]` **IN PROGRESS/resumable**: 1,488 blocks verified, 0 gaps, 19,970 in-window pump-program signatures, **0 create instructions** (partial-window observation — ~62 min chain time; no market claim until the window completes). Throughput ≈0.8 blocks/s via operator Infura. Production bug found and fixed: first live budget-stop crashed `main()` with `KeyError: blocks_fetched` — `_budget_stop_report` now carries checkpoint-derived summary keys (both raise-sites pass the checkpoint); regression test added; **117/117 harness tests**, lint-skills PASS. Operational note: `--max-requests` is cumulative across resume legs. Receipt: `docs/evidence/G0-SWEEP-20261105.md`.


## G0 window complete — zero-launch market fact (2026-11-05, main)

First at-scale COMPLETE cohort window: `[465625744, 465628744]`, 3,001/3,001 blocks verified, 0 gaps, 0 create events, 0 mints (manifest `manifest-20261105T090624-3953-*`, write-once). Executed in four flock-guarded checkpointed legs after two engineering fixes this session (budget-stop summary KeyError `268654f`; checkpoint flock after a zombie-sweep dual-writer incident `b90d7e5`; 119/119 harness tests). Sampled 60-block diagnostic (`harness/diag_sample_failed_creates.py`, seed 20261105): **zero `global:create` attempts among 3,450 pump txs (82% failure rate)** — launch activity is absent, not failing; secondary trading remains heavy (~57 pump txs/block). Consequences: S11/S12 blocked by market structure (machinery proven); next probes = repeat captures at launch-active hours, M4 launchpad-coverage decision (LetsBonk/LaunchLab), reserve-snapshot capability built against trading-active tokens. Receipt: `docs/evidence/G0-WINDOW-COMPLETE-20261105.md`.


## D0 launch-attribution audit session (2026-11-05, main)

**Scope:** the goal plan's FIRST NEXT package — audit launch-attribution completeness before D1/M4, offline (RPC env UNSET all session; `SNF_MAINNET_RPC_URL` never printed).

**Completed (labeled):** three statically-confirmed decoder defects fixed red-first (`VERIFIED_CODE`): (1) collector ALT dict-flattening — `list(loadedAddresses)` over the real `{"writable":[…],"readonly":[…]}` wire shape silently skipped every tx with an ALT-resident pump program id before the log scan (`launch_cohort_capture.py:513-515` at `fb1a762`); (2) diagnostic create-family blind spot — attempts counted only for legacy `global:create`; `create_v2` (`d6904cec5f8b31b4`) fell into the generic histogram, and the script was unimportable/untested; (3) log-attribution weakness — markers/payloads gated without program frames; now mirrors Rust `LogAttribution` exactly. Official IDL re-pinned: current `pump-fun/pump-public-docs` IDL is byte-identical to the vendored `639b87f` copy (56 instructions, 0 discriminator mismatches; `create`=`181ec828051c0777`, `create_v2`=`d6904cec5f8b31b4`, `CreateEvent`=`1b72a94ddeeb6376`), and the official Coin Creation docs document `create_v2` as THE current creation instruction — so the unmeasured family is plausibly the dominant one. Tests 119→132 (red: 5 failures + 2 errors demonstrated first; green: 132/132, exit 0); `py_compile` exit 0.

**Engineering notes:** `resolved_account_keys()` canonical order (static+writable+readonly) shared by collector and diag; diag restructured into importable pure core with both create families, outer+inner; collector output schema unchanged (fix strictly widens scan coverage); no Rust changes. Window re-evaluation BLOCKED exactly per charter (checkpoint `ckpt-sweep1.json` absent ∧ RPC unset) — no observation fabricated; the fixed diag re-runs (seed 20261105, N=60) once both preconditions hold.

**What is NOT claimed:** no claim that `create_v2` launches occurred or didn't occur in the window (unmeasured); "zero create attempts" from the prior receipt is downgraded in a NEW file to "zero LEGACY create attempts; `create_v2` unmeasured"; venue-migration (LetsBonk/LaunchLab) remains an unproven hypothesis — no M4 adapter justified by this audit; no market data captured, no strategy evaluated, no P&L (`NOT_APPLICABLE`); S00 still champion, `EDGE_NOT_DEMONSTRATED` everywhere; Rust decoder unchanged; the `wallet_observer.rs:178-225` ALT-shape defect (wallet-alpha venue path, same defect class) is documented and DEFERRED to its own H-stream fix; nothing pushed.

**Next:** D1a time-of-day discrimination at launch-active hours with the create_v2-aware tooling (cheapest probe; one window with real creates unblocks the entire frontier), or D1b venue widening with measured evidence; H-stream follow-up: wallet_observer ALT fix red-first. Receipt: `docs/evidence/D0-LAUNCH-ATTRIBUTION-AUDIT-20261105.md`.
