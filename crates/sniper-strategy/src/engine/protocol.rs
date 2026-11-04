//! Engine B — `ProtocolEconomicV1`.
//!
//! Venue-aware execution economics on ACTUAL reserve math (constant product
//! over virtual/effective reserves via `sniper-protocols::math`, the same
//! math the protocol builders and fork tests use) with source-proven,
//! slot-versioned fee configurations. Deliberately does NOT reproduce
//! `LegacySurrogateV1` numbers: linear price approximation is replaced by
//! reserve pricing, and impact emerges from the math instead of a bps add-on.
//!
//! Fail-closed rules:
//! - SOL-paired routes only; USDC/other quote mints are explicitly rejected.
//! - Missing/stale/retroactive market state → rejected attempt, never an
//!   optimistic fill.
//! - Unsupported venue configurations cannot silently execute.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::fees::{CurveFeeState, PumpSwapFeeState, ResolvedFees};
use crate::observation::Observation;
use crate::{Intent, PortfolioView, RiskConstraints, Strategy, StrategyId};

use sniper_protocols::math::{
    amm_sell_out, apply_fee_in, curve_buy_out, curve_sell_out, effective_quote_reserves,
};

/// Quote asset of a route. Only `Sol` is executable in this engine version;
/// anything else fails closed with an explicit reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuoteMint {
    Sol,
    Usdc,
    Unsupported(String),
}

/// Venue state for one mint at one slot, as observed on chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VenueState {
    BondingCurve {
        virtual_token_reserves: u64,
        virtual_quote_reserves: u64,
        mint_total_supply: u128,
        fee: CurveFeeState,
    },
    PumpSwap {
        base_reserves: u64,
        pool_quote_vault_amount: u64,
        /// Signed since 2026-09-30 (NEGATIVE_VIRTUAL_QUOTE_RESERVES.md).
        virtual_quote_reserves: i128,
        base_mint_supply: u128,
        fee: PumpSwapFeeState,
        canonical: bool,
    },
}

/// A point-in-time market snapshot with its own fee provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSnapshot {
    pub mint: String,
    pub slot: u64,
    pub quote_mint: QuoteMint,
    pub state: VenueState,
}

/// Market state lookup: latest snapshot at or before a slot.
#[derive(Debug, Clone, Default)]
pub struct MarketBook {
    by_mint: HashMap<String, Vec<MarketSnapshot>>,
}

impl MarketBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, snapshot: MarketSnapshot) {
        let entries = self.by_mint.entry(snapshot.mint.clone()).or_default();
        entries.push(snapshot);
        entries.sort_by_key(|s| s.slot);
    }

    /// Latest snapshot for `mint` with slot <= `slot` (a decision may only
    /// use state that already existed).
    pub fn latest_at(&self, mint: &str, slot: u64) -> Option<&MarketSnapshot> {
        self.by_mint
            .get(mint)?
            .iter()
            .rev()
            .find(|s| s.slot <= slot)
    }
}

/// Deterministic execution frictions (scenario input, never hidden state).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionFrictions {
    /// Adverse price move applied to the fill (bps off the math output).
    pub slippage_bps: u64,
    /// Network signature fee per attempt (paid on landing AND failure).
    pub base_network_fee_lamports: u64,
    /// Priority fee per attempt (paid on landing AND failure).
    pub priority_fee_lamports: u64,
    /// Tip on successful landing only.
    pub tip_lamports: u64,
    /// Execution lands against the snapshot this many slots after decision.
    pub latency_slots: u64,
    /// Deterministic fraction (bps) of attempts that never land.
    pub failed_landing_bps: u64,
}

impl Default for ExecutionFrictions {
    fn default() -> Self {
        Self {
            slippage_bps: 100,
            base_network_fee_lamports: 5_000,
            priority_fee_lamports: 5_000,
            tip_lamports: 0,
            latency_slots: 1,
            failed_landing_bps: 0,
        }
    }
}

/// One economic event in the execution ledger (input for the independent
/// accounting oracle; carries full fee attribution).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionRecord {
    BuyFill {
        event_id: String,
        mint: String,
        slot: u64,
        spend_lamports: u64,
        tokens_out: u64,
        protocol_fee_lamports: u64,
        creator_fee_lamports: u64,
        lp_fee_lamports: u64,
        network_fee_lamports: u64,
        priority_fee_lamports: u64,
        tip_lamports: u64,
        rent_lamports: u64,
        fee_synthetic_fallback: bool,
    },
    SellFill {
        event_id: String,
        mint: String,
        slot: u64,
        tokens_in: u64,
        quote_out_net: u64,
        protocol_fee_lamports: u64,
        creator_fee_lamports: u64,
        lp_fee_lamports: u64,
        network_fee_lamports: u64,
        priority_fee_lamports: u64,
        tip_lamports: u64,
        rent_recovered_lamports: u64,
        fee_synthetic_fallback: bool,
    },
    /// Entry attempt refused before any cash moved.
    AttemptRejected {
        event_id: String,
        mint: String,
        reason: RejectionReason,
    },
    /// Attempt landed nothing but still burned network + priority fees.
    AttemptFailedLanding {
        event_id: String,
        mint: String,
        network_fee_lamports: u64,
        priority_fee_lamports: u64,
    },
    /// Exit due but not executable (missing state, negative effective
    /// reserves, unsupported route). The position stays open as a liability.
    ExitBlocked {
        event_id: String,
        mint: String,
        reason: RejectionReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejectionReason {
    UnsupportedQuoteRoute,
    MissingMarketState,
    RetroactiveFeeState,
    InsufficientCash,
    EmptyOutput,
    InvalidReserves,
}

/// FNV-1a hash for deterministic failed-landing assignment.
fn attempt_hash(strategy: StrategyId, event_id: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in strategy.as_str().bytes().chain(event_id.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

/// One open whole-position lot (engine B tracks one lot per mint).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenLot {
    tokens: u64,
    entry_at_ms: u64,
}

/// Engine B run output: execution ledger + terminal portfolio facts. NAV
/// reconstruction is the oracle's job (H7-3), never the engine's own.
#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolRunOutcome {
    pub strategy: &'static str,
    pub records: Vec<ExecutionRecord>,
    pub ending_cash_lamports: u64,
    pub open_lots: HashMap<String, OpenLot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolParams {
    /// ATA rent charged once per mint; recorded as recoverable on exit.
    pub ata_rent_lamports: u64,
}

impl Default for ProtocolParams {
    fn default() -> Self {
        Self {
            ata_rent_lamports: 1_500_000,
        }
    }
}

/// Run one strategy under protocol economics.
///
/// Entry pricing: fees on the way in (protocol + creator from the resolved
/// tier), remainder priced on the curve, adverse slippage applied to the
/// token output. Exit pricing: whole-position sell against the same-venue
/// math with fees on the way out. Latency shifts execution to the snapshot
/// at `slot + latency_slots` — decisions use observation-time knowledge,
/// fills pay latency-state prices.
pub fn run(
    events: &[Observation],
    book: &MarketBook,
    strategy: &mut dyn Strategy,
    cash: u64,
    risk: &RiskConstraints,
    frictions: &ExecutionFrictions,
    params: &ProtocolParams,
) -> ProtocolRunOutcome {
    let mut balance = cash;
    let mut lots: HashMap<String, OpenLot> = HashMap::new();
    let mut rent_paid: HashMap<String, u64> = HashMap::new();
    let mut records: Vec<ExecutionRecord> = Vec::new();
    let exit_policy = crate::exit::ExitPolicy::default();

    for event in events {
        let mint = event.mint.clone();
        let held = lots.contains_key(&mint);
        if held {
            let lot = lots.get(&mint).expect("held");
            if exit_policy.due(lot.entry_at_ms, event) {
                match sell(
                    event, book, &mint, lot.tokens, frictions, &rent_paid, params,
                ) {
                    Ok(record) => {
                        if let ExecutionRecord::SellFill {
                            quote_out_net,
                            rent_recovered_lamports,
                            ..
                        } = record
                        {
                            balance += quote_out_net + rent_recovered_lamports;
                            lots.remove(&mint);
                            rent_paid.remove(&mint);
                        }
                        records.push(record);
                    }
                    Err(reason) => records.push(ExecutionRecord::ExitBlocked {
                        event_id: event.event_id.clone(),
                        mint: mint.clone(),
                        reason,
                    }),
                }
            }
        }
        let view = PortfolioView {
            cash_lamports: balance,
            open_mints: lots.keys().cloned().collect(),
        };
        if let Intent::Buy { spend_lamports } = strategy.decide(event, &view, risk) {
            if held {
                continue; // same-event exit never re-enters (elif semantics)
            }
            let attempts_fail = attempt_hash(strategy.id(), &event.event_id) % 10_000
                < frictions.failed_landing_bps;
            if attempts_fail {
                balance = balance.saturating_sub(
                    frictions.base_network_fee_lamports + frictions.priority_fee_lamports,
                );
                records.push(ExecutionRecord::AttemptFailedLanding {
                    event_id: event.event_id.clone(),
                    mint: mint.clone(),
                    network_fee_lamports: frictions.base_network_fee_lamports,
                    priority_fee_lamports: frictions.priority_fee_lamports,
                });
                continue;
            }
            match buy(
                event,
                book,
                &mint,
                spend_lamports,
                frictions,
                balance,
                &rent_paid,
                params,
            ) {
                Ok(record) => {
                    if let ExecutionRecord::BuyFill {
                        spend_lamports,
                        tokens_out,
                        network_fee_lamports,
                        priority_fee_lamports,
                        tip_lamports,
                        rent_lamports,
                        ..
                    } = record
                    {
                        let cost = spend_lamports
                            + network_fee_lamports
                            + priority_fee_lamports
                            + tip_lamports
                            + rent_lamports;
                        balance -= cost;
                        rent_paid.insert(mint.clone(), rent_lamports);
                        lots.insert(
                            mint.clone(),
                            OpenLot {
                                tokens: tokens_out,
                                entry_at_ms: event.feature_available_at_ms,
                            },
                        );
                    }
                    records.push(record);
                }
                Err(reason) => records.push(ExecutionRecord::AttemptRejected {
                    event_id: event.event_id.clone(),
                    mint: mint.clone(),
                    reason,
                }),
            }
        }
    }
    ProtocolRunOutcome {
        strategy: strategy.name(),
        records,
        ending_cash_lamports: balance,
        open_lots: lots,
    }
}

struct FillBreakdown {
    tokens_out: u64,
    protocol_fee: u64,
    creator_fee: u64,
    lp_fee: u64,
    synthetic_fallback: bool,
}

#[allow(clippy::too_many_arguments)]
fn buy(
    event: &Observation,
    book: &MarketBook,
    mint: &str,
    spend: u64,
    frictions: &ExecutionFrictions,
    balance: u64,
    rent_paid: &HashMap<String, u64>,
    params: &ProtocolParams,
) -> Result<ExecutionRecord, RejectionReason> {
    // Execute against the latency-shifted snapshot; the fee state itself
    // must predate the decision (no retroactive pricing).
    let exec_slot = event.slot.saturating_add(frictions.latency_slots);
    let snapshot = book
        .latest_at(mint, exec_slot)
        .ok_or(RejectionReason::MissingMarketState)?;
    match &snapshot.quote_mint {
        QuoteMint::Sol => {}
        QuoteMint::Usdc | QuoteMint::Unsupported(_) => {
            return Err(RejectionReason::UnsupportedQuoteRoute)
        }
    }
    let rent = if rent_paid.contains_key(mint) {
        0
    } else {
        params.ata_rent_lamports
    };
    let total_upfront = spend
        + frictions.base_network_fee_lamports
        + frictions.priority_fee_lamports
        + frictions.tip_lamports
        + rent;
    if total_upfront > balance {
        return Err(RejectionReason::InsufficientCash);
    }
    let fill = match &snapshot.state {
        VenueState::BondingCurve {
            virtual_token_reserves,
            virtual_quote_reserves,
            mint_total_supply,
            fee,
        } => {
            fee.ensure_not_retroactive(event.slot)
                .map_err(|_| RejectionReason::RetroactiveFeeState)?;
            let market_cap = crate::fees::bonding_curve_market_cap(
                *mint_total_supply,
                u128::from(*virtual_quote_reserves),
                u128::from(*virtual_token_reserves),
            )
            .ok_or(RejectionReason::InvalidReserves)?;
            let resolved: ResolvedFees = fee.resolve(market_cap);
            let fee_bps = (resolved.protocol_bps + resolved.creator_bps) as u16;
            let effective =
                apply_fee_in(spend, fee_bps).map_err(|_| RejectionReason::InvalidReserves)?;
            let tokens = curve_buy_out(effective, *virtual_token_reserves, *virtual_quote_reserves)
                .map_err(|_| RejectionReason::InvalidReserves)?;
            // Exact component split of the withheld input fee (PF0 CE-5).
            let withheld = spend - effective;
            let total = u64::from(fee_bps);
            let split = |bps: u64| (withheld * bps).checked_div(total).unwrap_or(0);
            FillBreakdown {
                tokens_out: tokens,
                protocol_fee: split(resolved.protocol_bps),
                creator_fee: split(resolved.creator_bps),
                lp_fee: 0,
                synthetic_fallback: resolved.synthetic_fallback,
            }
        }
        VenueState::PumpSwap {
            base_reserves,
            pool_quote_vault_amount,
            virtual_quote_reserves,
            base_mint_supply,
            fee,
            ..
        } => {
            fee.provenance
                .ensure_not_retroactive(event.slot)
                .map_err(|_| RejectionReason::RetroactiveFeeState)?;
            let effective_quote =
                effective_quote_reserves(*pool_quote_vault_amount, *virtual_quote_reserves)
                    .map_err(|_| RejectionReason::InvalidReserves)?;
            // Canonical pools are market-cap TIERED (observed FeeConfig):
            // resolve the actual components at this pool's cap.
            let market_cap = crate::fees::pool_market_cap(
                *base_mint_supply,
                u128::from(*base_reserves),
                u128::from(effective_quote),
            )
            .ok_or(RejectionReason::InvalidReserves)?;
            let resolved = fee.resolve(market_cap);
            let fee_bps =
                (resolved.lp_fee_bps + resolved.protocol_bps + resolved.creator_bps) as u16;
            let spend_effective =
                apply_fee_in(spend, fee_bps).map_err(|_| RejectionReason::InvalidReserves)?;
            let tokens = amm_sell_quote_buy(spend_effective, *base_reserves, effective_quote)
                .map_err(|_| RejectionReason::InvalidReserves)?;
            // Exact component split of the withheld input fee (no independent
            // recomputation -> no rounding drift; PF0 CE-5).
            let withheld = spend - spend_effective;
            let total = u64::from(fee_bps);
            let split = |bps: u64| (withheld * bps).checked_div(total).unwrap_or(0);
            FillBreakdown {
                tokens_out: tokens,
                protocol_fee: split(resolved.protocol_bps),
                creator_fee: split(resolved.creator_bps),
                lp_fee: split(resolved.lp_fee_bps),
                synthetic_fallback: resolved.synthetic_fallback,
            }
        }
    };
    // Adverse slippage on the token output (deterministic, conservative).
    let slipped = apply_fee_in(fill.tokens_out, frictions.slippage_bps as u16)
        .map_err(|_| RejectionReason::InvalidReserves)?;
    if slipped == 0 {
        return Err(RejectionReason::EmptyOutput);
    }
    Ok(ExecutionRecord::BuyFill {
        event_id: event.event_id.clone(),
        mint: mint.to_string(),
        slot: exec_slot,
        spend_lamports: spend,
        tokens_out: slipped,
        protocol_fee_lamports: fill.protocol_fee,
        creator_fee_lamports: fill.creator_fee,
        lp_fee_lamports: fill.lp_fee,
        network_fee_lamports: frictions.base_network_fee_lamports,
        priority_fee_lamports: frictions.priority_fee_lamports,
        tip_lamports: frictions.tip_lamports,
        rent_lamports: rent,
        fee_synthetic_fallback: fill.synthetic_fallback,
    })
}

/// Quote-in → base-out on the AMM (buy direction).
fn amm_sell_quote_buy(
    quote_in: u64,
    base_reserves: u64,
    effective_quote_reserves: u64,
) -> Result<u64, RejectionReason> {
    amm_buy(quote_in, base_reserves, effective_quote_reserves)
}

fn amm_buy(
    quote_in: u64,
    base_reserves: u64,
    effective_quote: u64,
) -> Result<u64, RejectionReason> {
    sniper_protocols::math::amm_buy_out(quote_in, base_reserves, effective_quote)
        .map_err(|_| RejectionReason::InvalidReserves)
}

fn sell(
    event: &Observation,
    book: &MarketBook,
    mint: &str,
    tokens: u64,
    frictions: &ExecutionFrictions,
    rent_paid: &HashMap<String, u64>,
    params: &ProtocolParams,
) -> Result<ExecutionRecord, RejectionReason> {
    let exec_slot = event.slot.saturating_add(frictions.latency_slots);
    let snapshot = book
        .latest_at(mint, exec_slot)
        .ok_or(RejectionReason::MissingMarketState)?;
    match &snapshot.quote_mint {
        QuoteMint::Sol => {}
        QuoteMint::Usdc | QuoteMint::Unsupported(_) => {
            return Err(RejectionReason::UnsupportedQuoteRoute)
        }
    }
    let rent_recovered = *rent_paid.get(mint).unwrap_or(&0);
    let _ = params;
    match &snapshot.state {
        VenueState::BondingCurve {
            virtual_token_reserves,
            virtual_quote_reserves,
            mint_total_supply,
            fee,
        } => {
            fee.ensure_not_retroactive(event.slot)
                .map_err(|_| RejectionReason::RetroactiveFeeState)?;
            let market_cap = crate::fees::bonding_curve_market_cap(
                *mint_total_supply,
                u128::from(*virtual_quote_reserves),
                u128::from(*virtual_token_reserves),
            )
            .ok_or(RejectionReason::InvalidReserves)?;
            let resolved = fee.resolve(market_cap);
            let gross = curve_sell_out(tokens, *virtual_token_reserves, *virtual_quote_reserves)
                .map_err(|_| RejectionReason::InvalidReserves)?;
            let fee_bps = (resolved.protocol_bps + resolved.creator_bps) as u16;
            let net = apply_fee_in(gross, fee_bps).map_err(|_| RejectionReason::InvalidReserves)?;
            // Adverse slippage on the quote output, then network costs.
            let net = apply_fee_in(net, frictions.slippage_bps as u16)
                .map_err(|_| RejectionReason::InvalidReserves)?;
            let net = net.saturating_sub(
                frictions.base_network_fee_lamports + frictions.priority_fee_lamports,
            );
            if net == 0 {
                return Err(RejectionReason::EmptyOutput);
            }
            // Exact component split of the withheld output fee (PF0 CE-5).
            let withheld = gross - apply_fee_in(gross, fee_bps).unwrap_or(gross);
            let total = u64::from(fee_bps);
            let split = |bps: u64| (withheld * bps).checked_div(total).unwrap_or(0);
            Ok(ExecutionRecord::SellFill {
                event_id: event.event_id.clone(),
                mint: mint.to_string(),
                slot: exec_slot,
                tokens_in: tokens,
                quote_out_net: net,
                protocol_fee_lamports: split(resolved.protocol_bps),
                creator_fee_lamports: split(resolved.creator_bps),
                lp_fee_lamports: 0,
                network_fee_lamports: frictions.base_network_fee_lamports,
                priority_fee_lamports: frictions.priority_fee_lamports,
                tip_lamports: 0,
                rent_recovered_lamports: rent_recovered,
                fee_synthetic_fallback: resolved.synthetic_fallback,
            })
        }
        VenueState::PumpSwap {
            base_reserves,
            pool_quote_vault_amount,
            virtual_quote_reserves,
            base_mint_supply,
            fee,
            ..
        } => {
            let effective_quote =
                effective_quote_reserves(*pool_quote_vault_amount, *virtual_quote_reserves)
                    .map_err(|_| RejectionReason::InvalidReserves)?;
            let market_cap = crate::fees::pool_market_cap(
                *base_mint_supply,
                u128::from(*base_reserves),
                u128::from(effective_quote),
            )
            .ok_or(RejectionReason::InvalidReserves)?;
            let resolved = fee.resolve(market_cap);
            let gross = amm_sell_out(tokens, *base_reserves, effective_quote)
                .map_err(|_| RejectionReason::InvalidReserves)?;
            let fee_bps =
                (resolved.lp_fee_bps + resolved.protocol_bps + resolved.creator_bps) as u16;
            let net = apply_fee_in(gross, fee_bps).map_err(|_| RejectionReason::InvalidReserves)?;
            let net = apply_fee_in(net, frictions.slippage_bps as u16)
                .map_err(|_| RejectionReason::InvalidReserves)?;
            let net = net.saturating_sub(
                frictions.base_network_fee_lamports + frictions.priority_fee_lamports,
            );
            if net == 0 {
                return Err(RejectionReason::EmptyOutput);
            }
            // Exact component split of the withheld output fee.
            let withheld = gross - apply_fee_in(gross, fee_bps).unwrap_or(gross);
            let total = u64::from(fee_bps);
            let split = |bps: u64| (withheld * bps).checked_div(total).unwrap_or(0);
            Ok(ExecutionRecord::SellFill {
                event_id: event.event_id.clone(),
                mint: mint.to_string(),
                slot: exec_slot,
                tokens_in: tokens,
                quote_out_net: net,
                protocol_fee_lamports: split(resolved.protocol_bps),
                creator_fee_lamports: split(resolved.creator_bps),
                lp_fee_lamports: split(resolved.lp_fee_bps),
                network_fee_lamports: frictions.base_network_fee_lamports,
                priority_fee_lamports: frictions.priority_fee_lamports,
                tip_lamports: 0,
                rent_recovered_lamports: rent_recovered,
                fee_synthetic_fallback: resolved.synthetic_fallback,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::{Phase, Safety};
    use crate::policies::EntryPolicy;

    fn observation(at_ms: u64) -> Observation {
        Observation {
            event_id: format!("e{at_ms}"),
            mint: "M".into(),
            slot: 10,
            chain_at_ms: at_ms - 5,
            observed_at_ms: at_ms,
            feature_available_at_ms: at_ms + 10,
            phase: Phase::ActiveCurve,
            safety: Safety::Eligible,
            coverage_ok: true,
            price_lamports_per_token: 1_000,
            exit_liquidity_lamports: u64::MAX / 4,
            net_flow_lamports: 1,
            unique_buyers: 10,
            creator_sell_lamports: 0,
            curve_progress_bps: 1_000,
            fee_bps: 100,
        }
    }

    fn curve_snapshot(slot: u64, fee: CurveFeeState) -> MarketSnapshot {
        MarketSnapshot {
            mint: "M".into(),
            slot,
            quote_mint: QuoteMint::Sol,
            state: VenueState::BondingCurve {
                virtual_token_reserves: 1_073_000_000_000_000,
                virtual_quote_reserves: 30_000_000_000,
                mint_total_supply: 1_000_000_000_000_000,
                fee,
            },
        }
    }

    fn tiered_fee(observed_slot: u64, protocol: u64, creator: u64) -> CurveFeeState {
        let mut fee = CurveFeeState::synthetic_fallback_100bps(observed_slot);
        fee.tiers = vec![crate::fees::FeeTier {
            market_cap_lamports_threshold: 0,
            protocol_bps: protocol,
            creator_bps: creator,
        }];
        fee.provenance.source = crate::fees::FeeSourceKind::FeeConfigAccount;
        fee
    }

    fn run_one(
        events: &[Observation],
        book: &MarketBook,
        strategy: &mut dyn Strategy,
        frictions: &ExecutionFrictions,
    ) -> ProtocolRunOutcome {
        run(
            events,
            book,
            strategy,
            100_000_000,
            &RiskConstraints::default(),
            frictions,
            &ProtocolParams::default(),
        )
    }

    fn fills(records: &[ExecutionRecord]) -> Vec<&ExecutionRecord> {
        records
            .iter()
            .filter(|r| matches!(r, ExecutionRecord::BuyFill { .. }))
            .collect()
    }

    #[test]
    fn buy_prices_on_real_reserve_math_with_fee_on_in() {
        let mut book = MarketBook::new();
        book.push(curve_snapshot(9, tiered_fee(9, 100, 0)));
        let events = vec![observation(100)];
        let outcome = run_one(
            &events,
            &book,
            &mut EntryPolicy::s01(crate::StrategyKnobs::default()),
            &ExecutionFrictions::default(),
        );
        assert_eq!(fills(&outcome.records).len(), 1);
        let ExecutionRecord::BuyFill {
            spend_lamports,
            tokens_out,
            protocol_fee_lamports,
            ..
        } = fills(&outcome.records)[0]
        else {
            panic!("expected fill");
        };
        assert_eq!(*spend_lamports, 1_000_000);
        // Fee on the way in: 100 bps of the input.
        assert_eq!(*protocol_fee_lamports, 10_000);
        // Effective 990_000 priced on constant product, then 100 bps slip.
        let v_tok = 1_073_000_000_000_000u128;
        let v_sol = 30_000_000_000u128;
        let expected_raw = 990_000u128 * v_tok / (v_sol + 990_000);
        let expected = expected_raw * 9_900 / 10_000;
        assert_eq!(u128::from(*tokens_out), expected);
    }

    #[test]
    fn tier_change_across_market_cap_boundary_changes_fees() {
        // Small cap: 50/50 bps. Large cap: 25/0 bps. Same reserves except
        // quote, so the resolved tier (and the fee charged) differs.
        let mut book = MarketBook::new();
        let make_state = |threshold: u128| {
            let mut fee = CurveFeeState::synthetic_fallback_100bps(9);
            fee.provenance.source = crate::fees::FeeSourceKind::FeeConfigAccount;
            fee.tiers = vec![
                crate::fees::FeeTier {
                    market_cap_lamports_threshold: 0,
                    protocol_bps: 50,
                    creator_bps: 50,
                },
                crate::fees::FeeTier {
                    market_cap_lamports_threshold: threshold,
                    protocol_bps: 25,
                    creator_bps: 0,
                },
            ];
            fee
        };
        let mut small = curve_snapshot(9, make_state(1_000_000_000_000_000_000));
        small.state = VenueState::BondingCurve {
            virtual_token_reserves: 1_073_000_000_000_000,
            virtual_quote_reserves: 30_000_000_000,
            mint_total_supply: 1_000_000_000_000_000,
            fee: make_state(1_000_000_000_000_000_000),
        };
        let _ = small;
        book.push(curve_snapshot(10, make_state(10)));
        // market cap ~27.96 SOL; tier threshold 10 lamports -> top tier (25 bps).
        let events = vec![observation(100)];
        let outcome = run_one(
            &events,
            &book,
            &mut EntryPolicy::s01(crate::StrategyKnobs::default()),
            &ExecutionFrictions::default(),
        );
        let ExecutionRecord::BuyFill {
            protocol_fee_lamports,
            ..
        } = fills(&outcome.records)[0]
        else {
            panic!("expected fill");
        };
        // 25 bps of 1_000_000 = 2_500 (top tier), not 10_000 (fallback).
        assert_eq!(*protocol_fee_lamports, 2_500);
    }

    #[test]
    fn missing_config_fallback_is_labeled_in_the_fill_record() {
        // No observed fee config: the synthetic fallback resolves fees AND
        // the fill record carries the label so receipts can never present
        // assumption-priced economics as observed.
        let mut book = MarketBook::new();
        book.push(curve_snapshot(
            9,
            CurveFeeState::synthetic_fallback_100bps(9),
        ));
        let events = vec![observation(100)];
        let outcome = run_one(
            &events,
            &book,
            &mut EntryPolicy::s01(crate::StrategyKnobs::default()),
            &ExecutionFrictions::default(),
        );
        let ExecutionRecord::BuyFill {
            fee_synthetic_fallback,
            protocol_fee_lamports,
            ..
        } = fills(&outcome.records)[0]
        else {
            panic!("expected fill");
        };
        assert!(fee_synthetic_fallback);
        assert_eq!(*protocol_fee_lamports, 10_000);
    }

    #[test]
    fn unsupported_quote_route_fails_closed() {
        let mut book = MarketBook::new();
        book.push(MarketSnapshot {
            mint: "M".into(),
            slot: 10,
            quote_mint: QuoteMint::Usdc,
            state: VenueState::BondingCurve {
                virtual_token_reserves: 1_073_000_000_000_000,
                virtual_quote_reserves: 30_000_000_000,
                mint_total_supply: 1_000_000_000_000_000,
                fee: tiered_fee(9, 100, 0),
            },
        });
        let events = vec![observation(100)];
        let outcome = run_one(
            &events,
            &book,
            &mut EntryPolicy::s01(crate::StrategyKnobs::default()),
            &ExecutionFrictions::default(),
        );
        assert!(matches!(
            outcome.records[0],
            ExecutionRecord::AttemptRejected {
                reason: RejectionReason::UnsupportedQuoteRoute,
                ..
            }
        ));
        assert_eq!(outcome.ending_cash_lamports, 100_000_000, "no cash moved");
    }

    #[test]
    fn missing_market_state_rejects_never_fills_optimistically() {
        let book = MarketBook::new();
        let events = vec![observation(100)];
        let outcome = run_one(
            &events,
            &book,
            &mut EntryPolicy::s01(crate::StrategyKnobs::default()),
            &ExecutionFrictions::default(),
        );
        assert!(matches!(
            outcome.records[0],
            ExecutionRecord::AttemptRejected {
                reason: RejectionReason::MissingMarketState,
                ..
            }
        ));
    }

    #[test]
    fn retroactive_fee_state_is_refused() {
        let mut book = MarketBook::new();
        // Fee state observed AFTER the decision slot (snapshot slot 9 but
        // fee observed at slot 50 > decision slot 10).
        book.push(curve_snapshot(9, tiered_fee(50, 100, 0)));
        let events = vec![observation(100)];
        let outcome = run_one(
            &events,
            &book,
            &mut EntryPolicy::s01(crate::StrategyKnobs::default()),
            &ExecutionFrictions::default(),
        );
        assert!(matches!(
            outcome.records[0],
            ExecutionRecord::AttemptRejected {
                reason: RejectionReason::RetroactiveFeeState,
                ..
            }
        ));
    }

    #[test]
    fn failed_landing_charges_network_costs_without_filling() {
        let mut book = MarketBook::new();
        book.push(curve_snapshot(9, tiered_fee(9, 100, 0)));
        let events = vec![observation(100)];
        let frictions = ExecutionFrictions {
            failed_landing_bps: 10_000, // every attempt fails
            ..Default::default()
        };
        let outcome = run_one(
            &events,
            &book,
            &mut EntryPolicy::s01(crate::StrategyKnobs::default()),
            &frictions,
        );
        assert!(fills(&outcome.records).is_empty());
        assert!(matches!(
            outcome.records[0],
            ExecutionRecord::AttemptFailedLanding { .. }
        ));
        // Network + priority fee burned, spend untouched.
        assert_eq!(outcome.ending_cash_lamports, 100_000_000 - 5_000 - 5_000);
    }

    #[test]
    fn latency_executes_against_later_snapshot() {
        // Snapshot at decision slot has generous reserves; the snapshot one
        // slot later is much thinner — the fill must price against the
        // LATER one.
        let mut book = MarketBook::new();
        book.push(curve_snapshot(11, tiered_fee(9, 0, 0)));
        let mut thin = curve_snapshot(12, tiered_fee(9, 0, 0));
        thin.state = VenueState::BondingCurve {
            virtual_token_reserves: 100_000,
            virtual_quote_reserves: 30_000_000,
            mint_total_supply: 1_000,
            fee: tiered_fee(9, 0, 0),
        };
        book.push(thin);
        let events = vec![observation(100)];
        let frictions = ExecutionFrictions {
            latency_slots: 2, // exec at slot 12 (thin)
            ..Default::default()
        };
        let outcome = run_one(
            &events,
            &book,
            &mut EntryPolicy::s01(crate::StrategyKnobs::default()),
            &frictions,
        );
        let ExecutionRecord::BuyFill { tokens_out, .. } = fills(&outcome.records)[0] else {
            panic!("expected fill");
        };
        // Thin curve: 1_000_000 * 100_000 / (30_000_000 + 1_000_000) = 3225
        // raw (0 bps fee), minus 100 bps slippage -> 3192.
        assert_eq!(*tokens_out, 3_192);
    }

    #[test]
    fn negative_effective_pumpswap_reserves_block_the_exit() {
        let mut book = MarketBook::new();
        book.push(MarketSnapshot {
            mint: "M".into(),
            slot: 9,
            quote_mint: QuoteMint::Sol,
            state: VenueState::PumpSwap {
                base_reserves: 1_000_000_000,
                pool_quote_vault_amount: 1_000,
                virtual_quote_reserves: -2_000, // effective negative
                base_mint_supply: 1_000_000_000,
                fee: crate::fees::PumpSwapFeeState::canonical_reference(5),
                canonical: true,
            },
        });
        // Enter first (blocked state also blocks buys via InvalidReserves),
        // then force a sell through the public path.
        let events = vec![observation(100)];
        let outcome = run_one(
            &events,
            &book,
            &mut EntryPolicy::s01(crate::StrategyKnobs::default()),
            &ExecutionFrictions::default(),
        );
        assert!(matches!(
            outcome.records[0],
            ExecutionRecord::AttemptRejected {
                reason: RejectionReason::InvalidReserves,
                ..
            }
        ));
    }

    #[test]
    fn pumpswap_sell_uses_documented_25_bps_and_signed_reserves() {
        // Effective reserves = vault + virtual (virtual positive here).
        let mut book = MarketBook::new();
        book.push(MarketSnapshot {
            mint: "M".into(),
            slot: 9,
            quote_mint: QuoteMint::Sol,
            state: VenueState::PumpSwap {
                base_reserves: 1_000_000_000_000,
                pool_quote_vault_amount: 40_000_000_000,
                virtual_quote_reserves: 10_000_000_000,
                base_mint_supply: 1_000_000_000_000,
                fee: crate::fees::PumpSwapFeeState::canonical_reference(5),
                canonical: true,
            },
        });
        // Direct sell quote of 1_000_000 tokens against effective reserves.
        let effective = effective_quote_reserves(40_000_000_000, 10_000_000_000).unwrap();
        assert_eq!(effective, 50_000_000_000);
        let gross = amm_sell_out(1_000_000, 1_000_000_000_000, effective).unwrap();
        assert!(gross > 0);
        // 25 bps documented total fee applies on top.
        let net = apply_fee_in(gross, 25).unwrap();
        assert_eq!(net, gross * 9_975 / 10_000);
    }
}
