//! Risk authorization: the single typed gate every money-moving order must
//! pass. Strategies, copy-trading adapters, and emergency exits all construct
//! orders through `RiskEngine`; execution accepts only `RiskAuthorization`
//! values it can validate against the order facts at build time.

use solana_sdk::pubkey::Pubkey;
use thiserror::Error;

use crate::primitives::{BasisPoints, Lamports, Microlamports};
use crate::venue::Venue;

/// Identifies the strategy requesting the trade (feature-flagged experiments
/// carry separate P&L, so identity is load-bearing).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct StrategyId(pub String);

#[derive(Debug, Error, PartialEq)]
pub enum AuthzError {
    #[error("authorization window invalid: valid_from {valid_from} >= valid_to {valid_to}")]
    InvalidWindow { valid_from: u64, valid_to: u64 },
    #[error("authorization expired at slot {current} (valid_to {valid_to})")]
    Expired { current: u64, valid_to: u64 },
    #[error("authorization not yet valid at slot {current} (valid_from {valid_from})")]
    NotYetValid { current: u64, valid_from: u64 },
    #[error("mint mismatch: authorized {authorized}, order {requested}")]
    MintMismatch {
        authorized: Pubkey,
        requested: Pubkey,
    },
    #[error("venue mismatch: authorized {authorized:?}, order {requested:?}")]
    VenueMismatch { authorized: Venue, requested: Venue },
    #[error("spend limit exceeded: limit {limit} lamports, requested {requested} lamports")]
    SpendLimitExceeded { limit: u64, requested: u64 },
    #[error("priority fee cap exceeded: cap {cap} microlamports, requested {requested}")]
    FeeCapExceeded { cap: u64, requested: u64 },
    #[error("tip cap exceeded: cap {cap} lamports, requested {requested}")]
    TipCapExceeded { cap: u64, requested: u64 },
    #[error("slippage bound exceeded: bound {bound} bps, order {requested} bps")]
    SlippageExceeded { bound: u16, requested: u16 },
    #[error("price impact bound exceeded: bound {bound} bps, order {requested} bps")]
    PriceImpactExceeded { bound: u16, requested: u16 },
    #[error("risk engine is disabled (kill switch or safe mode)")]
    EngineDisabled,
}

/// The bound, signed by the risk engine, that execution must re-validate.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RiskAuthorization {
    pub strategy_id: StrategyId,
    pub mint: Pubkey,
    pub venue: Venue,
    pub spend_limit: Lamports,
    pub fee_cap: Microlamports,
    pub tip_cap: Lamports,
    pub slippage_bps: BasisPoints,
    pub max_price_impact_bps: BasisPoints,
    pub valid_from_slot: u64,
    pub valid_to_slot: u64,
    pub nonce: u64,
}

impl RiskAuthorization {
    /// Validate this authorization against concrete order facts. Execution
    /// calls this at build time — a pass at issuance is not a pass at send.
    pub fn validate(&self, facts: &OrderFacts) -> Result<(), AuthzError> {
        if self.valid_from_slot >= self.valid_to_slot {
            return Err(AuthzError::InvalidWindow {
                valid_from: self.valid_from_slot,
                valid_to: self.valid_to_slot,
            });
        }
        if facts.current_slot >= self.valid_to_slot {
            return Err(AuthzError::Expired {
                current: facts.current_slot,
                valid_to: self.valid_to_slot,
            });
        }
        if facts.current_slot < self.valid_from_slot {
            return Err(AuthzError::NotYetValid {
                current: facts.current_slot,
                valid_from: self.valid_from_slot,
            });
        }
        if self.mint != facts.mint {
            return Err(AuthzError::MintMismatch {
                authorized: self.mint,
                requested: facts.mint,
            });
        }
        if self.venue != facts.venue {
            return Err(AuthzError::VenueMismatch {
                authorized: self.venue,
                requested: facts.venue,
            });
        }
        if facts.spend.0 > self.spend_limit.0 {
            return Err(AuthzError::SpendLimitExceeded {
                limit: self.spend_limit.0,
                requested: facts.spend.0,
            });
        }
        if facts.fee.0 > self.fee_cap.0 {
            return Err(AuthzError::FeeCapExceeded {
                cap: self.fee_cap.0,
                requested: facts.fee.0,
            });
        }
        if facts.tip.0 > self.tip_cap.0 {
            return Err(AuthzError::TipCapExceeded {
                cap: self.tip_cap.0,
                requested: facts.tip.0,
            });
        }
        if facts.slippage_bps.0 > self.slippage_bps.0 {
            return Err(AuthzError::SlippageExceeded {
                bound: self.slippage_bps.0,
                requested: facts.slippage_bps.0,
            });
        }
        if facts.price_impact_bps.0 > self.max_price_impact_bps.0 {
            return Err(AuthzError::PriceImpactExceeded {
                bound: self.max_price_impact_bps.0,
                requested: facts.price_impact_bps.0,
            });
        }
        Ok(())
    }
}

/// Concrete facts of the order being built. Filled by execution from the
/// actual transaction it is about to sign — never from strategy estimates.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderFacts {
    pub current_slot: u64,
    pub mint: Pubkey,
    pub venue: Venue,
    pub spend: Lamports,
    pub fee: Microlamports,
    pub tip: Lamports,
    pub slippage_bps: BasisPoints,
    pub price_impact_bps: BasisPoints,
}

/// What a strategy asks for. Bound fields become the authorization; the engine
/// may tighten them, never loosen.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthzRequest {
    pub strategy_id: StrategyId,
    pub mint: Pubkey,
    pub venue: Venue,
    pub spend: Lamports,
    pub fee: Microlamports,
    pub tip: Lamports,
    pub slippage_bps: BasisPoints,
    pub price_impact_bps: BasisPoints,
    pub valid_from_slot: u64,
    pub valid_for_slots: u64,
}

/// The only constructor of `RiskAuthorization`. Held by the risk engine task.
/// A disabled engine (kill switch / safe mode) issues nothing.
#[derive(Debug)]
pub struct RiskEngine {
    enabled: bool,
    next_nonce: u64,
}

impl RiskEngine {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            next_nonce: 1,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn authorize(&mut self, req: AuthzRequest) -> Result<RiskAuthorization, AuthzError> {
        if !self.enabled {
            return Err(AuthzError::EngineDisabled);
        }
        if req.valid_for_slots == 0
            || req.valid_from_slot >= req.valid_from_slot + req.valid_for_slots
        {
            return Err(AuthzError::InvalidWindow {
                valid_from: req.valid_from_slot,
                valid_to: req.valid_from_slot + req.valid_for_slots,
            });
        }
        let authz = RiskAuthorization {
            strategy_id: req.strategy_id,
            mint: req.mint,
            venue: req.venue,
            spend_limit: req.spend,
            fee_cap: req.fee,
            tip_cap: req.tip,
            slippage_bps: req.slippage_bps,
            max_price_impact_bps: req.price_impact_bps,
            valid_from_slot: req.valid_from_slot,
            valid_to_slot: req.valid_from_slot + req.valid_for_slots,
            nonce: self.next_nonce,
        };
        self.next_nonce += 1;
        Ok(authz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> AuthzRequest {
        AuthzRequest {
            strategy_id: StrategyId("momentum_or_migration".into()),
            mint: Pubkey::new_unique(),
            venue: Venue::PumpFun,
            spend: Lamports(100_000_000),
            fee: Microlamports(1_000_000),
            tip: Lamports(0),
            slippage_bps: BasisPoints(1_000),
            price_impact_bps: BasisPoints(2_000),
            valid_from_slot: 300,
            valid_for_slots: 30,
        }
    }

    fn facts_for(authz: &RiskAuthorization, slot: u64) -> OrderFacts {
        OrderFacts {
            current_slot: slot,
            mint: authz.mint,
            venue: authz.venue,
            spend: authz.spend_limit,
            fee: authz.fee_cap,
            tip: Lamports(0),
            slippage_bps: authz.slippage_bps,
            price_impact_bps: authz.max_price_impact_bps,
        }
    }

    #[test]
    fn authorize_and_validate_roundtrip() {
        let mut engine = RiskEngine::new(true);
        let authz = engine.authorize(req()).expect("authz");
        assert_eq!(authz.nonce, 1);
        authz.validate(&facts_for(&authz, 310)).expect("valid");
    }

    #[test]
    fn nonces_increment() {
        let mut engine = RiskEngine::new(true);
        let a = engine.authorize(req()).unwrap();
        let b = engine.authorize(req()).unwrap();
        assert_eq!(a.nonce + 1, b.nonce);
    }

    #[test]
    fn disabled_engine_refuses_everything() {
        let mut engine = RiskEngine::new(false);
        assert_eq!(engine.authorize(req()), Err(AuthzError::EngineDisabled));
    }

    #[test]
    fn expiry_and_window_enforced() {
        let mut engine = RiskEngine::new(true);
        let authz = engine.authorize(req()).unwrap();
        assert!(matches!(
            authz.validate(&facts_for(&authz, 330)),
            Err(AuthzError::Expired { .. })
        ));
        assert!(matches!(
            authz.validate(&facts_for(&authz, 299)),
            Err(AuthzError::NotYetValid { .. })
        ));
    }

    #[test]
    fn zero_window_rejected_at_issuance() {
        let mut engine = RiskEngine::new(true);
        let mut r = req();
        r.valid_for_slots = 0;
        assert!(matches!(
            engine.authorize(r),
            Err(AuthzError::InvalidWindow { .. })
        ));
    }

    #[test]
    fn bound_violations_rejected() {
        let mut engine = RiskEngine::new(true);
        let authz = engine.authorize(req()).unwrap();

        let mut facts = facts_for(&authz, 310);
        facts.spend = Lamports(authz.spend_limit.0 + 1);
        assert!(matches!(
            authz.validate(&facts),
            Err(AuthzError::SpendLimitExceeded { .. })
        ));

        let mut facts = facts_for(&authz, 310);
        facts.mint = Pubkey::new_unique();
        assert!(matches!(
            authz.validate(&facts),
            Err(AuthzError::MintMismatch { .. })
        ));

        let mut facts = facts_for(&authz, 310);
        facts.venue = Venue::PumpSwap;
        assert!(matches!(
            authz.validate(&facts),
            Err(AuthzError::VenueMismatch { .. })
        ));

        let mut facts = facts_for(&authz, 310);
        facts.slippage_bps = BasisPoints(authz.slippage_bps.0 + 1);
        assert!(matches!(
            authz.validate(&facts),
            Err(AuthzError::SlippageExceeded { .. })
        ));

        let mut facts = facts_for(&authz, 310);
        facts.fee = Microlamports(authz.fee_cap.0 + 1);
        assert!(matches!(
            authz.validate(&facts),
            Err(AuthzError::FeeCapExceeded { .. })
        ));

        let mut facts = facts_for(&authz, 310);
        facts.tip = Lamports(1);
        assert!(matches!(
            authz.validate(&facts),
            Err(AuthzError::TipCapExceeded { .. })
        ));

        let mut facts = facts_for(&authz, 310);
        facts.price_impact_bps = BasisPoints(authz.max_price_impact_bps.0 + 1);
        assert!(matches!(
            authz.validate(&facts),
            Err(AuthzError::PriceImpactExceeded { .. })
        ));
    }
}
