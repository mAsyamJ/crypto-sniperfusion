//! Instruction-data conformance against the OFFICIAL pump.fun IDLs
//! (`pump-fun/pump-public-docs` @ 639b87f, `idl/pump.json` /
//! `idl/pump_amm.json`).
//!
//! The IDL buy family — `buy`, `buy_exact_sol_in`, `buy_v2`,
//! `buy_exact_quote_in_v2`, `buy_v3`, `buy_exact_quote_in_v3` — carries a
//! trailing `partial_fill: bool` argument after its u64s; the sell family
//! (`sell`, `sell_v2`, `sell_v3`) takes exactly two u64s. These tests pin
//! both the discriminator bytes and the full data layout so a deployed-IDL
//! drift fails loudly instead of producing silently-rejected transactions.
//!
//! Discriminator vectors below were recomputed from the IDL names
//! (sha256("global:<name>")[0..8]) and cross-checked against the IDL
//! instruction tables on 2026-11-02.

use sniper_protocols::amm::PumpSwapIx;
use sniper_protocols::curve::CurveTradeIx;
use sniper_protocols::pda::{self, parse};
use sniper_protocols::{
    buy_exact_quote_in_v3, buy_v2, buy_v3, pumpswap_sell_v2_complete, sell_v2, sell_v3,
    CurveTradeContext, CurveV2Extras, PumpSwapSellV2Inputs,
};

const AMOUNT: u64 = 1_000_000;
const LIMIT: u64 = 50_000_000;

fn ctx() -> CurveTradeContext {
    CurveTradeContext::sol_paired(parse(pda::GLOBAL), parse(pda::WSOL_MINT))
}

fn extras() -> CurveV2Extras {
    CurveV2Extras {
        creator: parse(pda::MINT_AUTHORITY),
        fee_recipient: parse("62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV"),
    }
}

fn le(data: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(data[at..at + 8].try_into().unwrap())
}

#[test]
fn buy_v3_data_layout_matches_idl() {
    let ix = buy_v3(&ctx(), AMOUNT, LIMIT, 0, false);
    assert_eq!(&ix.data[..8], &CurveTradeIx::BuyV3.discriminator());
    assert_eq!(ix.data.len(), 8 + 8 + 8 + 1, "u64 + u64 + partial_fill");
    assert_eq!(le(&ix.data, 8), AMOUNT);
    assert_eq!(le(&ix.data, 16), LIMIT);
    assert_eq!(ix.data[24], 0, "partial_fill=false encodes as 0");

    let t = buy_v3(&ctx(), AMOUNT, LIMIT, 0, true);
    assert_eq!(t.data[24], 1, "partial_fill=true encodes as 1");
}

#[test]
fn buy_exact_quote_in_v3_data_layout_matches_idl() {
    let ix = buy_exact_quote_in_v3(&ctx(), AMOUNT, LIMIT, 0, true);
    assert_eq!(
        &ix.data[..8],
        &CurveTradeIx::BuyExactQuoteInV3.discriminator()
    );
    assert_eq!(ix.data.len(), 8 + 8 + 8 + 1);
    assert_eq!(le(&ix.data, 8), AMOUNT);
    assert_eq!(le(&ix.data, 16), LIMIT);
    assert_eq!(ix.data[24], 1);
}

#[test]
fn buy_v2_data_layout_matches_idl() {
    let ix = buy_v2(&ctx(), &extras(), AMOUNT, LIMIT, 0, false);
    assert_eq!(&ix.data[..8], &CurveTradeIx::BuyV2.discriminator());
    assert_eq!(ix.data.len(), 8 + 8 + 8 + 1);
    assert_eq!(le(&ix.data, 8), AMOUNT);
    assert_eq!(le(&ix.data, 16), LIMIT);
    assert_eq!(ix.data[24], 0);
}

#[test]
fn sell_family_stays_two_u64_args() {
    let s3 = sell_v3(&ctx(), AMOUNT, LIMIT, 0);
    assert_eq!(&s3.data[..8], &CurveTradeIx::SellV3.discriminator());
    assert_eq!(s3.data.len(), 8 + 8 + 8);
    assert_eq!(le(&s3.data, 8), AMOUNT);
    assert_eq!(le(&s3.data, 16), LIMIT);

    let s2 = sell_v2(&ctx(), &extras(), AMOUNT, LIMIT, 0);
    assert_eq!(&s2.data[..8], &CurveTradeIx::SellV2.discriminator());
    assert_eq!(s2.data.len(), 8 + 8 + 8);
}

#[test]
fn discriminators_match_official_idl_at_639b87f() {
    // (instruction, sha256("global:<name>")[0..8] from idl/pump.json)
    for (ix, disc) in [
        (CurveTradeIx::Create, "181ec828051c0777"),
        (CurveTradeIx::CreateV2, "d6904cec5f8b31b4"),
        (CurveTradeIx::BuyLegacy, "66063d1201daebea"),
        (CurveTradeIx::BuyExactSolIn, "38fc74089edfcd5f"),
        (CurveTradeIx::BuyV2, "b817ee6167c5d33d"),
        (CurveTradeIx::BuyExactQuoteInV2, "c2ab1c46684d5b2f"),
        (CurveTradeIx::BuyV3, "07051dc4f5176550"),
        (CurveTradeIx::BuyExactQuoteInV3, "e1f7501ed5b38488"),
        (CurveTradeIx::SellLegacy, "33e685a4017f83ad"),
        (CurveTradeIx::SellV2, "5df6823ce7e940b2"),
        (CurveTradeIx::SellV3, "1c92de7726c469d5"),
        (CurveTradeIx::Migrate, "9beae792ec9ea21e"),
        (CurveTradeIx::MigrateV2, "bbcb121fceedfe29"),
    ] {
        let want: [u8; 8] = hex(disc);
        assert_eq!(ix.discriminator(), want, "discriminator drift on {disc}");
    }
    // idl/pump_amm.json
    assert_eq!(PumpSwapIx::SellV2.discriminator(), hex("5df6823ce7e940b2"));
}

fn hex(s: &str) -> [u8; 8] {
    let mut out = [0u8; 8];
    for (i, pair) in s.as_bytes().chunks(2).enumerate() {
        let hi = (pair[0] as char).to_digit(16).unwrap() as u8;
        let lo = (pair[1] as char).to_digit(16).unwrap() as u8;
        out[i] = (hi << 4) | lo;
    }
    out
}

#[test]
fn pumpswap_sell_v2_data_layout_matches_idl() {
    let inputs = PumpSwapSellV2Inputs {
        pool: parse(pda::GLOBAL), // arbitrary valid pubkey; layout test only
        user: parse(pda::MINT_AUTHORITY),
        base_mint: parse(pda::WSOL_MINT),
        quote_mint: parse(pda::USDC_MINT),
        user_base_token_account: parse(pda::GLOBAL),
        user_quote_token_account: parse(pda::GLOBAL),
        pool_base_token_account: parse(pda::GLOBAL),
        pool_quote_token_account: parse(pda::GLOBAL),
        base_token_program: pda::token_program(),
        quote_token_program: pda::token_program(),
        amm_event_authority: parse(pda::GLOBAL),
        base_amount_in: AMOUNT,
        min_quote_amount_out: LIMIT,
        buyback_index: 0,
    };
    let ix = pumpswap_sell_v2_complete(&inputs);
    assert_eq!(ix.accounts.len(), 17);
    assert_eq!(ix.data.len(), 8 + 8 + 8, "sell_v2: two u64 args, no bool");
    assert_eq!(le(&ix.data, 8), AMOUNT);
    assert_eq!(le(&ix.data, 16), LIMIT);
}
