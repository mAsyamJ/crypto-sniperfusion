//! Instruction builders: pump.fun curve V3/V2 buy+sell, PumpSwap sell_v2.
//!
//! Account orders are the official doc tables (TRADE_V3.md / BUY.md /
//! SELL.md / PUMP_SWAP_TRADE_V2.md @ 639b87f), asserted count-by-count in
//! tests against `required_accounts()`. Args are borsh u64s after the
//! discriminator. Builders are pure: they produce
//! `Instruction`s and never touch accounts or sign anything.

use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;

use crate::amm::PumpSwapIx;
use crate::curve::CurveTradeIx;
use crate::pda::{self, parse};

/// Buyback fee recipient wallets (FEE_RECIPIENTS.md; v3/v2 pass one of
/// these or their quote ATAs).
pub const BUYBACK_FEE_RECIPIENTS: [&str; 8] = [
    "5YxQFdt3Tr9zJLvkFccqXVUwhdTWJQc1fFg2YPbxvxeD",
    "9M4giFFMxmFGXtc3feFzRai56WbBqehoSeRE5GK7gf7",
    "GXPFM2caqTtQYC2cJ5yJRi9VDkpsYZXzYdwYpGnLmtDL",
    "3BpXnfJaUTiwXnJNe7Ej1rcbzqTTQUvLShZaWazebsVR",
    "5cjcW9wExnJJiqgLjq7DEG75Pm6JBgE1hNv4B2vHXUW6",
    "EHAAiTxcdDwQ3U4bU6YcMsQGaekdzLS3B5SmYo46kJtL",
    "5eHhjP8JaYkz83CWwvGU2uMUXefd3AazWGx4gpcuEEYD",
    "A7hAgCzFw14fejgCp387JUJRMNyz4j89JKnhtKU8piqW",
];

/// A deterministic buyback-recipient picker (index-based; the pick is
/// arbitrary per the docs — "Pick any of the 8").
pub fn buyback_fee_recipient(index: usize) -> Pubkey {
    parse(BUYBACK_FEE_RECIPIENTS[index % BUYBACK_FEE_RECIPIENTS.len()])
}

fn u64_le(v: u64) -> [u8; 8] {
    v.to_le_bytes()
}

fn ix(
    program: &Pubkey,
    discriminator: [u8; 8],
    args: &[u64],
    accounts: Vec<AccountMeta>,
) -> Instruction {
    let mut data = Vec::with_capacity(8 + 8 * args.len());
    data.extend_from_slice(&discriminator);
    for a in args {
        data.extend_from_slice(&u64_le(*a));
    }
    Instruction {
        program_id: *program,
        accounts,
        data,
    }
}

/// Shared inputs every curve trade needs. Derivations are computed here so
/// callers cannot mis-order PDAs.
#[derive(Debug, Clone)]
pub struct CurveTradeContext {
    pub user: Pubkey,
    pub base_mint: Pubkey,
    /// WSOL for SOL-paired coins.
    pub quote_mint: Pubkey,
    pub base_token_program: Pubkey,
    pub quote_token_program: Pubkey,
}

impl CurveTradeContext {
    /// SOL-paired default (create_v2 mint = Token-2022, WSOL quote = Token).
    pub fn sol_paired(user: Pubkey, base_mint: Pubkey) -> Self {
        Self {
            user,
            base_mint,
            quote_mint: parse(pda::WSOL_MINT),
            base_token_program: pda::token_2022_program(),
            quote_token_program: pda::token_program(),
        }
    }
}

/// `buy_v3(amount, max_sol_cost)` — 17 accounts per TRADE_V3.md.
pub fn buy_v3(
    ctx: &CurveTradeContext,
    amount: u64,
    max_sol_cost: u64,
    buyback_index: usize,
) -> Instruction {
    let pump = pda::pump_program();
    let bonding_curve = pda::bonding_curve(&ctx.base_mint);
    let accounts = vec![
        AccountMeta::new_readonly(parse(pda::GLOBAL), false),
        AccountMeta::new_readonly(ctx.base_mint, false),
        AccountMeta::new_readonly(ctx.quote_mint, false),
        AccountMeta::new_readonly(ctx.base_token_program, false),
        AccountMeta::new_readonly(ctx.quote_token_program, false),
        AccountMeta::new(bonding_curve, false),
        AccountMeta::new(
            pda::associated_bonding_curve(&ctx.base_mint, &bonding_curve, &ctx.base_token_program),
            false,
        ),
        AccountMeta::new_readonly(
            pda::spl_associated(&bonding_curve, &ctx.quote_token_program, &ctx.quote_mint),
            false,
        ),
        AccountMeta::new_readonly(ctx.user, true),
        AccountMeta::new(
            pda::spl_associated(&ctx.user, &ctx.base_token_program, &ctx.base_mint),
            false,
        ),
        AccountMeta::new_readonly(
            pda::spl_associated(&ctx.user, &ctx.quote_token_program, &ctx.quote_mint),
            false,
        ),
        AccountMeta::new(pda::user_volume_accumulator(&ctx.user), false),
        AccountMeta::new_readonly(pda::curve_fee_config(), false),
        AccountMeta::new_readonly(buyback_fee_recipient(buyback_index), false),
        AccountMeta::new_readonly(pda::system_program(), false),
        AccountMeta::new_readonly(parse(pda::EVENT_AUTHORITY), false),
        AccountMeta::new_readonly(pump, false),
    ];
    ix(
        &pump,
        CurveTradeIx::BuyV3.discriminator(),
        &[amount, max_sol_cost],
        accounts,
    )
}

/// `buy_exact_quote_in_v3(spendable_quote_in, min_tokens_out)` — same 17 accounts.
pub fn buy_exact_quote_in_v3(
    ctx: &CurveTradeContext,
    spendable_quote_in: u64,
    min_tokens_out: u64,
    buyback_index: usize,
) -> Instruction {
    let mut i = buy_v3(ctx, spendable_quote_in, min_tokens_out, buyback_index);
    i.data[..8].copy_from_slice(&CurveTradeIx::BuyExactQuoteInV3.discriminator());
    i
}

/// `sell_v3(amount, min_sol_output)` — same 17 accounts per TRADE_V3.md.
pub fn sell_v3(
    ctx: &CurveTradeContext,
    amount: u64,
    min_sol_output: u64,
    buyback_index: usize,
) -> Instruction {
    let mut i = buy_v3(ctx, amount, min_sol_output, buyback_index);
    i.data[..8].copy_from_slice(&CurveTradeIx::SellV3.discriminator());
    i
}

/// Extra inputs the wider v2 account list needs.
#[derive(Debug, Clone)]
pub struct CurveV2Extras {
    /// The curve's `creator` (creator-vault derives from it).
    pub creator: Pubkey,
    /// Normal (non-mayhem) fee recipient wallet.
    pub fee_recipient: Pubkey,
}

/// Global volume accumulator PDA `["global_volume_accumulator"]`.
pub fn global_volume_accumulator() -> Pubkey {
    Pubkey::find_program_address(&[b"global_volume_accumulator"], &pda::pump_program()).0
}

/// `buy_v2(amount, max_sol_cost)` — 27 accounts per BUY.md.
#[allow(clippy::too_many_arguments)]
pub fn buy_v2(
    ctx: &CurveTradeContext,
    extras: &CurveV2Extras,
    amount: u64,
    max_sol_cost: u64,
    buyback_index: usize,
) -> Instruction {
    let pump = pda::pump_program();
    let bonding_curve = pda::bonding_curve(&ctx.base_mint);
    let creator_vault = pda::creator_vault(&extras.creator);
    let user_vol = pda::user_volume_accumulator(&ctx.user);
    let gva = global_volume_accumulator();
    let buyback = buyback_fee_recipient(buyback_index);
    let accounts = vec![
        AccountMeta::new_readonly(parse(pda::GLOBAL), false),
        AccountMeta::new_readonly(ctx.base_mint, false),
        AccountMeta::new_readonly(ctx.quote_mint, false),
        AccountMeta::new_readonly(ctx.base_token_program, false),
        AccountMeta::new_readonly(ctx.quote_token_program, false),
        AccountMeta::new_readonly(pda::associated_token_program(), false),
        AccountMeta::new_readonly(extras.fee_recipient, false),
        AccountMeta::new(
            pda::spl_associated(
                &extras.fee_recipient,
                &ctx.quote_token_program,
                &ctx.quote_mint,
            ),
            false,
        ),
        AccountMeta::new_readonly(buyback, false),
        AccountMeta::new(
            pda::spl_associated(&buyback, &ctx.quote_token_program, &ctx.quote_mint),
            false,
        ),
        AccountMeta::new(bonding_curve, false),
        AccountMeta::new(
            pda::associated_bonding_curve(&ctx.base_mint, &bonding_curve, &ctx.base_token_program),
            false,
        ),
        AccountMeta::new(
            pda::spl_associated(&bonding_curve, &ctx.quote_token_program, &ctx.quote_mint),
            false,
        ),
        AccountMeta::new_readonly(ctx.user, true),
        AccountMeta::new(
            pda::spl_associated(&ctx.user, &ctx.base_token_program, &ctx.base_mint),
            false,
        ),
        AccountMeta::new(
            pda::spl_associated(&ctx.user, &ctx.quote_token_program, &ctx.quote_mint),
            false,
        ),
        AccountMeta::new(creator_vault, false),
        AccountMeta::new(
            pda::spl_associated(&creator_vault, &ctx.quote_token_program, &ctx.quote_mint),
            false,
        ),
        AccountMeta::new_readonly(pda::sharing_config(&ctx.base_mint), false),
        AccountMeta::new_readonly(gva, false),
        AccountMeta::new(user_vol, false),
        AccountMeta::new(
            pda::spl_associated(&user_vol, &ctx.quote_token_program, &ctx.quote_mint),
            false,
        ),
        AccountMeta::new_readonly(pda::curve_fee_config(), false),
        AccountMeta::new_readonly(parse(pda::FEE_PROGRAM), false),
        AccountMeta::new_readonly(pda::system_program(), false),
        AccountMeta::new_readonly(parse(pda::EVENT_AUTHORITY), false),
        AccountMeta::new_readonly(pump, false),
    ];
    ix(
        &pump,
        CurveTradeIx::BuyV2.discriminator(),
        &[amount, max_sol_cost],
        accounts,
    )
}

/// `sell_v2(amount, min_sol_output)` — 26 accounts per SELL.md (buy list
/// minus `global_volume_accumulator`).
pub fn sell_v2(
    ctx: &CurveTradeContext,
    extras: &CurveV2Extras,
    amount: u64,
    min_sol_output: u64,
    buyback_index: usize,
) -> Instruction {
    let mut i = buy_v2(ctx, extras, amount, min_sol_output, buyback_index);
    i.data[..8].copy_from_slice(&CurveTradeIx::SellV2.discriminator());
    // SELL.md order = BUY.md order minus account 20 (global_volume_accumulator).
    i.accounts.remove(19);
    i
}

/// Complete PumpSwap sell_v2 inputs (vaults and event authority come from
/// pool state / program constants the caller read).
pub struct PumpSwapSellV2Inputs {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub user_base_token_account: Pubkey,
    pub user_quote_token_account: Pubkey,
    pub pool_base_token_account: Pubkey,
    pub pool_quote_token_account: Pubkey,
    pub base_token_program: Pubkey,
    pub quote_token_program: Pubkey,
    pub amm_event_authority: Pubkey,
    pub base_amount_in: u64,
    pub min_quote_amount_out: u64,
    pub buyback_index: usize,
}

/// PumpSwap `sell_v2(base_amount_in, min_quote_amount_out)` — 17 accounts in
/// the documented order.
pub fn pumpswap_sell_v2_complete(x: &PumpSwapSellV2Inputs) -> Instruction {
    let amm = pda::pump_swap_program();
    let buyback = buyback_fee_recipient(x.buyback_index);
    let accounts = vec![
        AccountMeta::new(x.pool, false),
        AccountMeta::new_readonly(x.user, true),
        AccountMeta::new_readonly(pda::amm_global_config(), false),
        AccountMeta::new_readonly(x.base_mint, false),
        AccountMeta::new_readonly(x.quote_mint, false),
        AccountMeta::new(x.user_base_token_account, false),
        AccountMeta::new(x.user_quote_token_account, false),
        AccountMeta::new(x.pool_base_token_account, false),
        AccountMeta::new(x.pool_quote_token_account, false),
        AccountMeta::new_readonly(x.base_token_program, false),
        AccountMeta::new_readonly(x.quote_token_program, false),
        AccountMeta::new_readonly(pda::system_program(), false),
        AccountMeta::new(
            Pubkey::find_program_address(&[b"user_volume_accumulator", x.user.as_ref()], &amm).0,
            false,
        ),
        AccountMeta::new_readonly(pda::amm_fee_config(), false),
        AccountMeta::new_readonly(
            pda::spl_associated(&buyback, &x.quote_token_program, &x.quote_mint),
            false,
        ),
        AccountMeta::new_readonly(x.amm_event_authority, false),
        AccountMeta::new_readonly(amm, false),
    ];
    ix(
        &amm,
        PumpSwapIx::SellV2.discriminator(),
        &[x.base_amount_in, x.min_quote_amount_out],
        accounts,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> CurveTradeContext {
        CurveTradeContext::sol_paired(parse(pda::GLOBAL), parse(pda::WSOL_MINT))
    }

    fn extras() -> CurveV2Extras {
        CurveV2Extras {
            creator: parse(pda::MINT_AUTHORITY),
            fee_recipient: parse("62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV"),
        }
    }

    #[test]
    fn v3_shapes_match_doc_tables() {
        for (name, ix) in [
            ("buy_v3", buy_v3(&ctx(), 100, 200, 0)),
            (
                "buy_exact_quote_in_v3",
                buy_exact_quote_in_v3(&ctx(), 100, 200, 0),
            ),
            ("sell_v3", sell_v3(&ctx(), 100, 200, 0)),
        ] {
            assert_eq!(ix.accounts.len(), 17, "{name} must take 17 accounts");
            assert_eq!(ix.program_id, pda::pump_program());
            // Arg encoding: discriminator + 2 u64 LE.
            assert_eq!(ix.data.len(), 8 + 16, "{name} args");
            assert_eq!(&ix.data[8..16], &100u64.to_le_bytes());
            assert_eq!(&ix.data[16..24], &200u64.to_le_bytes());
        }
        assert_eq!(
            &buy_v3(&ctx(), 1, 2, 0).data[..8],
            &CurveTradeIx::BuyV3.discriminator()
        );
        assert_eq!(
            &buy_exact_quote_in_v3(&ctx(), 1, 2, 0).data[..8],
            &CurveTradeIx::BuyExactQuoteInV3.discriminator()
        );
        assert_eq!(
            &sell_v3(&ctx(), 1, 2, 0).data[..8],
            &CurveTradeIx::SellV3.discriminator()
        );
    }

    #[test]
    fn v3_user_is_the_only_signer() {
        let c = ctx();
        let ix = buy_v3(&c, 1, 2, 0);
        let signers: Vec<_> = ix.accounts.iter().filter(|m| m.is_signer).collect();
        assert_eq!(signers.len(), 1, "exactly the user signs");
        assert_eq!(signers[0].pubkey, c.user);
    }

    #[test]
    fn v3_account_order_matches_tradef3_table() {
        let c = ctx();
        let ix = buy_v3(&c, 1, 2, 3);
        let bc = pda::bonding_curve(&c.base_mint);
        assert_eq!(ix.accounts[0].pubkey, parse(pda::GLOBAL)); // 1 global
        assert_eq!(ix.accounts[5].pubkey, bc); // 6 bonding_curve
        assert_eq!(ix.accounts[8].pubkey, c.user); // 9 user
        assert_eq!(ix.accounts[12].pubkey, pda::curve_fee_config()); // 13 fee_config
        assert_eq!(ix.accounts[13].pubkey, buyback_fee_recipient(3)); // 14 buyback
        assert_eq!(ix.accounts[14].pubkey, pda::system_program()); // 15 system
        assert_eq!(ix.accounts[15].pubkey, parse(pda::EVENT_AUTHORITY)); // 16 event auth
        assert_eq!(ix.accounts[16].pubkey, pda::pump_program()); // 17 program
    }

    #[test]
    fn v2_shapes_match_doc_tables() {
        let buy = buy_v2(&ctx(), &extras(), 1, 2, 0);
        assert_eq!(buy.accounts.len(), 27, "buy_v2 takes 27 accounts");
        assert_eq!(&buy.data[..8], &CurveTradeIx::BuyV2.discriminator());

        let sell = sell_v2(&ctx(), &extras(), 1, 2, 0);
        assert_eq!(sell.accounts.len(), 26, "sell_v2 takes 26 accounts");
        assert_eq!(&sell.data[..8], &CurveTradeIx::SellV2.discriminator());
        // SELL order = BUY order minus index 19 (global_volume_accumulator).
        for (i, acc) in sell.accounts.iter().enumerate() {
            let buy_acc = if i < 19 {
                &buy.accounts[i]
            } else {
                &buy.accounts[i + 1]
            };
            assert_eq!(
                acc.pubkey, buy_acc.pubkey,
                "sell account {i} must match buy-minus-gva"
            );
        }
    }

    #[test]
    fn v2_fee_and_vault_accounts_in_documented_positions() {
        let ix = buy_v2(&ctx(), &extras(), 1, 2, 0);
        let e = extras();
        let c = ctx();
        assert_eq!(ix.accounts[6].pubkey, e.fee_recipient); // 7 fee_recipient
        assert_eq!(ix.accounts[16].pubkey, pda::creator_vault(&e.creator)); // 17 creator_vault
        assert_eq!(
            ix.accounts[17].pubkey,
            pda::spl_associated(
                &pda::creator_vault(&e.creator),
                &c.quote_token_program,
                &c.quote_mint
            )
        ); // 18 associated_creator_vault
        assert_eq!(ix.accounts[18].pubkey, pda::sharing_config(&c.base_mint)); // 19 sharing_config
        assert_eq!(ix.accounts[19].pubkey, global_volume_accumulator()); // 20 gva
        assert_eq!(ix.accounts[22].pubkey, pda::curve_fee_config()); // 23 fee_config
        assert_eq!(ix.accounts[23].pubkey, parse(pda::FEE_PROGRAM)); // 24 fee_program
    }

    #[test]
    fn pumpswap_sell_v2_shape_matches_doc_table() {
        let inputs = PumpSwapSellV2Inputs {
            pool: Pubkey::new_unique(),
            user: Pubkey::new_unique(),
            base_mint: Pubkey::new_unique(),
            quote_mint: parse(pda::WSOL_MINT),
            user_base_token_account: Pubkey::new_unique(),
            user_quote_token_account: Pubkey::new_unique(),
            pool_base_token_account: Pubkey::new_unique(),
            pool_quote_token_account: Pubkey::new_unique(),
            base_token_program: pda::token_2022_program(),
            quote_token_program: pda::token_program(),
            amm_event_authority: Pubkey::new_unique(),
            base_amount_in: 42,
            min_quote_amount_out: 24,
            buyback_index: 0,
        };
        let ix = pumpswap_sell_v2_complete(&inputs);
        assert_eq!(ix.accounts.len(), 17);
        assert_eq!(ix.program_id, pda::pump_swap_program());
        assert_eq!(&ix.data[..8], &PumpSwapIx::SellV2.discriminator());
        assert_eq!(&ix.data[8..16], &42u64.to_le_bytes());
        assert_eq!(&ix.data[16..24], &24u64.to_le_bytes());
        assert_eq!(ix.accounts[0].pubkey, inputs.pool);
        assert_eq!(ix.accounts[2].pubkey, pda::amm_global_config());
        assert_eq!(ix.accounts[13].pubkey, pda::amm_fee_config());
        let signers: Vec<_> = ix.accounts.iter().filter(|m| m.is_signer).collect();
        assert_eq!(signers.len(), 1);
        assert_eq!(signers[0].pubkey, inputs.user);
    }

    #[test]
    fn buyback_recipient_wraps_deterministically() {
        assert_eq!(buyback_fee_recipient(0), buyback_fee_recipient(8));
        assert_eq!(buyback_fee_recipient(3), buyback_fee_recipient(11));
        assert_ne!(buyback_fee_recipient(0), buyback_fee_recipient(1));
    }
}
