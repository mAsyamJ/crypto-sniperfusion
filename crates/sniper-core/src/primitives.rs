//! Integer money primitives. No floats on the money path: floats exist only
//! behind explicitly-named display helpers for logs.

use crate::error::SniperError;

macro_rules! checked_newtype {
    ($name:ident, $ty:ty, $doc:expr) => {
        #[doc = $doc]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
            serde::Serialize, serde::Deserialize,
        )]
        pub struct $name(pub $ty);

        impl $name {
            pub const ZERO: Self = Self(0);

            pub fn checked_add(self, rhs: Self) -> Result<Self, SniperError> {
                self.0
                    .checked_add(rhs.0)
                    .map(Self)
                    .ok_or(SniperError::MathOverflow {
                        op: stringify!($name add),
                        lhs: self.0 as u128,
                        rhs: rhs.0 as u128,
                    })
            }

            pub fn checked_sub(self, rhs: Self) -> Result<Self, SniperError> {
                self.0
                    .checked_sub(rhs.0)
                    .map(Self)
                    .ok_or(SniperError::MathOverflow {
                        op: stringify!($name sub),
                        lhs: self.0 as u128,
                        rhs: rhs.0 as u128,
                    })
            }

            /// Multiply by a scalar with overflow checking.
            pub fn checked_mul(self, scalar: u64) -> Result<Self, SniperError> {
                self.0
                    .checked_mul(scalar)
                    .map(Self)
                    .ok_or(SniperError::MathOverflow {
                        op: stringify!($name mul),
                        lhs: self.0 as u128,
                        rhs: scalar as u128,
                    })
            }
        }

        impl From<$ty> for $name {
            fn from(v: $ty) -> Self {
                Self(v)
            }
        }
    };
}

checked_newtype!(
    Lamports,
    u64,
    "Native SOL amount in lamports (1 SOL = 1e9)."
);
checked_newtype!(
    Microlamports,
    u64,
    "Priority-fee price unit: microlamports per compute unit."
);

/// Basis points: hundredths of a percent. Valid range 0..=10_000.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct BasisPoints(pub u16);

pub const BPS_MAX: u16 = 10_000;

impl BasisPoints {
    pub const ZERO: Self = Self(0);
    pub const HUNDRED_PCT: Self = Self(BPS_MAX);

    /// Construct with range validation.
    pub fn new(v: u16) -> Result<Self, SniperError> {
        if v <= BPS_MAX {
            Ok(Self(v))
        } else {
            Err(SniperError::Protocol(format!(
                "basis points out of range: {v} > {BPS_MAX}"
            )))
        }
    }

    /// `value * bps / 10_000` in u128 with a single rounding-down division.
    pub fn apply_to(self, value: u64) -> Result<u64, SniperError> {
        let prod =
            (value as u128)
                .checked_mul(self.0 as u128)
                .ok_or(SniperError::MathOverflow {
                    op: "bps apply",
                    lhs: value as u128,
                    rhs: self.0 as u128,
                })?;
        u64::try_from(prod / BPS_MAX as u128).map_err(|_| SniperError::MathOverflow {
            op: "bps apply",
            lhs: prod,
            rhs: BPS_MAX as u128,
        })
    }
}

/// Token amount in base units with explicit decimals. Two amounts with
/// different decimals never add — the type system forbids silent mis-scaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TokenAmount {
    pub amount: u128,
    pub decimals: u8,
}

impl TokenAmount {
    pub fn from_base_units(amount: u128, decimals: u8) -> Self {
        Self { amount, decimals }
    }

    pub fn checked_add(self, rhs: Self) -> Result<Self, SniperError> {
        if self.decimals != rhs.decimals {
            return Err(SniperError::Protocol(format!(
                "token decimals mismatch: {} vs {}",
                self.decimals, rhs.decimals
            )));
        }
        let amount = self
            .amount
            .checked_add(rhs.amount)
            .ok_or(SniperError::MathOverflow {
                op: "token add",
                lhs: self.amount,
                rhs: rhs.amount,
            })?;
        Ok(Self {
            amount,
            decimals: self.decimals,
        })
    }

    pub fn checked_sub(self, rhs: Self) -> Result<Self, SniperError> {
        if self.decimals != rhs.decimals {
            return Err(SniperError::Protocol(format!(
                "token decimals mismatch: {} vs {}",
                self.decimals, rhs.decimals
            )));
        }
        let amount = self
            .amount
            .checked_sub(rhs.amount)
            .ok_or(SniperError::MathOverflow {
                op: "token sub",
                lhs: self.amount,
                rhs: rhs.amount,
            })?;
        Ok(Self {
            amount,
            decimals: self.decimals,
        })
    }

    /// Display-only float conversion for logs and metrics. Never use for
    /// trading decisions or persistence.
    pub fn display_units(self) -> f64 {
        self.amount as f64 / 10f64.powi(self.decimals as i32)
    }
}
