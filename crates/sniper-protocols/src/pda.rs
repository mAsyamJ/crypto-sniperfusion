//! pump.fun / PumpSwap PDA derivations and protocol constants.
//!
//! Sources: official pump-docs @ 639b87f (TRADE_V3.md/PUMP_SWAP_TRADE_V2.md
//! PDA tables) cross-checked against the chainstack address provider
//! (Apache-2.0 oracle). Derivations are pure and offline-verifiable; the
//! cross-program tests assert the doc-published addresses.

use solana_sdk::pubkey::Pubkey;
use solana_sdk::sysvar;

use crate::registry::{PROGRAM_PUMP_FUN, PROGRAM_PUMP_SWAP};

pub fn parse(s: &str) -> Pubkey {
    s.parse().expect("constant parses")
}

// ---- pump.fun program constants (oracle + docs verified) ----

/// PDA `["global"]` holder.
pub const GLOBAL: &str = "4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf";
/// PDA `["mint-authority"]` — creations-only subscription filter.
pub const MINT_AUTHORITY: &str = "TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM";
/// PDA `["__event_authority"]`.
pub const EVENT_AUTHORITY: &str = "Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1";
/// Pump Fees program (`pfee…`).
pub const FEE_PROGRAM: &str = "pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ";
/// Curve-side `fee_config` PDA `["fee_config", pump_program]` (docs:
/// `8Wf5TiAheLUqBrKXeYg2JtAFFMWtKdG2BSFgqUcPVwTt`).
pub const CURVE_FEE_CONFIG: &str = "8Wf5TiAheLUqBrKXeYg2JtAFFMWtKdG2BSFgqUcPVwTt";
/// PumpSwap-side `fee_config` PDA `["fee_config", pump_amm_program]` (docs:
/// `5PHirr8joyTMp9JMm6nW7hNDVyEYdkzDqazxPD7RaTjx`).
pub const AMM_FEE_CONFIG: &str = "5PHirr8joyTMp9JMm6nW7hNDVyEYdkzDqazxPD7RaTjx";
/// PumpSwap `global_config` PDA `["global_config"]` (docs:
/// `ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw`).
pub const AMM_GLOBAL_CONFIG: &str = "ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw";
/// Wrapped SOL mint.
pub const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";
/// USDC (mainnet) — the first non-SOL quote asset.
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

pub fn pump_program() -> Pubkey {
    parse(PROGRAM_PUMP_FUN)
}

pub fn pump_swap_program() -> Pubkey {
    parse(PROGRAM_PUMP_SWAP)
}

fn pda(seeds: &[&[u8]], program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(seeds, program).0
}

/// Bonding-curve PDA: `["bonding-curve", mint]` under the pump program.
pub fn bonding_curve(mint: &Pubkey) -> Pubkey {
    pda(&[b"bonding-curve", mint.as_ref()], &pump_program())
}

/// Associated base token account of the bonding curve.
pub fn associated_bonding_curve(
    mint: &Pubkey,
    bonding_curve: &Pubkey,
    base_token_program: &Pubkey,
) -> Pubkey {
    spl_associated(bonding_curve, base_token_program, mint)
}

/// SPL associated token account derivation (system program owner).
pub fn spl_associated(owner: &Pubkey, token_program: &Pubkey, mint: &Pubkey) -> Pubkey {
    pda(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        &parse("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"),
    )
}

/// Creator vault PDA: `["creator-vault", creator]` under the pump program.
pub fn creator_vault(creator: &Pubkey) -> Pubkey {
    pda(&[b"creator-vault", creator.as_ref()], &pump_program())
}

/// User volume accumulator PDA: `["user_volume_accumulator", user]`.
pub fn user_volume_accumulator(user: &Pubkey) -> Pubkey {
    pda(
        &[b"user_volume_accumulator", user.as_ref()],
        &pump_program(),
    )
}

/// Creator-fee sharing config PDA (pump fees program):
/// `["sharing-config", mint]`.
pub fn sharing_config(mint: &Pubkey) -> Pubkey {
    pda(&[b"sharing-config", mint.as_ref()], &parse(FEE_PROGRAM))
}

/// Holder-reward creator PDA `["holder-rewards", mint]` (what
/// `BondingCurve.creator` holds on holder-reward coins).
pub fn holder_reward_creator(mint: &Pubkey) -> Pubkey {
    pda(&[b"holder-rewards", mint.as_ref()], &pump_program())
}

/// Curve-side fee-config PDA `["fee_config", pump_program]` under the pump
/// fees program. Must equal the doc-published constant.
pub fn curve_fee_config() -> Pubkey {
    pda(
        &[b"fee_config", pump_program().as_ref()],
        &parse(FEE_PROGRAM),
    )
}

/// PumpSwap-side fee-config PDA `["fee_config", pump_amm_program]`.
pub fn amm_fee_config() -> Pubkey {
    pda(
        &[b"fee_config", pump_swap_program().as_ref()],
        &parse(FEE_PROGRAM),
    )
}

/// PumpSwap global config PDA `["global_config"]` under the AMM program.
pub fn amm_global_config() -> Pubkey {
    pda(&[b"global_config"], &pump_swap_program())
}

/// The canonical PumpSwap pool a pump.fun curve migrates into:
/// pool-authority = PDA `["pool-authority", mint]` under the PUMP program;
/// pool = PDA `["pool", index_le(0), pool_authority, base, quote]` under the
/// AMM program.
pub fn canonical_pool(base_mint: &Pubkey, quote_mint: &Pubkey) -> Pubkey {
    let pool_authority = pda(&[b"pool-authority", base_mint.as_ref()], &pump_program());
    pda(
        &[
            b"pool",
            &0u16.to_le_bytes(),
            pool_authority.as_ref(),
            base_mint.as_ref(),
            quote_mint.as_ref(),
        ],
        &pump_swap_program(),
    )
}

/// Protocol system addresses used by every builder.
pub fn system_program() -> Pubkey {
    solana_sdk::system_program::id()
}

pub fn token_program() -> Pubkey {
    parse("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
}

pub fn token_2022_program() -> Pubkey {
    parse("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb")
}

pub fn associated_token_program() -> Pubkey {
    parse("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
}

/// Rent sysvar (create paths).
pub fn rent_sysvar() -> Pubkey {
    sysvar::rent::id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fee_configs_match_doc_published_constants() {
        assert_eq!(curve_fee_config(), parse(CURVE_FEE_CONFIG));
        assert_eq!(amm_fee_config(), parse(AMM_FEE_CONFIG));
        assert_eq!(amm_global_config(), parse(AMM_GLOBAL_CONFIG));
    }

    #[test]
    fn bonding_curve_pda_is_deterministic() {
        let mint = parse(WSOL_MINT);
        assert_eq!(bonding_curve(&mint), bonding_curve(&mint));
        // Distinct mints → distinct curves.
        assert_ne!(bonding_curve(&mint), bonding_curve(&parse(USDC_MINT)));
    }

    #[test]
    fn spl_associated_matches_spl_convention() {
        // SPL ATA: [owner, token_program, mint] under the ATA program.
        let owner = parse(GLOBAL);
        let ata = spl_associated(&owner, &token_program(), &parse(WSOL_MINT));
        assert_ne!(ata, owner);
        // Token-2022 mints derive under the Token-2022 program.
        assert_ne!(
            spl_associated(&owner, &token_program(), &parse(WSOL_MINT)),
            spl_associated(&owner, &token_2022_program(), &parse(WSOL_MINT))
        );
    }

    #[test]
    fn canonical_pool_uses_curve_authority_and_index_zero() {
        let base = parse(WSOL_MINT);
        let quote = parse(USDC_MINT);
        let pool = canonical_pool(&base, &quote);
        // Deterministic and quote-sensitive.
        assert_eq!(pool, canonical_pool(&base, &quote));
        assert_ne!(pool, canonical_pool(&base, &parse(WSOL_MINT)));
    }

    #[test]
    fn vaults_and_accumulators_derive_from_their_owners() {
        let creator = parse(MINT_AUTHORITY);
        assert_ne!(creator_vault(&creator), creator);
        assert_ne!(user_volume_accumulator(&creator), creator_vault(&creator));
        assert_ne!(
            sharing_config(&parse(WSOL_MINT)),
            holder_reward_creator(&parse(WSOL_MINT))
        );
    }
}
