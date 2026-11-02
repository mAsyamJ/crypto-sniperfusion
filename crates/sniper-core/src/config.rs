//! Typed configuration with safe defaults. Field names carry explicit units
//! (lamports, bps, seconds). Mirrors Chainstack's YAML semantics where they
//! exist (see docs/CONFIG_MIGRATION.md at M4) but in typed TOML.

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::primitives::BasisPoints;

/// Live activation acknowledgment. Written deliberately by the operator;
/// its content is checked verbatim.
pub const OPERATOR_ACK_MAGIC: &str = "I AUTHORIZE LIVE TRADING FOR SOLANA SNIPER FUSION";

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse config file {path}: {message}")]
    Parse { path: PathBuf, message: String },
    #[error("config validation failed: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    Observe,
    Replay,
    Paper,
    Live,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    pub mode: RunMode,
    /// Master switch. Default and safe-mode value: false. This alone is never
    /// sufficient to go live (see sniper-runtime capability gates).
    #[serde(default)]
    pub live_trading_enabled: bool,
    /// Required to be Some and point at a valid ack file when
    /// `live_trading_enabled == true`.
    #[serde(default)]
    pub operator_ack_path: Option<PathBuf>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            mode: RunMode::Observe,
            live_trading_enabled: false,
            operator_ack_path: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitStrategy {
    TimeBased {
        wait_after_buy_secs: u64,
    },
    TakeProfitStopLoss {
        take_profit_bps: u16,
        stop_loss_bps: u16,
        max_hold_secs: u64,
        price_check_interval_secs: u64,
        max_exit_sell_attempts: u32,
    },
    Manual,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeConfig {
    /// SOL spend per buy, in lamports. Zero means "no buys" — safe default.
    pub buy_amount_lamports: u64,
    pub buy_slippage_bps: u16,
    pub sell_slippage_bps: u16,
    pub exit_strategy: ExitStrategy,
}

impl Default for TradeConfig {
    fn default() -> Self {
        Self {
            buy_amount_lamports: 0,
            buy_slippage_bps: 0,
            sell_slippage_bps: 0,
            exit_strategy: ExitStrategy::Manual,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FiltersConfig {
    #[serde(default)]
    pub allowed_venues: Vec<crate::venue::Venue>,
    #[serde(default)]
    pub max_token_age_ms: u64,
    #[serde(default)]
    pub marry_mode: bool,
    #[serde(default)]
    pub yolo_mode: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetriesConfig {
    #[serde(default)]
    pub max_attempts: u32,
    #[serde(default)]
    pub wait_after_buy_secs: u64,
    #[serde(default)]
    pub wait_before_new_token_secs: u64,
}

impl Default for RetriesConfig {
    fn default() -> Self {
        Self {
            max_attempts: 1,
            wait_after_buy_secs: 0,
            wait_before_new_token_secs: 0,
        }
    }
}

/// Hard risk ceilings. Enforced on every send (M3/M7); validated at load.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiskLimitsConfig {
    pub max_open_positions: u32,
    pub max_position_lamports: u64,
    pub max_aggregate_lamports: u64,
    pub max_daily_loss_lamports: u64,
    pub max_priority_fee_microlamports: u64,
    pub max_tip_lamports: u64,
    pub max_pending_orders: u32,
    pub max_staleness_slots: u32,
}

#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    #[serde(default)]
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub trade: TradeConfig,
    #[serde(default)]
    pub filters: FiltersConfig,
    #[serde(default)]
    pub retries: RetriesConfig,
    #[serde(default)]
    pub risk: RiskLimitsConfig,
}

impl AppConfig {
    /// Zero-spend, live-off configuration. The only config safe to construct
    /// without deliberate intent.
    pub fn safe_default() -> Self {
        Self::default()
    }

    /// Load and validate a TOML config file.
    pub fn load_toml(path: &Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let cfg: AppConfig = toml::from_str(&text).map_err(|e| ConfigError::Parse {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Cross-field validation. Fails closed on unsafe combinations.
    pub fn validate(&self) -> Result<(), ConfigError> {
        BasisPoints::new(self.trade.buy_slippage_bps)
            .map_err(|e| ConfigError::Validation(e.to_string()))?;
        BasisPoints::new(self.trade.sell_slippage_bps)
            .map_err(|e| ConfigError::Validation(e.to_string()))?;

        if let ExitStrategy::TakeProfitStopLoss {
            max_exit_sell_attempts,
            ..
        } = &self.trade.exit_strategy
        {
            if *max_exit_sell_attempts < 1 {
                return Err(ConfigError::Validation(
                    "max_exit_sell_attempts must be >= 1".into(),
                ));
            }
        }

        if self.runtime.live_trading_enabled {
            if self.runtime.mode != RunMode::Live {
                return Err(ConfigError::Validation(
                    "live_trading_enabled=true requires runtime.mode = live".into(),
                ));
            }
            let ack_path = self.runtime.operator_ack_path.as_ref().ok_or_else(|| {
                ConfigError::Validation(
                    "live_trading_enabled=true requires operator_ack_path".into(),
                )
            })?;
            let ack = std::fs::read_to_string(ack_path).map_err(|source| ConfigError::Io {
                path: ack_path.clone(),
                source,
            })?;
            if ack.trim() != OPERATOR_ACK_MAGIC {
                return Err(ConfigError::Validation(format!(
                    "operator ack file {ack_path:?} does not contain the required authorization statement"
                )));
            }
            // A live config with zero limits could never trade; refuse early so
            // misconfiguration is visible at load, not at first order.
            if self.risk.max_position_lamports == 0 || self.risk.max_open_positions == 0 {
                return Err(ConfigError::Validation(
                    "live config requires nonzero risk.max_position_lamports and risk.max_open_positions".into(),
                ));
            }
            if self.trade.buy_amount_lamports == 0 {
                return Err(ConfigError::Validation(
                    "live config requires nonzero trade.buy_amount_lamports".into(),
                ));
            }
        } else if self.runtime.mode == RunMode::Live {
            return Err(ConfigError::Validation(
                "runtime.mode = live requires live_trading_enabled = true".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_defaults_are_inert() {
        let cfg = AppConfig::safe_default();
        cfg.validate().expect("safe default validates");
        assert!(!cfg.runtime.live_trading_enabled);
        assert_eq!(cfg.trade.buy_amount_lamports, 0);
        assert_eq!(cfg.risk.max_position_lamports, 0);
        assert_eq!(cfg.risk.max_open_positions, 0);
    }

    #[test]
    fn unknown_keys_rejected() {
        let bad = "[runtime]\nmode = \"observe\"\nbogus_key = 1\n";
        let err = toml::from_str::<AppConfig>(bad).unwrap_err();
        assert!(err.to_string().contains("bogus_key"), "got: {err}");
    }

    #[test]
    fn live_without_ack_rejected() {
        let cfg = toml::from_str::<AppConfig>(
            "[runtime]\nmode = \"live\"\nlive_trading_enabled = true\n",
        )
        .unwrap();
        assert!(matches!(cfg.validate(), Err(ConfigError::Validation(_))));
    }

    #[test]
    fn live_with_valid_ack_and_limits_passes() {
        let dir = tempfile::tempdir().unwrap();
        let ack = dir.path().join("ack.txt");
        std::fs::write(&ack, OPERATOR_ACK_MAGIC).unwrap();
        let text = format!(
            "[runtime]\nmode = \"live\"\nlive_trading_enabled = true\noperator_ack_path = \"{}\"\n\
             [trade]\nbuy_amount_lamports = 1000000\nbuy_slippage_bps = 1000\nsell_slippage_bps = 1000\nexit_strategy = \"manual\"\n\
             [risk]\nmax_open_positions = 1\nmax_position_lamports = 2000000\nmax_aggregate_lamports = 2000000\n\
             max_daily_loss_lamports = 1000000\nmax_priority_fee_microlamports = 1000000\nmax_tip_lamports = 0\nmax_pending_orders = 1\nmax_staleness_slots = 5\n",
            ack.display()
        );
        let cfg: AppConfig = toml::from_str(&text).unwrap();
        cfg.validate().expect("complete live config validates");
    }

    #[test]
    fn slippage_over_100pct_rejected() {
        let cfg = toml::from_str::<AppConfig>(
            "[trade]\nbuy_amount_lamports = 1000000\nbuy_slippage_bps = 20000\nsell_slippage_bps = 100\nexit_strategy = \"manual\"\n",
        )
        .unwrap();
        assert!(matches!(cfg.validate(), Err(ConfigError::Validation(_))));
    }

    #[test]
    fn load_toml_missing_file_is_typed_error() {
        let err = AppConfig::load_toml(Path::new("/nonexistent/config.toml")).unwrap_err();
        assert!(matches!(err, ConfigError::Io { .. }));
    }
}
