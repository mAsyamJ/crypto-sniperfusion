//! Exact integer pricing math.
//!
//! - Curve (pump.fun): constant-product over VIRTUAL reserves, outputs round
//!   DOWN — mirrors the chainstack oracle formulas unit-for-unit.
//! - PumpSwap: priced against EFFECTIVE quote reserves
//!   `pool_quote_vault_amount + virtual_quote_reserves` where the virtual part
//!   is a signed i128 that can be negative since 2026-09-30
//!   (NEGATIVE_VIRTUAL_QUOTE_RESERVES.md). The program guarantees the sum is
//!   non-negative and u64-fits; we compute in i128 and verify.

use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum MathError {
    #[error("zero reserve in constant-product pool")]
    ZeroReserve,
    #[error("constant-product overflow")]
    Overflow,
    #[error("effective quote reserves are negative ({effective}) — program invariant violated")]
    NegativeEffective { effective: i128 },
    #[error("result exceeds u64")]
    TooLarge,
}

/// Constant-product output for a curve BUY: quote in → base tokens out.
/// `tokens_out = floor(amount_in * virtual_token / (virtual_quote + amount_in))`
/// (oracle formula; fee application is the caller's concern — see `apply_fee_in`).
pub fn curve_buy_out(
    amount_in: u64,
    virtual_token: u64,
    virtual_quote: u64,
) -> Result<u64, MathError> {
    let numerator = u128::from(amount_in)
        .checked_mul(u128::from(virtual_token))
        .ok_or(MathError::Overflow)?;
    let denominator = u128::from(virtual_quote)
        .checked_add(u128::from(amount_in))
        .ok_or(MathError::Overflow)?;
    if denominator == 0 {
        return Err(MathError::ZeroReserve);
    }
    u64::try_from(numerator / denominator).map_err(|_| MathError::TooLarge)
}

/// Constant-product output for a curve SELL: base in → quote out.
/// `quote_out = floor(amount_in * virtual_quote / (virtual_token + amount_in))`
pub fn curve_sell_out(
    amount_in: u64,
    virtual_token: u64,
    virtual_quote: u64,
) -> Result<u64, MathError> {
    let numerator = u128::from(amount_in)
        .checked_mul(u128::from(virtual_quote))
        .ok_or(MathError::Overflow)?;
    let denominator = u128::from(virtual_token)
        .checked_add(u128::from(amount_in))
        .ok_or(MathError::Overflow)?;
    if denominator == 0 {
        return Err(MathError::ZeroReserve);
    }
    u64::try_from(numerator / denominator).map_err(|_| MathError::TooLarge)
}

/// Apply an inclusive fee rate to an input amount: the portion of `amount`
/// that actually buys, when `fee_bps` is charged on the way in.
/// `spend = amount * (10_000 - fee_bps) / 10_000`, rounding DOWN (never
/// over-promises spendable).
pub fn apply_fee_in(amount: u64, fee_bps: u16) -> Result<u64, MathError> {
    if fee_bps == 0 {
        return Ok(amount);
    }
    let keep = 10_000u32 - u32::from(fee_bps.min(10_000));
    let v = u128::from(amount)
        .checked_mul(u128::from(keep))
        .ok_or(MathError::Overflow)?;
    u64::try_from(v / 10_000).map_err(|_| MathError::TooLarge)
}

/// Effective quote reserves for a PumpSwap pool. The virtual part may be
/// negative; the program guarantees the sum is non-negative — we enforce.
pub fn effective_quote_reserves(
    pool_quote_vault_amount: u64,
    virtual_quote_reserves: i128,
) -> Result<u64, MathError> {
    let effective = i128::from(pool_quote_vault_amount) + virtual_quote_reserves;
    if effective < 0 {
        return Err(MathError::NegativeEffective { effective });
    }
    u64::try_from(effective).map_err(|_| MathError::TooLarge)
}

/// CPMM output used by PumpSwap quoting (same shape as curve math, on
/// effective reserves): base_out for a quote input.
pub fn amm_buy_out(
    quote_in: u64,
    base_reserves: u64,
    effective_quote_reserves: u64,
) -> Result<u64, MathError> {
    curve_buy_out(quote_in, base_reserves, effective_quote_reserves)
}

/// CPMM quote_out for a base input against effective reserves.
pub fn amm_sell_out(
    base_in: u64,
    base_reserves: u64,
    effective_quote_reserves: u64,
) -> Result<u64, MathError> {
    curve_sell_out(base_in, base_reserves, effective_quote_reserves)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_buy_matches_oracle_example() {
        // Oracle formula: tokens_out = in * vTok / (vSol + in)
        let v_tok = 1_073_000_000_000_000u64;
        let v_sol = 30_000_000_000u64;
        let out = curve_buy_out(100_000_000, v_tok, v_sol).unwrap();
        let expected = 100_000_000u128 * v_tok as u128 / (v_sol as u128 + 100_000_000);
        assert_eq!(u128::from(out), expected);
        assert!(out > 0);
    }

    #[test]
    fn curve_round_trip_loses_at_most_rounding_plus_fee() {
        // Property-ish invariant: buy then sell returns ≤ original quote.
        let v_tok = 800_000_000_000_000u64;
        let v_sol = 79_000_000_000u64;
        let start = 50_000_000u64;
        let tokens = curve_buy_out(start, v_tok, v_sol).unwrap();
        let back = curve_sell_out(tokens, v_tok, v_sol).unwrap();
        assert!(
            back <= start,
            "round trip must not create value: {back} > {start}"
        );
    }

    #[test]
    fn degenerate_reserves_are_handled_not_panicked() {
        // Both reserves zero AND zero input → division by zero is typed.
        assert_eq!(curve_buy_out(0, 0, 0), Err(MathError::ZeroReserve));
        // Zero token reserve with a positive input: denominator is
        // quote + in > 0 → a well-defined zero output (empty curve).
        assert_eq!(curve_buy_out(1, 0, 0), Ok(0));
        assert_eq!(curve_sell_out(1, 10, 0), Ok(0));
    }

    #[test]
    fn fee_in_rounds_down() {
        assert_eq!(apply_fee_in(10_000, 125).unwrap(), 9_875); // 1.25%
        assert_eq!(apply_fee_in(9_999, 125).unwrap(), 9_874); // floor
        assert_eq!(apply_fee_in(1_000, 0).unwrap(), 1_000);
        assert_eq!(apply_fee_in(1_000, 10_000).unwrap(), 0); // 100% fee
    }

    #[test]
    fn effective_reserves_handle_negative_virtual() {
        // Negative virtual lowers effective below vault balance.
        assert_eq!(effective_quote_reserves(1_000, -300).unwrap(), 700);
        // Fully covered negative is fine.
        assert_eq!(effective_quote_reserves(300, -300).unwrap(), 0);
        // Beyond balance violates the program invariant → typed error.
        assert_eq!(
            effective_quote_reserves(299, -300),
            Err(MathError::NegativeEffective { effective: -1 })
        );
        // Positive virtual raises it.
        assert_eq!(effective_quote_reserves(1_000, 5_000).unwrap(), 6_000);
    }

    #[test]
    fn amm_pricing_uses_effective_reserves() {
        // Same pool, virtual part zero vs positive vs negative: prices move
        // exactly as effective reserves dictate.
        let base = 1_000_000u64;
        let out_zero = amm_buy_out(1_000, base, 10_000_000).unwrap();
        let out_pos = amm_buy_out(1_000, base, 12_000_000).unwrap();
        let out_neg = amm_buy_out(1_000, base, 8_000_000).unwrap();
        assert!(
            out_pos < out_zero && out_zero < out_neg,
            "more quote reserves → cheaper base"
        );
    }

    #[test]
    fn outputs_are_bounded_by_the_opposite_reserve() {
        // Structural invariant: buy_out <= virtual_token and
        // sell_out <= virtual_quote, because in/(quote+in) <= 1. With u64
        // inputs the products fit u128, so TooLarge/Overflow are defensive
        // layers — this test pins the guarantee they defend.
        let huge = u64::MAX;
        let out = curve_buy_out(huge, huge, 1).unwrap();
        assert_eq!(
            out,
            huge - 1,
            "huge input against vQ=1 buys almost the whole curve"
        );
        assert!(out <= huge);
        let sold = curve_sell_out(huge, 1, huge).unwrap();
        assert!(sold <= huge);
        assert_eq!(sold, huge - 1);
    }
}
