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
            protocol_bps: tier.protocol_bps,
            creator_bps: tier.creator_bps,
            synthetic_fallback: false,
        }
    }
}

/// PumpSwap fee state (canonical or custom pair).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PumpSwapFeeState {
    pub provenance: FeeProvenance,
    /// AMM lp fee (pinned docs: 20 bps).
    pub lp_bps: u64,
    /// AMM protocol fee (pinned docs: 5 bps).
    pub protocol_bps: u64,
    /// Coin-creator fee on canonical pools (GlobalConfig value; 0 at pin).
    pub coin_creator_bps: u64,
    /// Custom-pair pool override (`pool.creator_fee_bps`; None on canonical).
    pub pool_creator_fee_override_bps: Option<u64>,
    /// Legacy cashback coin: creator fee routed to the buyer instead.
    pub cashback_coin: bool,
}

impl PumpSwapFeeState {
    /// Canonical-pool reference state from the pinned docs (creator bps 0
    /// at the documented pin; supply observed values for real batches).
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
            pool_creator_fee_override_bps: None,
            cashback_coin: false,
        }
    }

    /// Total bps withheld on a swap (excluding network costs).
    pub fn total_bps(&self) -> u64 {
        let pool_creator = self.pool_creator_fee_override_bps.unwrap_or(0);
        self.lp_bps + self.protocol_bps + self.coin_creator_bps + pool_creator
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
        assert_eq!((low.protocol_bps, low.creator_bps), (50, 50));
        // Exactly at a threshold -> that tier (>=).
        let mid = state.resolve(50_000_000_000);
        assert_eq!((mid.protocol_bps, mid.creator_bps), (40, 10));
        // One lamport below -> previous tier.
        let below = state.resolve(50_000_000_000 - 1);
        assert_eq!((below.protocol_bps, below.creator_bps), (50, 50));
        // Top tier.
        let top = state.resolve(10_000_000_000_000);
        assert_eq!((top.protocol_bps, top.creator_bps), (25, 0));
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
    }
}
