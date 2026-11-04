# PF0 Evidence — P&L Audit and On-Chain Fee Provenance

Date: 2026-11-04. Branch: `codex/profit-first-h8-pnl-goal-20261104`.
Commits: `8653d76` (audit doc), `ba20ded` (probe + observed constructors + tier-resolved engine fees).
`LIVE_TRADING_ENABLED=false`; mainnet strictly read-only (getAccountInfo/getSlot only); operator RPC from `~/.config/solana/sniperfusion/rpc.env`, credential never printed, never committed.

## Deliverables

1. **`docs/PF0_PNL_AUDIT.md`** — frozen unit invariants (lamports; SOL-denominated ledger; marks never post; fees counted exactly once; per-posting conservation; matched comparison contract), the PnlRunReceiptV2 contract, explicit blind spots, and the five verified counterexamples (CE-1 oracle sell-fee double-count; CE-2 25 bps canonical fee reference vs tiered on-chain feeConfig; CE-3 deposits unrepresentable; CE-4 shared-wallet attribution impossible; CE-5 waterfall component gaps).
2. **`harness/fee_config_probe.py`** (+10 offline tests) — read-only borsh decoder for the four pinned fee accounts with anchor-discriminator checks, prefix-tolerant trailing fields, slot-stamped provenance, write-once 0444 receipts.
3. **`fees.rs`** — `CurveFeeState::from_observed_fee_config`, `PumpSwapTier` schedule, `PumpSwapFeeState::from_observed_fee_config / resolve(market_cap) / total_bps_at`; `canonical_reference()` now carries `is_reference_fixture()` and its resolution is labeled synthetic so real-batch paths can fail closed. Engine B resolves ALL PumpSwap fees by the pool's market cap and attributes exact withheld-fee splits.

## The CE-2 discrepancy: RESOLVED with on-chain truth

Receipt `harness/runs/fee-config/fee-config-20261104T140641-10620.json` (slots 465329418..465329431, evidence class `MAINNET_READ_ONLY_OBSERVED`, nonpromotable):

| Account | Observed |
|---|---|
| curve FeeConfig `8Wf5Ti…` | flat = **protocol 95 + creator 30 = 125 bps total** (single tier at cap ≥ 0); stable/exotic identical |
| AMM FeeConfig `5PHirr…` | flat = lp 25 + protocol 5 + creator 0 (**30 bps**, non-canonical pools); **25 canonical tiers**: cap<420 SOL → lp 2 + protocol 93 + creator 30 = **125 bps**; at ≥420 SOL → 20+5 with creator decreasing 95→5 bps (120→30 bps); ≥98,240 SOL → 30 bps |
| pump Global `4wTV1Y…` | fallback fee_basis_points **95**, creator_fee_basis_points **5**; pool_migration_fee 15,000,001 lamports |
| AMM GlobalConfig `ADyA8h…` | UNREADABLE on this provider (`Invalid param: WrongSize` in base64/jsonParsed/base64+zstd) — recorded honestly; it is only the `feeConfig == null` fallback |

**Conclusion**: the code's 25 bps `canonical_reference` underprices a fresh canonical pool by **5×** (125 bps actual) and the curve synthetic fallback (100/0) misstates both total (125) and split (95/30). All prior Engine-B synthetic numbers priced with the 100 bps fallback were diagnostic only; from `ba20ded` the observed constructors exist and new tests pin the observed values (boundary at 420 SOL, top tier 30 bps). Pinned pump-docs could not have shown this: shallow clone @ `639b87f`, tier table shipped as a PNG.

## Gates

fmt clean; clippy `-D warnings` (all-features) clean; workspace tests 270→ with PF1 285 (both feature sets); harness 79→103 with later missions. Exact commands and counts in PF1/H8 evidence.
