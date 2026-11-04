//! Versioned, source-proven protocol fee model (H7-2).
//!
//! Sources (pinned in `upstream/pump-docs`):
//! - `docs/FEE_PROGRAM_README.md` — since 2026-09-01 the bonding curve and
//!   PumpSwap CANONICAL pools price fees from a dynamic, market-cap-tiered
//!   `feeConfig`; when `feeConfig == null` the global fields apply. Market
//!   cap: `virtualSolReserves * mintSupply / virtualTokenReserves` (curve)
//!   and `quoteReserves * baseMintSupply / baseReserve` (pool). Tier pick:
//!   highest tier whose threshold `<=` market cap, else the first tier.
//! - `docs/PUMP_SWAP_README.md` — lp fee 20 bps + protocol fee 5 bps on
//!   buy/sell; custom-pair pools may carry a pool-level `creator_fee_bps`.
//! - `docs/PUMP_SWAP_CREATOR_FEE_README.md` — canonical pools also route
//!   `GlobalConfig::coin_creator_fee_basis_points` (documented value 0 at
//!   the pin) to the coin-creator vault.
//! - `docs/PUMP_CASHBACK_README.md` — cashback coins are deprecated for new
//!   creates; legacy coins keep creator-fee-as-cashback routing.
//!
//! The NUMERIC tier table lives on chain (fee config accounts), not in the
//! docs: a real batch must supply observed tiers. A missing configuration
//! resolves to an explicitly-labeled synthetic fallback — never silently to
//! "100 bps everywhere".

use serde::{Deserialize, Serialize};

/// Which venue's fee schedule applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VenueKind {
    BondingCurve,
    /// Pool created by the pump.fun `migrate` instruction
    /// (`pool.creator == pump pool-authority PDA`).
    PumpSwapCanonical,
    /// Custom-pair pool (non-SOL/USDC quote or third-party created).
    PumpSwapCustomPair,
}

/// Provenance of one fee state: where the numbers came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeProvenance {
    pub venue: VenueKind,
    /// Executable program revision the schedule applies to.
    pub program_version: u32,
    pub source: FeeSourceKind,
    /// Slot at which the configuration was observed.
    pub observed_slot: u64,
    /// Pinned source document identity.
    pub source_doc: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeeSourceKind {
    /// Read from the on-chain fee config account.
    FeeConfigAccount,
    /// `feeConfig == null` → program global fields.
    GlobalConfig,
    /// Labeled assumption for tests; MUST NOT back real-batch receipts.
    SyntheticAssumption,
}

impl FeeSourceKind {
    pub fn is_synthetic(self) -> bool {
        matches!(self, FeeSourceKind::SyntheticAssumption)
    }
}

pub const FEE_PROGRAM_DOC: &str = "upstream/pump-docs/docs/FEE_PROGRAM_README.md";
pub const PUMP_SWAP_DOC: &str = "upstream/pump-docs/docs/PUMP_SWAP_README.md";
pub const PUMP_SWAP_CREATOR_FEE_DOC: &str =
    "upstream/pump-docs/docs/PUMP_SWAP_CREATOR_FEE_README.md";

/// One market-cap tier: at or above `market_cap_lamports_threshold` these
/// bps apply (protocol + creator components are separate on-chain).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeTier {
    pub market_cap_lamports_threshold: u128,
    pub protocol_bps: u64,
    pub creator_bps: u64,
}

/// Fee state for the bonding curve (tiered or global fallback).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurveFeeState {
    pub provenance: FeeProvenance,
    /// Observed fee-config tiers, ascending by threshold. Empty = fallback.
    pub tiers: Vec<FeeTier>,
    /// `feeConfig == null` fallback (program global fields).
    pub fallback_protocol_bps: u64,
    pub fallback_creator_bps: u64,
}

/// Resolved fee components for one trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedFees {
    /// LP fee (PumpSwap only; always 0 on the bonding curve).
    pub lp_fee_bps: u64,
    pub protocol_bps: u64,
    pub creator_bps: u64,
    pub synthetic_fallback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("fee state observed at slot {observed_slot} postdates decision slot {decision_slot}; retroactive pricing forbidden")]
pub struct RetroactiveFeeState {
    pub observed_slot: u64,
    pub decision_slot: u64,
}

impl FeeProvenance {
    /// A fee state observed after the decision slot would price history
    /// with future configuration — refused.
    pub fn ensure_not_retroactive(&self, decision_slot: u64) -> Result<(), RetroactiveFeeState> {
        if self.observed_slot > decision_slot {
            return Err(RetroactiveFeeState {
                observed_slot: self.observed_slot,
                decision_slot,
            });
        }
        Ok(())
    }
}

impl CurveFeeState {
    /// Labeled synthetic fallback (tests/illustration only): the
    /// pre-fee-program single-rate assumption. Real batches must supply
    /// observed fee-config/global values.
    pub fn synthetic_fallback_100bps(observed_slot: u64) -> Self {
        Self {
            provenance: FeeProvenance {
                venue: VenueKind::BondingCurve,
                program_version: 1,
                source: FeeSourceKind::SyntheticAssumption,
                observed_slot,
                source_doc: FEE_PROGRAM_DOC.to_string(),
            },
            tiers: Vec::new(),
            fallback_protocol_bps: 100,
            fallback_creator_bps: 0,
        }
    }

    /// A fee state observed after the decision slot would price history
    /// with future configuration — refused.
    pub fn ensure_not_retroactive(&self, decision_slot: u64) -> Result<(), RetroactiveFeeState> {
        self.provenance.ensure_not_retroactive(decision_slot)
    }

    /// Resolve the fee for a market cap, per the documented tier algorithm.
    pub fn resolve(&self, market_cap_lamports: u128) -> ResolvedFees {
        if self.tiers.is_empty() {
            return ResolvedFees {
                lp_fee_bps: 0,
                protocol_bps: self.fallback_protocol_bps,
                creator_bps: self.fallback_creator_bps,
                synthetic_fallback: self.provenance.source.is_synthetic(),
            };
        }
        // Highest tier whose threshold <= market cap; below every threshold
        // the FIRST tier applies.
        let tier = self
            .tiers
            .iter()
            .rev()
            .find(|t| market_cap_lamports >= t.market_cap_lamports_threshold)
            .unwrap_or(&self.tiers[0]);
        ResolvedFees {
            lp_fee_bps: 0,
            protocol_bps: tier.protocol_bps,
            creator_bps: tier.creator_bps,
            synthetic_fallback: false,
        }
    }

    /// Build from an OBSERVED curve FeeConfig account (PF0: the on-chain
    /// truth read by `harness/fee_config_probe.py` — e.g. 95 protocol +
    /// 30 creator = 125 bps at slot ~465.3M). Source is FeeConfigAccount,
    /// so receipts carry observed provenance, never a synthetic label.
    pub fn from_observed_fee_config(
        protocol_bps: u64,
        creator_bps: u64,
        observed_slot: u64,
    ) -> Self {
        Self {
            provenance: FeeProvenance {
                venue: VenueKind::BondingCurve,
                program_version: 2,
                source: FeeSourceKind::FeeConfigAccount,
                observed_slot,
                source_doc: FEE_PROGRAM_DOC.to_string(),
            },
            tiers: vec![FeeTier {
                market_cap_lamports_threshold: 0,
                protocol_bps,
                creator_bps,
            }],
            fallback_protocol_bps: protocol_bps,
            fallback_creator_bps: creator_bps,
        }
    }
}

/// One OBSERVED canonical-pool market-cap tier (three fee components, per
/// the AMM FeeConfig account layout `Fees {lp, protocol, creator}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PumpSwapTier {
    pub market_cap_lamports_threshold: u128,
    pub lp_fee_bps: u64,
    pub protocol_fee_bps: u64,
    pub creator_fee_bps: u64,
}

/// PumpSwap fee state (canonical or custom pair).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PumpSwapFeeState {
    pub provenance: FeeProvenance,
    /// AMM lp fee (flat / non-canonical fallback; pinned docs: 20 bps).
    pub lp_bps: u64,
    /// AMM protocol fee (flat / non-canonical fallback; pinned docs: 5 bps).
    pub protocol_bps: u64,
    /// Coin-creator fee on canonical pools (GlobalConfig value; 0 at pin).
    pub coin_creator_bps: u64,
    /// OBSERVED canonical market-cap tier schedule (AMM FeeConfig). When
    /// present, canonical pools price from tiers — the flat fields above
    /// are then only the feeConfig==null / non-canonical fallback.
    #[serde(default)]
    pub canonical_tiers: Option<Vec<PumpSwapTier>>,
    /// True ONLY for `canonical_reference()` — a test fixture that MUST NOT
    /// back real-batch P&L (PF0 CE-2 guard).
    #[serde(default)]
    pub reference_fixture: bool,
    /// Custom-pair pool override (`pool.creator_fee_bps`; None on canonical).
    pub pool_creator_fee_override_bps: Option<u64>,
    /// Legacy cashback coin: creator fee routed to the buyer instead.
    pub cashback_coin: bool,
}

impl PumpSwapFeeState {
    /// Canonical-pool reference state from the pinned docs — **TEST FIXTURE
    /// ONLY** (PF0 CE-2): the observed canonical schedule is market-cap
    /// TIERED (125 bps at low caps; the 20+5 flat shape only at high caps)
    /// and must come from `from_observed_fee_config`. `is_reference_fixture`
    /// lets real-batch paths fail closed on this constructor.
    pub fn canonical_reference(observed_slot: u64) -> Self {
        Self {
            provenance: FeeProvenance {
                venue: VenueKind::PumpSwapCanonical,
                program_version: 2,
                source: FeeSourceKind::GlobalConfig,
                observed_slot,
                source_doc: PUMP_SWAP_DOC.to_string(),
            },
            lp_bps: 20,
            protocol_bps: 5,
            coin_creator_bps: 0,
            canonical_tiers: None,
            reference_fixture: true,
            pool_creator_fee_override_bps: None,
            cashback_coin: false,
        }
    }

    /// True when this state is the 25 bps doc reference rather than an
    /// observed schedule. Real-batch P&L paths MUST refuse it (fail closed).
    pub fn is_reference_fixture(&self) -> bool {
        self.reference_fixture && self.canonical_tiers.is_none()
    }

    /// Build from OBSERVED AMM FeeConfig data (PF0): flat fees for
    /// non-canonical pools + the market-cap tier schedule for canonical
    /// pools, as decoded by `harness/fee_config_probe.py`.
    pub fn from_observed_fee_config(
        flat_lp_bps: u64,
        flat_protocol_bps: u64,
        flat_creator_bps: u64,
        tiers: Vec<PumpSwapTier>,
        observed_slot: u64,
    ) -> Self {
        Self {
            provenance: FeeProvenance {
                venue: VenueKind::PumpSwapCanonical,
                program_version: 2,
                source: FeeSourceKind::FeeConfigAccount,
                observed_slot,
                source_doc: FEE_PROGRAM_DOC.to_string(),
            },
            lp_bps: flat_lp_bps,
            protocol_bps: flat_protocol_bps,
            coin_creator_bps: flat_creator_bps,
            canonical_tiers: Some(tiers),
            reference_fixture: false,
            pool_creator_fee_override_bps: None,
            cashback_coin: false,
        }
    }

    /// Resolve canonical-pool fee components for a market cap (documented
    /// tier algorithm: highest tier whose threshold <= cap, else first).
    /// Falls back to the flat fields when no observed schedule exists.
    pub fn resolve(&self, market_cap_lamports: u128) -> ResolvedFees {
        if let Some(tiers) = &self.canonical_tiers {
            let tier = tiers
                .iter()
                .rev()
                .find(|t| market_cap_lamports >= t.market_cap_lamports_threshold)
                .unwrap_or(&tiers[0]);
            return ResolvedFees {
                lp_fee_bps: tier.lp_fee_bps,
                protocol_bps: tier.protocol_fee_bps,
                creator_bps: tier.creator_fee_bps,
                synthetic_fallback: false,
            };
        }
        ResolvedFees {
            lp_fee_bps: self.lp_bps,
            protocol_bps: self.protocol_bps,
            creator_bps: self.coin_creator_bps,
            synthetic_fallback: self.is_reference_fixture(),
        }
    }

    /// Total bps withheld on a swap (excluding network costs). Flat
    /// interpretation — prefer `resolve()` for canonical pools.
    pub fn total_bps(&self) -> u64 {
        let pool_creator = self.pool_creator_fee_override_bps.unwrap_or(0);
        self.lp_bps + self.protocol_bps + self.coin_creator_bps + pool_creator
    }

    /// Total bps for a canonical pool at a market cap (observed tiers).
    pub fn total_bps_at(&self, market_cap_lamports: u128) -> u64 {
        let r = self.resolve(market_cap_lamports);
        r.lp_fee_bps + r.protocol_bps + r.creator_bps
    }
}

/// Curve market cap in lamports: `vSol * supply / vTok` (docs formula).
/// Zero token reserves make the cap undefined.
pub fn bonding_curve_market_cap(
    mint_supply: u128,
    virtual_sol_reserves: u128,
    virtual_token_reserves: u128,
) -> Option<u128> {
    if virtual_token_reserves == 0 {
        return None;
    }
    Some(virtual_sol_reserves * mint_supply / virtual_token_reserves)
}

/// PumpSwap pool market cap in lamports: `quoteReserve * supply / baseReserve`.
pub fn pool_market_cap(
    base_mint_supply: u128,
    base_reserve: u128,
    quote_reserve: u128,
) -> Option<u128> {
    if base_reserve == 0 {
        return None;
    }
    Some(quote_reserve * base_mint_supply / base_reserve)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiered_state() -> CurveFeeState {
        CurveFeeState {
            provenance: FeeProvenance {
                venue: VenueKind::BondingCurve,
                program_version: 1,
                source: FeeSourceKind::FeeConfigAccount,
                observed_slot: 100,
                source_doc: FEE_PROGRAM_DOC.to_string(),
            },
            tiers: vec![
                FeeTier {
                    market_cap_lamports_threshold: 0,
                    protocol_bps: 50,
                    creator_bps: 50,
                },
                FeeTier {
                    market_cap_lamports_threshold: 50_000_000_000,
                    protocol_bps: 40,
                    creator_bps: 10,
                },
                FeeTier {
                    market_cap_lamports_threshold: 500_000_000_000,
                    protocol_bps: 25,
                    creator_bps: 0,
                },
            ],
            fallback_protocol_bps: 100,
            fallback_creator_bps: 0,
        }
    }

    #[test]
    fn tier_boundaries_resolve_per_docs_algorithm() {
        let state = tiered_state();
        // Below every threshold -> first tier.
        let low = state.resolve(1_000);
        assert_eq!(
            (low.lp_fee_bps, low.protocol_bps, low.creator_bps),
            (0, 50, 50)
        );
        // Exactly at a threshold -> that tier (>=).
        let mid = state.resolve(50_000_000_000);
        assert_eq!(
            (mid.lp_fee_bps, mid.protocol_bps, mid.creator_bps),
            (0, 40, 10)
        );
        // One lamport below -> previous tier.
        let below = state.resolve(50_000_000_000 - 1);
        assert_eq!(
            (below.lp_fee_bps, below.protocol_bps, below.creator_bps),
            (0, 50, 50)
        );
        // Top tier.
        let top = state.resolve(10_000_000_000_000);
        assert_eq!(
            (top.lp_fee_bps, top.protocol_bps, top.creator_bps),
            (0, 25, 0)
        );
        assert!(!top.synthetic_fallback);
    }

    #[test]
    fn missing_config_falls_back_explicitly_labeled() {
        let state = CurveFeeState::synthetic_fallback_100bps(100);
        let fees = state.resolve(123_456);
        assert_eq!(fees.protocol_bps, 100);
        assert!(
            fees.synthetic_fallback,
            "fallback must be labeled synthetic"
        );
    }

    #[test]
    fn fee_state_after_decision_slot_is_retroactive_and_refused() {
        let state = tiered_state();
        assert!(state.ensure_not_retroactive(200).is_ok());
        let future = CurveFeeState {
            provenance: FeeProvenance {
                observed_slot: 300,
                ..state.provenance.clone()
            },
            ..state
        };
        assert!(future.ensure_not_retroactive(200).is_err());
    }

    #[test]
    fn market_cap_formulas_match_docs() {
        // vSol 30 SOL, supply 1e15, vTok 1.073e15 -> ~27.96 SOL cap.
        let cap =
            bonding_curve_market_cap(1_000_000_000_000_000, 30_000_000_000, 1_073_000_000_000_000)
                .unwrap();
        assert_eq!(
            cap,
            30_000_000_000u128 * 1_000_000_000_000_000 / 1_073_000_000_000_000
        );
        assert_eq!(bonding_curve_market_cap(1, 1, 0), None);
        assert!(pool_market_cap(1_000, 10, 500).is_some());
        assert_eq!(pool_market_cap(1_000, 0, 500), None);
    }

    #[test]
    fn pumpswap_reference_total_is_documented_25_bps() {
        let state = PumpSwapFeeState::canonical_reference(1);
        assert_eq!(state.lp_bps, 20);
        assert_eq!(state.protocol_bps, 5);
        assert_eq!(state.total_bps(), 25);
        // PF0 CE-2: the reference is a fixture and must say so.
        assert!(state.is_reference_fixture());
    }

    /// Values OBSERVED on-chain by harness/fee_config_probe.py at slots
    /// 465329418..465329431 (receipt harness/runs/fee-config/): curve flat
    /// = 95 protocol + 30 creator (125 bps); AMM canonical tier 0 = lp 2 +
    /// protocol 93 + creator 30 (125 bps); >=420 SOL cap = 20+5+95.
    fn observed_amm_tiers() -> Vec<PumpSwapTier> {
        vec![
            PumpSwapTier {
                market_cap_lamports_threshold: 0,
                lp_fee_bps: 2,
                protocol_fee_bps: 93,
                creator_fee_bps: 30,
            },
            PumpSwapTier {
                market_cap_lamports_threshold: 420_000_000_000,
                lp_fee_bps: 20,
                protocol_fee_bps: 5,
                creator_fee_bps: 95,
            },
            PumpSwapTier {
                market_cap_lamports_threshold: 49_120_000_000_000,
                lp_fee_bps: 20,
                protocol_fee_bps: 5,
                creator_fee_bps: 5,
            },
        ]
    }

    #[test]
    fn curve_from_observed_matches_onchain_125bps() {
        let state = CurveFeeState::from_observed_fee_config(95, 30, 465_329_418);
        let fees = state.resolve(0);
        assert_eq!(fees.lp_fee_bps, 0);
        assert_eq!(fees.protocol_bps, 95);
        assert_eq!(fees.creator_bps, 30);
        assert!(!fees.synthetic_fallback);
        assert_eq!(state.provenance.source, FeeSourceKind::FeeConfigAccount);
    }

    #[test]
    fn canonical_low_cap_pays_125bps_not_25() {
        let state =
            PumpSwapFeeState::from_observed_fee_config(25, 5, 0, observed_amm_tiers(), 465_329_418);
        assert!(!state.is_reference_fixture());
        // Fresh canonical pool (tiny cap): 2+93+30 = 125 bps — 5x the
        // 25 bps reference. Underpricing here was PF0 CE-2.
        assert_eq!(state.total_bps_at(1_000), 125);
        assert_eq!(state.total_bps_at(419_999_999_999), 125);
    }

    #[test]
    fn canonical_tier_boundaries_follow_observed_schedule() {
        let state =
            PumpSwapFeeState::from_observed_fee_config(25, 5, 0, observed_amm_tiers(), 465_329_418);
        // Exactly at the 420 SOL threshold -> next tier (20+5+95=120).
        assert_eq!(state.total_bps_at(420_000_000_000), 120);
        // Top observed tier: 20+5+5 = 30.
        assert_eq!(state.total_bps_at(60_000_000_000_000), 30);
        // Component split resolves per tier.
        let low = state.resolve(0);
        assert_eq!(
            (low.lp_fee_bps, low.protocol_bps, low.creator_bps),
            (2, 93, 30)
        );
        let mid = state.resolve(420_000_000_000);
        assert_eq!(
            (mid.lp_fee_bps, mid.protocol_bps, mid.creator_bps),
            (20, 5, 95)
        );
    }

    #[test]
    fn reference_fixture_resolution_is_labeled_synthetic() {
        let state = PumpSwapFeeState::canonical_reference(1);
        let fees = state.resolve(123);
        assert!(
            fees.synthetic_fallback,
            "fixture resolution must be labeled so real receipts can refuse it"
        );
    }
}
