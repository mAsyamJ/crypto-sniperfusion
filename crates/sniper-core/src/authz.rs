//! Risk authorization: the typed gate every money-moving order must pass
//! (H0.1 trust boundary).
//!
//! Two-type discipline:
//! - `RiskAuthorization` is a PUBLIC, serializable **evidence DTO**. Anyone
//!   can build or deserialize one — that is harmless, because it grants
//!   nothing. It exists for journals, logs, and audits.
//! - `AuthorizedOrder` is the ONLY execution currency. It is constructed
//!   exclusively by `RiskEngine::authorize` (private field, no
//!   `Deserialize`), and execution re-validates it against concrete
//!   `OrderFacts` at build time. A forged DTO is structurally unable to
//!   become an order: `accept_for_execution` always rejects.
//!
//! Strategies, copy-trading adapters, and emergency exits all route through
//! the engine; bypass is unrepresentable.

use solana_sdk::pubkey::Pubkey;
use thiserror::Error;

use crate::primitives::{BasisPoints, Lamports, Microlamports};
use crate::venue::Venue;

/// Target chain. SniperFusion is chain-agnostic by design; Solana is the
/// only implemented chain today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Chain {
    Solana,
    Unknown,
}

/// Identifies the strategy requesting the trade (feature-flagged experiments
/// carry separate P&L, so identity is load-bearing).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct StrategyId(pub String);

#[derive(Debug, Error, PartialEq)]
pub enum AuthzError {
    #[error("authorization window invalid: valid_from {valid_from} >= valid_to {valid_to}")]
    InvalidWindow { valid_from: u64, valid_to: u64 },
    #[error("authorization window overflows slot arithmetic")]
    WindowOverflow,
    #[error("authorization expired at slot {current} (valid_to {valid_to})")]
    Expired { current: u64, valid_to: u64 },
    #[error("authorization not yet valid at slot {current} (valid_from {valid_from})")]
    NotYetValid { current: u64, valid_from: u64 },
    #[error("chain mismatch: authorized {authorized:?}, order {requested:?}")]
    ChainMismatch { authorized: Chain, requested: Chain },
    #[error("wallet mismatch: authorized {authorized}, order {requested}")]
    WalletMismatch {
        authorized: Pubkey,
        requested: Pubkey,
    },
    #[error("mint mismatch: authorized {authorized}, order {requested}")]
    MintMismatch {
        authorized: Pubkey,
        requested: Pubkey,
    },
    #[error("quote mint mismatch: authorized {authorized}, order {requested}")]
    QuoteMintMismatch {
        authorized: Pubkey,
        requested: Pubkey,
    },
    #[error("venue mismatch: authorized {authorized:?}, order {requested:?}")]
    VenueMismatch { authorized: Venue, requested: Venue },
    #[error("spend limit exceeded: limit {limit} lamports, requested {requested} lamports")]
    SpendLimitExceeded { limit: u64, requested: u64 },
    #[error("CU price cap exceeded: cap {cap} microlamports, requested {requested}")]
    CuPriceExceeded { cap: u64, requested: u64 },
    #[error("CU limit exceeded: limit {limit}, requested {requested}")]
    CuLimitExceeded { limit: u32, requested: u32 },
    #[error("total network fee exceeded: cap {cap} lamports, requested {requested}")]
    NetworkFeeExceeded { cap: u64, requested: u64 },
    #[error("tip cap exceeded: cap {cap} lamports, requested {requested}")]
    TipCapExceeded { cap: u64, requested: u64 },
    #[error("slippage bound exceeded: bound {bound} bps, order {requested} bps")]
    SlippageExceeded { bound: u16, requested: u16 },
    #[error("price impact bound exceeded: bound {bound} bps, order {requested} bps")]
    PriceImpactExceeded { bound: u16, requested: u16 },
    #[error("risk engine is disabled (kill switch or safe mode)")]
    EngineDisabled,
}

/// The evidence DTO: everything an authorization bound, for journals and
/// audits. PUBLIC and deserializable ON PURPOSE — it is not a capability.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RiskAuthorization {
    pub strategy_id: StrategyId,
    pub chain: Chain,
    pub wallet: Pubkey,
    pub mint: Pubkey,
    pub quote_mint: Pubkey,
    pub venue: Venue,
    pub spend_limit: Lamports,
    pub cu_price_cap: Microlamports,
    pub cu_limit: u32,
    pub max_network_fee_lamports: Lamports,
    pub tip_cap: Lamports,
    pub slippage_bps: BasisPoints,
    pub max_price_impact_bps: BasisPoints,
    pub valid_from_slot: u64,
    pub valid_to_slot: u64,
    pub nonce: u64,
    /// Digest of the screening facts that justified this authorization.
    pub risk_evidence: u64,
    pub remaining_budget: Lamports,
}

/// The execution currency. Constructed ONLY by `RiskEngine::authorize`
/// (private field); deliberately NOT `Deserialize` — a serialized order can
/// never be revived into an executable one.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthorizedOrder {
    inner: RiskAuthorization,
}

impl AuthorizedOrder {
    // Accessors only: execution reads bounds, it cannot forge them.
    pub fn strategy_id(&self) -> &StrategyId {
        &self.inner.strategy_id
    }
    pub fn chain(&self) -> Chain {
        self.inner.chain
    }
    pub fn wallet(&self) -> Pubkey {
        self.inner.wallet
    }
    pub fn mint(&self) -> Pubkey {
        self.inner.mint
    }
    pub fn quote_mint(&self) -> Pubkey {
        self.inner.quote_mint
    }
    pub fn venue(&self) -> Venue {
        self.inner.venue
    }
    pub fn spend_limit(&self) -> Lamports {
        self.inner.spend_limit
    }
    pub fn cu_price_cap(&self) -> Microlamports {
        self.inner.cu_price_cap
    }
    pub fn cu_limit(&self) -> u32 {
        self.inner.cu_limit
    }
    pub fn max_network_fee_lamports(&self) -> Lamports {
        self.inner.max_network_fee_lamports
    }
    pub fn tip_cap(&self) -> Lamports {
        self.inner.tip_cap
    }
    pub fn slippage_bps(&self) -> BasisPoints {
        self.inner.slippage_bps
    }
    pub fn max_price_impact_bps(&self) -> BasisPoints {
        self.inner.max_price_impact_bps
    }
    pub fn nonce(&self) -> u64 {
        self.inner.nonce
    }
    pub fn risk_evidence(&self) -> u64 {
        self.inner.risk_evidence
    }
    pub fn remaining_budget(&self) -> Lamports {
        self.inner.remaining_budget
    }

    /// Read-only evidence view (the DTO), e.g. for journaling.
    pub fn evidence(&self) -> &RiskAuthorization {
        &self.inner
    }

    /// Re-validate against the CONCRETE facts of the order being built.
    /// Execution calls this at build time — issuance passing is not send
    /// passing. A pass here is the authorization to spend.
    pub fn revalidate(&self, facts: &OrderFacts) -> Result<(), AuthzError> {
        let a = &self.inner;
        if a.valid_from_slot >= a.valid_to_slot {
            return Err(AuthzError::InvalidWindow {
                valid_from: a.valid_from_slot,
                valid_to: a.valid_to_slot,
            });
        }
        if facts.current_slot >= a.valid_to_slot {
            return Err(AuthzError::Expired {
                current: facts.current_slot,
                valid_to: a.valid_to_slot,
            });
        }
        if facts.current_slot < a.valid_from_slot {
            return Err(AuthzError::NotYetValid {
                current: facts.current_slot,
                valid_from: a.valid_from_slot,
            });
        }
        if a.chain != facts.chain {
            return Err(AuthzError::ChainMismatch {
                authorized: a.chain,
                requested: facts.chain,
            });
        }
        if a.wallet != facts.wallet {
            return Err(AuthzError::WalletMismatch {
                authorized: a.wallet,
                requested: facts.wallet,
            });
        }
        if a.mint != facts.mint {
            return Err(AuthzError::MintMismatch {
                authorized: a.mint,
                requested: facts.mint,
            });
        }
        if a.quote_mint != facts.quote_mint {
            return Err(AuthzError::QuoteMintMismatch {
                authorized: a.quote_mint,
                requested: facts.quote_mint,
            });
        }
        if a.venue != facts.venue {
            return Err(AuthzError::VenueMismatch {
                authorized: a.venue,
                requested: facts.venue,
            });
        }
        if facts.spend.0 > a.spend_limit.0 {
            return Err(AuthzError::SpendLimitExceeded {
                limit: a.spend_limit.0,
                requested: facts.spend.0,
            });
        }
        if facts.cu_price.0 > a.cu_price_cap.0 {
            return Err(AuthzError::CuPriceExceeded {
                cap: a.cu_price_cap.0,
                requested: facts.cu_price.0,
            });
        }
        if facts.cu_limit > a.cu_limit {
            return Err(AuthzError::CuLimitExceeded {
                limit: a.cu_limit,
                requested: facts.cu_limit,
            });
        }
        if facts.network_fee_lamports.0 > a.max_network_fee_lamports.0 {
            return Err(AuthzError::NetworkFeeExceeded {
                cap: a.max_network_fee_lamports.0,
                requested: facts.network_fee_lamports.0,
            });
        }
        if facts.tip.0 > a.tip_cap.0 {
            return Err(AuthzError::TipCapExceeded {
                cap: a.tip_cap.0,
                requested: facts.tip.0,
            });
        }
        if facts.slippage_bps.0 > a.slippage_bps.0 {
            return Err(AuthzError::SlippageExceeded {
                bound: a.slippage_bps.0,
                requested: facts.slippage_bps.0,
            });
        }
        if facts.price_impact_bps.0 > a.max_price_impact_bps.0 {
            return Err(AuthzError::PriceImpactExceeded {
                bound: a.max_price_impact_bps.0,
                requested: facts.price_impact_bps.0,
            });
        }
        Ok(())
    }
}

/// Concrete facts of the order being built, filled by execution from the
/// actual transaction about to be signed — never from strategy estimates.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderFacts {
    pub current_slot: u64,
    pub chain: Chain,
    pub wallet: Pubkey,
    pub mint: Pubkey,
    pub quote_mint: Pubkey,
    pub venue: Venue,
    pub spend: Lamports,
    pub cu_price: Microlamports,
    pub cu_limit: u32,
    pub network_fee_lamports: Lamports,
    pub tip: Lamports,
    pub slippage_bps: BasisPoints,
    pub price_impact_bps: BasisPoints,
}

/// What a strategy asks for. Bound fields become the authorization; the
/// engine may tighten them, never loosen.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthzRequest {
    pub strategy_id: StrategyId,
    pub chain: Chain,
    pub wallet: Pubkey,
    pub mint: Pubkey,
    pub quote_mint: Pubkey,
    pub venue: Venue,
    pub spend: Lamports,
    pub cu_price_cap: Microlamports,
    pub cu_limit: u32,
    pub max_network_fee_lamports: Lamports,
    pub tip: Lamports,
    pub slippage_bps: BasisPoints,
    pub price_impact_bps: BasisPoints,
    pub valid_from_slot: u64,
    pub valid_for_slots: u64,
    pub risk_evidence: u64,
    pub remaining_budget: Lamports,
}

/// The issuance boundary. Held by the risk-engine task. A disabled engine
/// (kill switch / safe mode) issues nothing.
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

    pub fn authorize(&mut self, req: AuthzRequest) -> Result<AuthorizedOrder, AuthzError> {
        if !self.enabled {
            return Err(AuthzError::EngineDisabled);
        }
        if req.valid_for_slots == 0 {
            return Err(AuthzError::InvalidWindow {
                valid_from: req.valid_from_slot,
                valid_to: req.valid_from_slot,
            });
        }
        // Checked window arithmetic: overflow is a typed rejection, never a
        // wrapped (fail-open-looking) small window.
        let valid_to = req
            .valid_from_slot
            .checked_add(req.valid_for_slots)
            .ok_or(AuthzError::WindowOverflow)?;
        if req.valid_from_slot >= valid_to {
            return Err(AuthzError::InvalidWindow {
                valid_from: req.valid_from_slot,
                valid_to,
            });
        }
        let authz = RiskAuthorization {
            strategy_id: req.strategy_id,
            chain: req.chain,
            wallet: req.wallet,
            mint: req.mint,
            quote_mint: req.quote_mint,
            venue: req.venue,
            spend_limit: req.spend,
            cu_price_cap: req.cu_price_cap,
            cu_limit: req.cu_limit,
            max_network_fee_lamports: req.max_network_fee_lamports,
            tip_cap: req.tip,
            slippage_bps: req.slippage_bps,
            max_price_impact_bps: req.price_impact_bps,
            valid_from_slot: req.valid_from_slot,
            valid_to_slot: valid_to,
            nonce: self.next_nonce,
            risk_evidence: req.risk_evidence,
            remaining_budget: req.remaining_budget,
        };
        // Nonce rollover: skip 0 so an issued nonce is never zero and never
        // reused across the u64 boundary.
        self.next_nonce = if self.next_nonce == u64::MAX {
            1
        } else {
            self.next_nonce + 1
        };
        Ok(AuthorizedOrder { inner: authz })
    }

    /// The ONLY bridge from DTO to execution currency — and it never grants.
    /// Serialized/hand-built authorizations are evidence, not permissions.
    pub fn accept_for_execution(&self, _dto: &RiskAuthorization) -> Option<AuthorizedOrder> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> AuthzRequest {
        AuthzRequest {
            strategy_id: StrategyId("momentum_or_migration".into()),
            chain: Chain::Solana,
            wallet: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            quote_mint: Pubkey::new_unique(),
            venue: Venue::PumpFun,
            spend: Lamports(100_000_000),
            cu_price_cap: Microlamports(1_000_000),
            cu_limit: 150_000,
            max_network_fee_lamports: Lamports(2_000_000),
            tip: Lamports(0),
            slippage_bps: BasisPoints(1_000),
            price_impact_bps: BasisPoints(2_000),
            valid_from_slot: 300,
            valid_for_slots: 30,
            risk_evidence: 42,
            remaining_budget: Lamports(1_000_000_000),
        }
    }

    fn facts(order: &AuthorizedOrder, slot: u64) -> OrderFacts {
        OrderFacts {
            current_slot: slot,
            chain: Chain::Solana,
            wallet: order.wallet(),
            mint: order.mint(),
            quote_mint: order.quote_mint(),
            venue: Venue::PumpFun,
            spend: order.spend_limit(),
            cu_price: order.cu_price_cap(),
            cu_limit: order.cu_limit(),
            network_fee_lamports: order.max_network_fee_lamports(),
            tip: Lamports(0),
            slippage_bps: order.slippage_bps(),
            price_impact_bps: order.max_price_impact_bps(),
        }
    }

    #[test]
    fn authorize_and_revalidate_roundtrip() {
        let mut engine = RiskEngine::new(true);
        let order = engine.authorize(req()).expect("authz");
        assert_eq!(order.nonce(), 1);
        order.revalidate(&facts(&order, 310)).expect("valid at 310");
    }

    #[test]
    fn window_overflow_is_typed_never_wrapped() {
        let mut engine = RiskEngine::new(true);
        let mut r = req();
        r.valid_from_slot = u64::MAX - 5;
        r.valid_for_slots = 10;
        assert_eq!(engine.authorize(r), Err(AuthzError::WindowOverflow));
    }

    #[test]
    fn max_representable_window_is_accepted() {
        let mut engine = RiskEngine::new(true);
        let mut r = req();
        r.valid_from_slot = 0;
        r.valid_for_slots = u64::MAX;
        assert!(engine.authorize(r).is_ok());
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
    fn nonce_rollover_skips_zero() {
        let mut engine = RiskEngine::new(true);
        engine.next_nonce = u64::MAX; // internal counter reachable in-unit
        let maxed = engine.authorize(req()).unwrap();
        assert_eq!(maxed.nonce(), u64::MAX);
        let next = engine.authorize(req()).unwrap();
        assert_eq!(next.nonce(), 1, "rollover skips zero");
        let third = engine.authorize(req()).unwrap();
        assert_eq!(third.nonce(), 2);
    }

    #[test]
    fn disabled_engine_refuses_everything() {
        let mut engine = RiskEngine::new(false);
        assert_eq!(engine.authorize(req()), Err(AuthzError::EngineDisabled));
    }

    #[test]
    fn expiry_and_notyet_valid_enforced() {
        let mut engine = RiskEngine::new(true);
        let order = engine.authorize(req()).unwrap(); // window 300..330
        assert!(matches!(
            order.revalidate(&facts(&order, 331)),
            Err(AuthzError::Expired { .. })
        ));
        assert!(matches!(
            order.revalidate(&facts(&order, 299)),
            Err(AuthzError::NotYetValid { .. })
        ));
    }

    #[test]
    fn evidence_view_matches_issued_bounds() {
        let mut engine = RiskEngine::new(true);
        let order = engine.authorize(req()).unwrap();
        let evidence = order.evidence();
        assert_eq!(evidence.nonce, order.nonce());
        assert_eq!(evidence.spend_limit, order.spend_limit());
        assert_eq!(evidence.cu_limit, order.cu_limit());
        assert_eq!(evidence.risk_evidence, order.risk_evidence());
        // The DTO serializes (evidence path)…
        let json = serde_json::to_string(evidence).unwrap();
        assert!(json.contains("\"nonce\""));
        // …and reviving it grants nothing.
        let revived: RiskAuthorization = serde_json::from_str(&json).unwrap();
        assert_eq!(engine.accept_for_execution(&revived), None);
    }
}
