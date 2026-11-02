//! pump.fun bonding-curve instruction-version table.
//!
//! Account counts and semantics: `docs/PROTOCOL_COMPATIBILITY.md` (source:
//! pump-docs TRADE_V3.md / BUY.md / SELL.md / COIN_CREATION.md @ 639b87f).
//! Discriminators are Anchor `sha256("global:<snake>")[0..8]`, recomputed at
//! runtime and asserted against IDL-derived constants in tests.

use sha2::{Digest, Sha256};

/// Anchor instruction discriminator: first 8 bytes of `sha256("global:<name>")`.
pub fn anchor_discriminator(snake_name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(b"global:");
    hasher.update(snake_name.as_bytes());
    let digest = hasher.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&digest[..8]);
    out
}

/// Bonding-curve trade/launch instructions we decode or build. Ordered by
/// generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CurveTradeIx {
    Create,
    CreateV2,
    BuyLegacy,
    BuyExactSolIn,
    BuyV2,
    BuyExactQuoteInV2,
    BuyV3,
    BuyExactQuoteInV3,
    SellLegacy,
    SellV2,
    SellV3,
    Migrate,
    MigrateV2,
}

impl CurveTradeIx {
    pub const fn snake_name(self) -> &'static str {
        match self {
            CurveTradeIx::Create => "create",
            CurveTradeIx::CreateV2 => "create_v2",
            CurveTradeIx::BuyLegacy => "buy",
            CurveTradeIx::BuyExactSolIn => "buy_exact_sol_in",
            CurveTradeIx::BuyV2 => "buy_v2",
            CurveTradeIx::BuyExactQuoteInV2 => "buy_exact_quote_in_v2",
            CurveTradeIx::BuyV3 => "buy_v3",
            CurveTradeIx::BuyExactQuoteInV3 => "buy_exact_quote_in_v3",
            CurveTradeIx::SellLegacy => "sell",
            CurveTradeIx::SellV2 => "sell_v2",
            CurveTradeIx::SellV3 => "sell_v3",
            CurveTradeIx::Migrate => "migrate",
            CurveTradeIx::MigrateV2 => "migrate_v2",
        }
    }

    /// Anchor discriminator for this instruction.
    pub fn discriminator(self) -> [u8; 8] {
        anchor_discriminator(self.snake_name())
    }

    /// Account-count contract from the official docs/IDL (see
    /// PROTOCOL_COMPATIBILITY tables). Used as a decode sanity bound.
    pub const fn required_accounts(self) -> usize {
        match self {
            CurveTradeIx::Create => 14,
            CurveTradeIx::CreateV2 => 16, // +3–5 remaining accounts for non-native quote
            CurveTradeIx::BuyLegacy | CurveTradeIx::BuyExactSolIn => 16,
            CurveTradeIx::BuyV2 | CurveTradeIx::BuyExactQuoteInV2 => 27,
            CurveTradeIx::BuyV3 | CurveTradeIx::BuyExactQuoteInV3 | CurveTradeIx::SellV3 => 17,
            CurveTradeIx::SellLegacy => 14,
            CurveTradeIx::SellV2 => 26,
            CurveTradeIx::Migrate => 25,
            CurveTradeIx::MigrateV2 => 27,
        }
    }

    /// Decode an instruction variant from its 8-byte discriminator.
    pub fn from_discriminator(bytes: &[u8]) -> Option<Self> {
        let all = [
            CurveTradeIx::Create,
            CurveTradeIx::CreateV2,
            CurveTradeIx::BuyLegacy,
            CurveTradeIx::BuyExactSolIn,
            CurveTradeIx::BuyV2,
            CurveTradeIx::BuyExactQuoteInV2,
            CurveTradeIx::BuyV3,
            CurveTradeIx::BuyExactQuoteInV3,
            CurveTradeIx::SellLegacy,
            CurveTradeIx::SellV2,
            CurveTradeIx::SellV3,
            CurveTradeIx::Migrate,
            CurveTradeIx::MigrateV2,
        ];
        let bytes: [u8; 8] = bytes.try_into().ok()?;
        all.into_iter().find(|ix| ix.discriminator() == bytes)
    }

    /// Does this instruction keep fees on the curve (V3 semantics)?
    pub const fn fees_on_curve(self) -> bool {
        matches!(
            self,
            CurveTradeIx::BuyV3 | CurveTradeIx::BuyExactQuoteInV3 | CurveTradeIx::SellV3
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Constants derived offline from the official IDL naming convention
    /// (sha256("global:<name>")[0..8]) and cross-checked against the well-known
    /// pump.fun `create`/`buy` discriminators seen in on-chain fixtures.
    #[test]
    fn discriminators_match_idl_derived_constants() {
        let expected: &[(&str, &str)] = &[
            ("create", "181ec828051c0777"),
            ("create_v2", "d6904cec5f8b31b4"),
            ("buy", "66063d1201daebea"),
            ("buy_exact_sol_in", "38fc74089edfcd5f"),
            ("buy_v2", "b817ee6167c5d33d"),
            ("buy_exact_quote_in_v2", "c2ab1c46684d5b2f"),
            ("buy_v3", "07051dc4f5176550"),
            ("buy_exact_quote_in_v3", "e1f7501ed5b38488"),
            ("sell", "33e685a4017f83ad"),
            ("sell_v2", "5df6823ce7e940b2"),
            ("sell_v3", "1c92de7726c469d5"),
            ("migrate", "9beae792ec9ea21e"),
            ("migrate_v2", "bbcb121fceedfe29"),
        ];
        for (name, hex) in expected {
            let computed = anchor_discriminator(name);
            let got: String = computed.iter().map(|b| format!("{b:02x}")).collect();
            assert_eq!(&got, hex, "discriminator drift for {name}");
        }
    }

    #[test]
    fn roundtrip_discriminator_decode() {
        for ix in [
            CurveTradeIx::BuyV3,
            CurveTradeIx::SellV2,
            CurveTradeIx::Create,
            CurveTradeIx::MigrateV2,
        ] {
            assert_eq!(
                CurveTradeIx::from_discriminator(&ix.discriminator()),
                Some(ix)
            );
        }
        assert_eq!(CurveTradeIx::from_discriminator(&[0u8; 8]), None);
        assert_eq!(CurveTradeIx::from_discriminator(&[1, 2, 3]), None);
    }

    #[test]
    fn account_counts_match_protocol_doc() {
        assert_eq!(CurveTradeIx::BuyV2.required_accounts(), 27);
        assert_eq!(CurveTradeIx::SellV2.required_accounts(), 26);
        assert_eq!(CurveTradeIx::BuyV3.required_accounts(), 17);
        assert_eq!(CurveTradeIx::BuyExactQuoteInV3.required_accounts(), 17);
        assert_eq!(CurveTradeIx::SellV3.required_accounts(), 17);
        assert_eq!(CurveTradeIx::BuyLegacy.required_accounts(), 16);
        assert_eq!(CurveTradeIx::SellLegacy.required_accounts(), 14);
        assert_eq!(CurveTradeIx::Create.required_accounts(), 14);
        assert_eq!(CurveTradeIx::CreateV2.required_accounts(), 16);
    }

    #[test]
    fn v3_flags_fees_on_curve() {
        assert!(CurveTradeIx::BuyV3.fees_on_curve());
        assert!(CurveTradeIx::SellV3.fees_on_curve());
        assert!(!CurveTradeIx::BuyV2.fees_on_curve());
        assert!(!CurveTradeIx::SellLegacy.fees_on_curve());
    }
}
