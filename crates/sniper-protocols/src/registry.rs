//! Venue → on-chain program registry. Only programs with M0-verified evidence
//! are registered; everything else is `UnsupportedVenue` (fail closed).

use solana_sdk::pubkey::Pubkey;
use thiserror::Error;

use sniper_core::Venue;

/// pump.fun bonding-curve program (source: official pump.json IDL `address`).
pub const PROGRAM_PUMP_FUN: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
/// PumpSwap AMM program (source: official pump_amm.json IDL `address`).
pub const PROGRAM_PUMP_SWAP: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
/// Raydium LaunchLab — LetsBonk flavor (source: chainstack letsbonk PLATFORM_CONFIGS).
pub const PROGRAM_LETS_BONK: &str = "5thqcDwKp5QQ8US4XRMoseGeGbmLKMmoKZmS6zHrQAsA";

#[derive(Debug, Error, PartialEq)]
#[error("venue {venue:?} has no registered program id (fail closed)")]
pub struct UnregisteredVenue {
    pub venue: Venue,
}

/// Resolve a venue to its program id. Unregistered venues are errors, never a
/// guess: an unknown program must not silently enter the pipeline.
pub fn program_id(venue: Venue) -> Result<Pubkey, UnregisteredVenue> {
    let s = match venue {
        Venue::PumpFun => PROGRAM_PUMP_FUN,
        Venue::PumpSwap => PROGRAM_PUMP_SWAP,
        Venue::LetsBonk => PROGRAM_LETS_BONK,
        Venue::StonkFun | Venue::RaydiumAmm | Venue::MeteoraDbc | Venue::Unknown => {
            return Err(UnregisteredVenue { venue })
        }
    };
    // Parse failure here is a programmer error in the constants, not a
    // runtime condition; the venue-level Result is about registration.
    Ok(s.parse::<Pubkey>()
        .expect("registry constants are valid base58 program ids"))
}

/// Reverse lookup: which venue does this program belong to, if registered.
pub fn program_id_for(program: &Pubkey) -> Option<Venue> {
    for venue in [Venue::PumpFun, Venue::PumpSwap, Venue::LetsBonk] {
        if let Ok(id) = program_id(venue) {
            if &id == program {
                return Some(venue);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_program_ids_parse_and_match_expected_constants() {
        assert_eq!(
            program_id(Venue::PumpFun).unwrap(),
            PROGRAM_PUMP_FUN.parse::<Pubkey>().unwrap()
        );
        assert_eq!(
            program_id(Venue::PumpSwap).unwrap(),
            PROGRAM_PUMP_SWAP.parse::<Pubkey>().unwrap()
        );
        assert_eq!(
            program_id(Venue::LetsBonk).unwrap(),
            PROGRAM_LETS_BONK.parse::<Pubkey>().unwrap()
        );
    }

    #[test]
    fn unregistered_venues_fail_closed() {
        for venue in [
            Venue::StonkFun,
            Venue::RaydiumAmm,
            Venue::MeteoraDbc,
            Venue::Unknown,
        ] {
            assert!(program_id(venue).is_err(), "{venue:?} must fail closed");
        }
    }

    #[test]
    fn reverse_lookup_roundtrip() {
        for venue in [Venue::PumpFun, Venue::PumpSwap, Venue::LetsBonk] {
            assert_eq!(program_id_for(&program_id(venue).unwrap()), Some(venue));
        }
        assert_eq!(program_id_for(&Pubkey::new_unique()), None);
    }
}
