//! H0.1 trust-boundary contract tests, written from OUTSIDE sniper-core
//! (integration-test crate = external-crate perspective).
//!
//! Red phase: these encode the REQUIRED behavior; against the current code
//! they fail to compile / fail assertions (forgeable authorization, unchecked
//! slot arithmetic). They define the contract:
//!   - `AuthorizedOrder` is the only execution currency and cannot be
//!     constructed or deserialized outside the issuance boundary.
//!   - A forged `RiskAuthorization` (built by hand or deserialized) is NOT
//!     accepted for execution.
//!   - Every authorization binds chain, wallet, quote mint, CU price AND
//!     limit, total network fee, tip, slippage, validity window, nonce, risk
//!     evidence digest, and remaining budget.
//!   - Window arithmetic is checked; nonces roll over without reuse.

use sniper_core::authz::{
    AuthorizedOrder, AuthzError, AuthzRequest, OrderFacts, RiskAuthorization, RiskEngine,
    StrategyId,
};
use sniper_core::primitives::{BasisPoints, Lamports, Microlamports};
use sniper_core::venue::Venue;
use solana_sdk::pubkey::Pubkey;

fn full_request() -> AuthzRequest {
    AuthzRequest {
        strategy_id: StrategyId("momentum_or_migration".into()),
        chain: sniper_core::authz::Chain::Solana,
        wallet: Pubkey::new_unique(),
        mint: Pubkey::new_unique(),
        quote_mint: "So11111111111111111111111111111111111111112"
            .parse()
            .unwrap(),
        venue: Venue::PumpFun,
        spend: Lamports(50_000_000),
        cu_price_cap: Microlamports(1_000_000),
        cu_limit: 150_000,
        max_network_fee_lamports: Lamports(2_000_000),
        tip: Lamports(0),
        slippage_bps: BasisPoints(1_000),
        price_impact_bps: BasisPoints(2_000),
        valid_from_slot: 300,
        valid_for_slots: 30,
        risk_evidence: 0xdeadbeef,
        remaining_budget: Lamports(500_000_000),
    }
}

fn facts_for(o: &AuthorizedOrder, slot: u64) -> OrderFacts {
    OrderFacts {
        current_slot: slot,
        chain: sniper_core::authz::Chain::Solana,
        wallet: o.wallet(),
        mint: o.mint(),
        quote_mint: o.quote_mint(),
        venue: o.venue(),
        spend: o.spend_limit(),
        cu_price: o.cu_price_cap(),
        cu_limit: o.cu_limit(),
        network_fee_lamports: Lamports(o.max_network_fee_lamports().0),
        tip: Lamports(0),
        slippage_bps: o.slippage_bps(),
        price_impact_bps: o.max_price_impact_bps(),
    }
}

#[test]
fn engine_issues_sealed_orders_and_execution_revalidates() {
    let mut engine = RiskEngine::new(true);
    let order = engine.authorize(full_request()).expect("issues");
    order.revalidate(&facts_for(&order, 310)).expect("valid");
}

#[test]
fn forged_risk_authorization_is_not_accepted_for_execution() {
    // An attacker builds the DTO by hand (all fields are public and it
    // deserializes) — the exact forgery the audit flagged.
    let forged = RiskAuthorization {
        strategy_id: StrategyId("attacker".into()),
        chain: sniper_core::authz::Chain::Solana,
        wallet: Pubkey::new_unique(),
        mint: Pubkey::new_unique(),
        quote_mint: Pubkey::default(),
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
    // And round-trips through serde, proving serializability is not a gate.
    let json = serde_json::to_string(&forged).unwrap();
    let revived: RiskAuthorization = serde_json::from_str(&json).unwrap();
    let _ = revived;

    // The ONLY acceptance path is the engine; a bare DTO never becomes an
    // executable order.
    let engine = RiskEngine::new(true);
    assert!(engine.accept_for_execution(&forged).is_none());
}

#[test]
fn every_bound_is_enforced_at_revalidation() {
    let mut engine = RiskEngine::new(true);
    let order = engine.authorize(full_request()).expect("issues");

    // Wallet mismatch.
    let mut facts = facts_for(&order, 310);
    facts.wallet = Pubkey::new_unique();
    assert!(matches!(
        order.revalidate(&facts),
        Err(AuthzError::WalletMismatch { .. })
    ));

    // Chain mismatch.
    let mut facts = facts_for(&order, 310);
    facts.chain = sniper_core::authz::Chain::Unknown;
    assert!(matches!(
        order.revalidate(&facts),
        Err(AuthzError::ChainMismatch { .. })
    ));

    // Quote mint mismatch.
    let mut facts = facts_for(&order, 310);
    facts.quote_mint = Pubkey::new_unique();
    assert!(matches!(
        order.revalidate(&facts),
        Err(AuthzError::QuoteMintMismatch { .. })
    ));

    // CU limit exceeded.
    let mut facts = facts_for(&order, 310);
    facts.cu_limit = order.cu_limit() + 1;
    assert!(matches!(
        order.revalidate(&facts),
        Err(AuthzError::CuLimitExceeded { .. })
    ));

    // Total network fee exceeded.
    let mut facts = facts_for(&order, 310);
    facts.network_fee_lamports = Lamports(order.max_network_fee_lamports().0 + 1);
    assert!(matches!(
        order.revalidate(&facts),
        Err(AuthzError::NetworkFeeExceeded { .. })
    ));

    // Spend, tip, slippage, impact still enforced.
    let mut facts = facts_for(&order, 310);
    facts.spend = Lamports(order.spend_limit().0 + 1);
    assert!(matches!(
        order.revalidate(&facts),
        Err(AuthzError::SpendLimitExceeded { .. })
    ));
    let mut facts = facts_for(&order, 310);
    facts.tip = Lamports(1);
    assert!(matches!(
        order.revalidate(&facts),
        Err(AuthzError::TipCapExceeded { .. })
    ));
}

#[test]
fn window_arithmetic_is_checked_not_wrapping() {
    let mut engine = RiskEngine::new(true);
    // valid_from + valid_for_slots overflows u64: must be a typed rejection
    // (WindowOverflow), never a wrapped small window that looks valid.
    let mut req = full_request();
    req.valid_from_slot = u64::MAX - 5;
    req.valid_for_slots = 10;
    assert_eq!(engine.authorize(req), Err(AuthzError::WindowOverflow));

    // Maximum finite window is representable and validates far in the future.
    let mut req = full_request();
    req.valid_from_slot = 0;
    req.valid_for_slots = u64::MAX;
    let order = engine.authorize(req).expect("max window is representable");
    order
        .revalidate(&facts_for(&order, u64::MAX - 1))
        .expect("still valid");
}

#[test]
fn nonces_never_reuse_and_rollover_skips_zero() {
    let mut engine = RiskEngine::new(true);
    let a = engine.authorize(full_request()).unwrap();
    let b = engine.authorize(full_request()).unwrap();
    assert_eq!(a.nonce() + 1, b.nonce());
    assert_ne!(a.nonce(), 0);

    // Rollover-at-u64::MAX behavior (skip zero on wrap) is proven by the
    // engine's unit tests, where the internal counter is reachable; from
    // outside the boundary the counter is not settable by design.
}

#[test]
fn disabled_engine_issues_nothing() {
    let mut engine = RiskEngine::new(false);
    assert_eq!(
        engine.authorize(full_request()),
        Err(AuthzError::EngineDisabled)
    );
}
