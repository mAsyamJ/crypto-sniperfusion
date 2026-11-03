//! H6-1 future outcome labels — physically and logically separate from
//! strategy-visible features.
//!
//! Separation is structural: labels live in their own module, serialize into
//! their own dataset namespace, and are never part of the observation type
//! passed to `Strategy::decide` (H7). Research tooling may join them for
//! evaluation AFTER decisions are frozen; a strategy can never read them.

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Dataset namespace selector. Distinct directories keep feature and label
/// files from being conflated at rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetNamespace {
    Features,
    Labels,
}

impl DatasetNamespace {
    pub fn directory(self) -> &'static str {
        match self {
            DatasetNamespace::Features => "features",
            DatasetNamespace::Labels => "labels",
        }
    }
}

/// Future-outcome labels for one mint, attached after the observation window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutcomeLabels {
    pub mint: Pubkey,
    /// Whether the bonding curve completed and migrated.
    pub graduated: Option<bool>,
    pub graduation_slot: Option<u64>,
    /// Max price observed over the labeled window (lamports per whole token).
    pub max_observed_price_lamports_per_token: Option<u64>,
    /// Lifecycle phase at window end, for research attribution only.
    pub window_end_phase: Option<String>,
}
