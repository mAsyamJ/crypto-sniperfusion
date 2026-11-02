//! External construction of AuthorizedOrder must not compile (private seal).
use sniper_core::authz::{AuthorizedOrder, RiskAuthorization};
use sniper_core::primitives::{BasisPoints, Lamports, Microlamports};
use sniper_core::venue::Venue;

fn main() {
    let dto = RiskAuthorization {
        strategy_id: sniper_core::authz::StrategyId("attacker".into()),
        chain: sniper_core::authz::Chain::Solana,
        wallet: solana_sdk::pubkey::Pubkey::new_unique(),
        mint: solana_sdk::pubkey::Pubkey::new_unique(),
        quote_mint: solana_sdk::pubkey::Pubkey::default(),
        venue: Venue::PumpFun,
        spend_limit: Lamports(u64::MAX),
        cu_price_cap: Microlamports(u64::MAX),
        cu_limit: u32::MAX,
        max_network_fee_lamports: Lamports(u64::MAX),
        tip_cap: Lamports(u64::MAX),
        slippage_bps: BasisPoints(10_000),
        max_price_impact_bps: BasisPoints(10_000),
        valid_from_slot: 0,
        valid_to_slot: u64::MAX,
        nonce: 1,
        risk_evidence: 0,
        remaining_budget: Lamports(u64::MAX),
    };
    // The forgery attempt: promote the DTO to the execution currency.
    let _order = AuthorizedOrder { inner: dto };
}
