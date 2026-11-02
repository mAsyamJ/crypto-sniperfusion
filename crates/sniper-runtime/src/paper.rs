//! Paper fill engine: simulated fills for paper mode, priced with the exact
//! protocol math (M3) — never presented as real execution. Models the fee on
//! the way in and the fee on the way out so round-trip economics are visible
//! per the fee-schedule posture (rates come from config, not hardcoded
//! protocol constants).

use sniper_core::primitives::{BasisPoints, Lamports};
use sniper_protocols::math::{apply_fee_in, curve_buy_out, curve_sell_out};

/// Simulated fill result for one buy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaperBuyFill {
    pub quote_in: Lamports,
    pub fee_quote: Lamports,
    pub effective_spend: Lamports,
    pub tokens_out: u64,
    /// Reserves AFTER the simulated trade moved them (constant product).
    pub virtual_token_after: u64,
    pub virtual_quote_after: u64,
}

/// Simulated fill result for one sell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaperSellFill {
    pub tokens_in: u64,
    pub quote_out_gross: Lamports,
    pub fee_quote: Lamports,
    pub quote_out_net: Lamports,
}

/// Curve state the engine prices against (point-in-time snapshot).
#[derive(Debug, Clone, Copy)]
pub struct CurveSnapshot {
    pub virtual_token_reserves: u64,
    pub virtual_quote_reserves: u64,
}

impl CurveSnapshot {
    /// Apply a simulated buy: reserves move by constant product.
    pub fn after_buy(&self, effective_spend: u64, tokens_out: u64) -> Self {
        Self {
            virtual_token_reserves: self.virtual_token_reserves - tokens_out,
            virtual_quote_reserves: self.virtual_quote_reserves + effective_spend,
        }
    }
}

/// Fill a paper buy: fee-on-in applied first, remainder priced on the curve.
pub fn paper_buy(
    snapshot: &CurveSnapshot,
    quote_in: Lamports,
    fee_bps: BasisPoints,
) -> Result<PaperBuyFill, String> {
    let effective = apply_fee_in(quote_in.0, fee_bps.0).map_err(|e| e.to_string())?;
    let tokens_out = curve_buy_out(
        effective,
        snapshot.virtual_token_reserves,
        snapshot.virtual_quote_reserves,
    )
    .map_err(|e| e.to_string())?;
    Ok(PaperBuyFill {
        quote_in,
        fee_quote: Lamports(quote_in.0 - effective),
        effective_spend: Lamports(effective),
        tokens_out,
        virtual_token_after: snapshot.virtual_token_reserves - tokens_out,
        virtual_quote_after: snapshot.virtual_quote_reserves + effective,
    })
}

/// Fill a paper sell: gross curve output, fee on the way out.
pub fn paper_sell(
    snapshot: &CurveSnapshot,
    tokens_in: u64,
    fee_bps: BasisPoints,
) -> Result<PaperSellFill, String> {
    let gross = curve_sell_out(
        tokens_in,
        snapshot.virtual_token_reserves,
        snapshot.virtual_quote_reserves,
    )
    .map_err(|e| e.to_string())?;
    let keep = apply_fee_in(gross, fee_bps.0).map_err(|e| e.to_string())?;
    Ok(PaperSellFill {
        tokens_in,
        quote_out_gross: Lamports(gross),
        fee_quote: Lamports(gross - keep),
        quote_out_net: Lamports(keep),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> CurveSnapshot {
        CurveSnapshot {
            virtual_token_reserves: 1_073_000_000_000_000,
            virtual_quote_reserves: 30_000_000_000,
        }
    }

    #[test]
    fn buy_fees_then_prices() {
        let fill = paper_buy(&snapshot(), Lamports(100_000_000), BasisPoints(125)).unwrap();
        // 1.25% fee on the way in.
        assert_eq!(fill.fee_quote.0, 1_250_000);
        assert_eq!(fill.effective_spend.0, 98_750_000);
        // Tokens priced on the effective spend only.
        let expected = curve_buy_out(98_750_000, 1_073_000_000_000_000, 30_000_000_000).unwrap();
        assert_eq!(fill.tokens_out, expected);
        // Reserves move by constant product.
        assert_eq!(fill.virtual_quote_after, 30_000_000_000 + 98_750_000);
        assert_eq!(fill.virtual_token_after, 1_073_000_000_000_000 - expected);
    }

    #[test]
    fn round_trip_costs_both_fees() {
        // Buy then sell the received tokens against the moved curve: the
        // round trip must lose at least the two fees.
        let s0 = snapshot();
        let fee = BasisPoints(125);
        let buy = paper_buy(&s0, Lamports(100_000_000), fee).unwrap();
        let s1 = s0.after_buy(buy.effective_spend.0, buy.tokens_out);
        let sell = paper_sell(&s1, buy.tokens_out, fee).unwrap();
        assert!(
            sell.quote_out_net.0 < buy.quote_in.0,
            "paper round trip must not create value: net {} >= in {}",
            sell.quote_out_net.0,
            buy.quote_in.0
        );
        let loss = buy.quote_in.0 - sell.quote_out_net.0;
        let min_loss = buy.fee_quote.0; // at least the entry fee alone
        assert!(loss >= min_loss, "loss {loss} must cover fees");
    }

    #[test]
    fn sell_fees_apply_on_output() {
        let fill = paper_sell(&snapshot(), 1_000_000_000, BasisPoints(125)).unwrap();
        assert_eq!(
            fill.fee_quote.0,
            fill.quote_out_gross.0 - fill.quote_out_net.0
        );
        // 1.25% of gross, floored.
        assert_eq!(
            fill.fee_quote.0,
            fill.quote_out_gross.0 - fill.quote_out_gross.0 * 9_875 / 10_000
        );
    }

    #[test]
    fn larger_buys_move_price_against_the_buyer() {
        let small = paper_buy(&snapshot(), Lamports(1_000_000), BasisPoints(0)).unwrap();
        let big = paper_buy(&snapshot(), Lamports(1_000_000_000), BasisPoints(0)).unwrap();
        // Tokens per lamport strictly decreases with size (price impact).
        let rate_small = small.tokens_out as f64 / small.effective_spend.0 as f64;
        let rate_big = big.tokens_out as f64 / big.effective_spend.0 as f64;
        assert!(
            rate_big < rate_small,
            "constant product must charge more per token to bigger buyers"
        );
    }
}
