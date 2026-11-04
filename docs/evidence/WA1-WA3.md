# WA1–WA3 Evidence — Wallet Alpha: Normalization, Leader P&L, Delayed Follower

Date: 2026-11-04. Branch: `codex/wallet-alpha-upstream-integration-20261104`.
Commits: `c0270c0` (WA1), `486e20b` (WA2), `5a54d40` (WA3) — local only, NOT pushed.
`LIVE_TRADING_ENABLED=false`; mainnet strictly read-only (getTransaction/getSignaturesForAddress/getSlot via operator env, never printed); P&L V2 / H8 / G0 assets preserved. MIT DoradoDevs/Solsonar/million used as behavioral references only — every line reimplemented originally.

## WA1 — wallet transaction normalization (`c0270c0`)

`crates/sniper-runtime/src/wallet_observer.rs` + 13 contract tests (red-first: test file written before the module existed).

- `WalletTradeEvent`: signature/slot/blockTime/commitment/wallet_owner/authority_provenance(Signer|AtaOwnerResolved|Unresolved)/mint/venue(PumpFun|PumpSwap|Jupiter|Raydium|Meteora|UnknownDex)/side(Buy|Sell|Transfer|LpFlow|Unknown)/signed i128 deltas/network_fee/coverage/classification.
- Balance-delta normalizer over `getTransaction` receipts; quote = native SOL + WSOL netted, fee payer's `meta.fee` excluded (reported once); multi-hop → ONE net trade; transfers ≠ buys; LP shape → single LpFlow; failed tx → nothing; missing token history → INCOMPLETE + UNKNOWN-mint fail-closed event (zero invented holdings).
- `dedup_events` keyed (signature, slot, wallet, mint); `tx_conservation_invariants` (Σ SOL deltas == −fee; fixtures carry compensating pool accounts).
- Fixture matrix: Pump.fun buy+sell, PumpSwap WSOL-wrap netting, Jupiter 2-hop, Raydium+Meteora, SPL transfer, LP deposit, Token-2022, failed tx, unknown DEX, uninvolved wallet, partial history, replay/dedup/conservation.

**Tests**: `cargo test -p sniper-runtime --test wallet_observer` → 13 passed. Gates: fmt 0, clippy −D warnings (all-features) 0, workspace 299/299 at commit.

## Live read-only smoke (WA1 validation)

Fetched 12 real successful mainnet pump-program transactions (1000-signature page during the ongoing congestion storm: **97% of recent pump signatures failed**; 63/1000 successful) + 22 distinct participating wallets (saved under `harness/runs/wallet-alpha/`).

## WA2 — leader P&L through the PF1 oracle (`486e20b`)

`sniper wallet-pnl --transactions --wallet --out-dir` (WA1 normalize → observed-amount ExecutionRecords → independent double-entry oracle → full PnlRunReceiptV2 bundle) + `harness/wallet_roster.py` (discovery excluding infrastructure accounts/failed txs; bounded genesis-checked history fetch; copyability ranking with mandatory uncertainty, tiers CANDIDATE/REJECTED_LOSS/LOW_CONFIDENCE/UNRANKABLE, loss-beats-uncertainty precedence).

Unknown-history accounting (both found by live data, both fixed test-first):
- unobserved opening **capital** → external deposits (excluded from P&L by `net_pnl`);
- unobserved opening **inventory** → funded from ExternalDeposits at basis == proceeds (zero realized P&L by construction — an earlier Equity-funded variant wrongly counted a real wallet's sell proceeds as profit).

**Real observed results** (12-tx window, congestion period; receipts `harness/runs/wallet-alpha/wa2_wallet_*/run.json`):

| Wallet | Records | Net P&L (lamports) | Unknown-history adjustments |
|---|---|---|---|
| `4ityA4Kq…` | 3 buys/sells | **−4,961,316,798** (real loss) | 4.96 SOL unobserved capital |
| `5iQoSoDu…` | 3 sells (pre-window tokens) | **exactly 0** (unknown basis) | 3 opening-inventory adjustments |
| `Gygj9QQb…` | 0 | 0 | — |
| `BwWK17cb…` | 0 (2 transfers/LP skipped) | 0 | — |

Classification `MAINNET_READ_ONLY_OBSERVED`, `nonpromotable`. Window too small to rank any wallet CANDIDATE — honest.

## WA3 — delayed follower paper trading (`5a54d40`)

`crates/sniper-strategy/src/follower.rs` (6 unit tests) + `sniper wallet-follow` CLI (integration test). Entry priced ONLY at/after observed+delay via `DelayPricer` (leader price structurally unreachable); exits at next quote ≥ entry+hold; 125 bps round-trip cost = the PF0-observed curve fee; refusals counted by reason; duplicates never double-enter; unquotable exits open at zero.

**Demo run** (4 synthetic signals, 3 followed, decaying price timeline, receipt `615a43db…`):

| Delay | Entries | Net P&L (lamports) |
|---|---|---|
| 500 ms | 3 | −1,448,179 |
| 1 s | 3 | −1,448,179 |
| 3 s | 3 | −1,351,728 |
| 10 s | 3 | −1,182,045 |
| 30 s | 3 | −3,030,000 |
| 60 s | 3 | −3,030,000 |
| **S00** | 0 | **0** |

Every delay arm loses; **S00 dominates**; verdict `EDGE_NOT_DEMONSTRATED`, nonpromotable.

## Gates at session close

fmt 0; clippy `--workspace --all-targets --all-features -D warnings` 0; `cargo test --workspace` **306/306** (both feature sets); `python3 -m unittest discover -s harness/tests -q` **114 OK**; cargo-deny ok; secret/license scans clean.

## Coverage, uncertainty, blockers

- Coverage: WA1 fixtures = full mandated matrix; live validation = 12 real txs (storm-limited). WA2 roster = 4 wallets × 12-tx window — far below ranking-grade history.
- Uncertainty: leader P&L windows are partial (unobserved openings disclosed on every receipt); follower results are quote-timeline synthetic.
- Blockers: meaningful roster needs a calmer market window (or wider bounded sweep) — 97% tx failure during captures; real follower evaluation needs as-of reserve snapshots per signal (WA4+); WA0 lab, H8 registry and holdout remain the governance path for any future candidate.
- Next falsifiable step (pre-registered intent): capture ≥1 hour of successful pump trades in a calm window → rank top-coverage wallets → run W02 (copyability-ranked follow) vs S00/S02/RX through WA3 at all six delays.
