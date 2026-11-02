//! Launch/ trading venues. Program-ID resolution lives in `sniper-protocols`;
//! core only owns the discriminated enum so events stay venue-typed.

/// Canonical venue identifiers. Unknown venues must never be traded, only
/// logged — the registry decides what is registered at build time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Venue {
    /// pump.fun bonding curve (program `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P`).
    PumpFun,
    /// PumpSwap AMM, post-graduation (program `pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA`).
    PumpSwap,
    /// Raydium LaunchLab shared base — LetsBonk flavor (program `5thqcDwKp5QQ8US4XRMoseGeGbmLKMmoKZmS6zHrQAsA`).
    LetsBonk,
    /// Raydium LaunchLab shared base — StonkFun flavor (registry at M4/M6).
    StonkFun,
    /// Raydium AMM family (M6).
    RaydiumAmm,
    /// Meteora Dynamic Bonding Curve (M6).
    MeteoraDbc,
    /// Anything not registered yet. Never tradeable.
    Unknown,
}

impl Venue {
    pub fn is_tradeable(self) -> bool {
        !matches!(self, Venue::Unknown)
    }
}

impl std::fmt::Display for Venue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Venue::PumpFun => "pump_fun",
            Venue::PumpSwap => "pump_swap",
            Venue::LetsBonk => "lets_bonk",
            Venue::StonkFun => "stonk_fun",
            Venue::RaydiumAmm => "raydium_amm",
            Venue::MeteoraDbc => "meteora_dbc",
            Venue::Unknown => "unknown",
        };
        f.write_str(s)
    }
}
