//! PumpSwap (pAMM) instruction-version table.
//!
//! Source: pump-docs PUMP_SWAP_TRADE_V2.md @ 639b87f (v1 23/21-account with
//! remaining accounts; v2 17-account, no remaining accounts, fees stay in
//! pool; cashback pools require v1 — error 6079).

use crate::curve::anchor_discriminator;

/// PumpSwap trade/pool instructions we decode or build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PumpSwapIx {
    CreatePool,
    BuyV1,
    BuyExactQuoteInV1,
    BuyV2,
    BuyExactQuoteInV2,
    SellV1,
    SellV2,
}

impl PumpSwapIx {
    pub const fn snake_name(self) -> &'static str {
        match self {
            PumpSwapIx::CreatePool => "create_pool",
            PumpSwapIx::BuyV1 => "buy",
            PumpSwapIx::BuyExactQuoteInV1 => "buy_exact_quote_in",
            PumpSwapIx::BuyV2 => "buy_v2",
            PumpSwapIx::BuyExactQuoteInV2 => "buy_exact_quote_in_v2",
            PumpSwapIx::SellV1 => "sell",
            PumpSwapIx::SellV2 => "sell_v2",
        }
    }

    pub fn discriminator(self) -> [u8; 8] {
        anchor_discriminator(self.snake_name())
    }

    /// Declared accounts (excluding v1 remaining accounts).
    pub const fn required_accounts(self) -> usize {
        match self {
            PumpSwapIx::CreatePool => 18,
            PumpSwapIx::BuyV1 | PumpSwapIx::BuyExactQuoteInV1 => 23,
            PumpSwapIx::SellV1 => 21,
            PumpSwapIx::BuyV2 | PumpSwapIx::BuyExactQuoteInV2 | PumpSwapIx::SellV2 => 17,
        }
    }

    pub fn from_discriminator(bytes: &[u8]) -> Option<Self> {
        let all = [
            PumpSwapIx::CreatePool,
            PumpSwapIx::BuyV1,
            PumpSwapIx::BuyExactQuoteInV1,
            PumpSwapIx::BuyV2,
            PumpSwapIx::BuyExactQuoteInV2,
            PumpSwapIx::SellV1,
            PumpSwapIx::SellV2,
        ];
        let bytes: [u8; 8] = bytes.try_into().ok()?;
        all.into_iter().find(|ix| ix.discriminator() == bytes)
    }

    /// v1 instructions must be used on cashback pools (v2 fails 6079).
    pub const fn cashback_compatible(self) -> bool {
        matches!(
            self,
            PumpSwapIx::BuyV1 | PumpSwapIx::BuyExactQuoteInV1 | PumpSwapIx::SellV1
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discriminators_match_idl_derived_constants() {
        // sha256("global:<name>")[0..8]; note `buy`/`buy_v2`/`sell` collide by
        // design with the curve program names — the 8-byte discriminator is
        // only unique per program, decode is always program-scoped.
        let expected: &[(&str, &str)] = &[
            ("create_pool", "e992d18ecf6840bc"),
            ("buy", "66063d1201daebea"),
            ("buy_exact_quote_in", "c62e1552b4d9e870"),
            ("buy_v2", "b817ee6167c5d33d"),
            ("buy_exact_quote_in_v2", "c2ab1c46684d5b2f"),
            ("sell", "33e685a4017f83ad"),
            ("sell_v2", "5df6823ce7e940b2"),
        ];
        for (name, hex) in expected {
            let got: String = anchor_discriminator(name)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            assert_eq!(&got, hex, "discriminator drift for {name}");
        }
    }

    #[test]
    fn account_counts_match_protocol_doc() {
        assert_eq!(PumpSwapIx::BuyV1.required_accounts(), 23);
        assert_eq!(PumpSwapIx::SellV1.required_accounts(), 21);
        assert_eq!(PumpSwapIx::BuyV2.required_accounts(), 17);
        assert_eq!(PumpSwapIx::SellV2.required_accounts(), 17);
        assert_eq!(PumpSwapIx::BuyExactQuoteInV2.required_accounts(), 17);
    }

    #[test]
    fn cashback_compatibility_is_v1_only() {
        assert!(PumpSwapIx::BuyV1.cashback_compatible());
        assert!(!PumpSwapIx::BuyV2.cashback_compatible());
        assert!(!PumpSwapIx::SellV2.cashback_compatible());
    }

    #[test]
    fn roundtrip_discriminator_decode() {
        for ix in [
            PumpSwapIx::BuyV2,
            PumpSwapIx::SellV1,
            PumpSwapIx::CreatePool,
        ] {
            assert_eq!(
                PumpSwapIx::from_discriminator(&ix.discriminator()),
                Some(ix)
            );
        }
    }
}
