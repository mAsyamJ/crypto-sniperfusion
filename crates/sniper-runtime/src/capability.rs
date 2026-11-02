//! Live capability acquisition — the four independent gates.
//!
//! Gate 1 (compile-time): the acquiring code only exists with the non-default
//! `live` cargo feature. Default builds return `LiveGateError::NotCompiledIn`.
//! Gate 2 (env): `LIVE_TRADING_ENABLED=true` in the process environment.
//! Gate 3 (operator): config validated in live mode, which requires the
//! operator acknowledgment file on disk.
//! Gate 4 (runtime health): kill switch clear.
//!
//! No other module may construct `LiveCapability`.

use std::path::PathBuf;

use sniper_core::config::AppConfig;
use thiserror::Error;

/// Proof that all live gates passed. Zero-sized; valuable because it is
/// unconstructible any other way. The `live`-feature signer/broadcaster
/// consume it.
#[derive(Debug, Clone, Copy)]
pub struct LiveCapability {
    _priv: (),
}

#[derive(Debug, Error)]
pub enum LiveGateError {
    #[error("gate 1 failed: live capability is not compiled into this build (rebuild with --features live)")]
    NotCompiledIn,
    #[error("gate 2 failed: LIVE_TRADING_ENABLED is not 'true' in the process environment")]
    EnvSwitchOff,
    #[error("gate 3 failed: operator authorization/config invalid: {0}")]
    OperatorAck(String),
    #[error("gate 4 failed: kill switch engaged: {0:?}")]
    KillSwitchEngaged(PathBuf),
}

/// Inputs to the runtime gates that come from outside the config file.
#[derive(Debug, Clone)]
pub struct LiveGateInputs {
    /// Process environment variables (injected for testability).
    pub env: Vec<(String, String)>,
    /// Path whose existence means KILL (engaged). Absent = clear. Any content
    /// (including empty) counts as engaged — an empty kill file still kills.
    pub kill_switch_path: PathBuf,
}

/// Acquire the live capability. All four gates must pass. In default builds
/// this is a compile-time refusal.
pub fn acquire_live(
    cfg: &AppConfig,
    inputs: &LiveGateInputs,
) -> Result<LiveCapability, LiveGateError> {
    #[cfg(not(feature = "live"))]
    {
        let _ = (cfg, inputs);
        Err(LiveGateError::NotCompiledIn)
    }
    #[cfg(feature = "live")]
    {
        live_impl::acquire(cfg, inputs)
    }
}

#[cfg(feature = "live")]
pub mod live_impl {
    use super::*;
    use sniper_core::config::RunMode;

    /// Live signer wrapper. Only constructible with a `LiveCapability`.
    /// Key loading from explicitly authorized storage arrives at M3/M7.
    #[derive(Debug)]
    pub struct LiveSigner {
        _cap: LiveCapability,
    }

    /// Live broadcaster wrapper. Only constructible with a `LiveCapability`.
    /// Route clients (DirectRpc/Jito/JupiterV2) arrive at M7.
    #[derive(Debug)]
    pub struct LiveBroadcaster {
        _cap: LiveCapability,
    }

    impl LiveSigner {
        pub fn new(cap: LiveCapability) -> Self {
            Self { _cap: cap }
        }
    }

    impl LiveBroadcaster {
        pub fn new(cap: LiveCapability) -> Self {
            Self { _cap: cap }
        }
    }

    pub(crate) fn acquire(
        cfg: &AppConfig,
        inputs: &LiveGateInputs,
    ) -> Result<LiveCapability, LiveGateError> {
        // Gate 2: env switch.
        let env_on = inputs
            .env
            .iter()
            .any(|(k, v)| k == "LIVE_TRADING_ENABLED" && v == "true");
        if !env_on {
            return Err(LiveGateError::EnvSwitchOff);
        }
        // Gate 3: config must be in validated live mode (reads the operator
        // ack file and cross-checks limits).
        if !cfg.runtime.live_trading_enabled || cfg.runtime.mode != RunMode::Live {
            return Err(LiveGateError::OperatorAck(
                "runtime config is not in live mode".into(),
            ));
        }
        if let Err(e) = cfg.validate() {
            return Err(LiveGateError::OperatorAck(e.to_string()));
        }
        // Gate 4: kill switch.
        if inputs.kill_switch_path.exists() {
            return Err(LiveGateError::KillSwitchEngaged(
                inputs.kill_switch_path.clone(),
            ));
        }
        Ok(LiveCapability { _priv: () })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn inputs(env: &[(&str, &str)]) -> LiveGateInputs {
        LiveGateInputs {
            env: env
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            kill_switch_path: Path::new("/tmp/snf-kill-switch-test-sentinel").to_path_buf(),
        }
    }

    #[test]
    #[cfg(not(feature = "live"))]
    fn default_build_cannot_acquire_live_capability() {
        // Gate 1: this test binary is built with default features. Even with
        // every other gate satisfiable, acquisition must refuse.
        let err = acquire_live(
            &AppConfig::safe_default(),
            &inputs(&[("LIVE_TRADING_ENABLED", "true")]),
        )
        .unwrap_err();
        assert!(matches!(err, LiveGateError::NotCompiledIn));
    }

    #[cfg(feature = "live")]
    #[test]
    fn live_build_gates_are_independent() {
        let (cfg, _ack_dir) = live_test_config();
        // Gate 2 off.
        assert!(matches!(
            live_impl::acquire(&cfg, &inputs(&[])),
            Err(LiveGateError::EnvSwitchOff)
        ));
        // Gate 3: safe config with env on still fails (not live-validated).
        assert!(matches!(
            live_impl::acquire(
                &AppConfig::safe_default(),
                &inputs(&[("LIVE_TRADING_ENABLED", "true")])
            ),
            Err(LiveGateError::OperatorAck(_))
        ));
        // Gate 4: kill file present.
        let dir = tempfile::tempdir().unwrap();
        let kill = dir.path().join("kill");
        std::fs::write(&kill, "").unwrap();
        let mut i = inputs(&[("LIVE_TRADING_ENABLED", "true")]);
        i.kill_switch_path = kill;
        assert!(matches!(
            live_impl::acquire(&cfg, &i),
            Err(LiveGateError::KillSwitchEngaged(_))
        ));
        // All gates clear → capability.
        assert!(live_impl::acquire(&cfg, &inputs(&[("LIVE_TRADING_ENABLED", "true")])).is_ok());
    }

    // Constructing a live-validated config requires a valid ack file; the
    // returned TempDir keeps that file alive for the test's duration.
    #[cfg(feature = "live")]
    fn live_test_config() -> (sniper_core::config::AppConfig, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let ack = dir.path().join("ack.txt");
        std::fs::write(&ack, sniper_core::config::OPERATOR_ACK_MAGIC).unwrap();
        let text = format!(
            "[runtime]\nmode = \"live\"\nlive_trading_enabled = true\noperator_ack_path = \"{}\"\n\
             [ingest]\nproviders = [\"geyser\"]\ngeyser_endpoint = \"https://geyser.example\"\n\
             [trade]\nbuy_amount_lamports = 1000000\nbuy_slippage_bps = 1000\nsell_slippage_bps = 1000\nexit_strategy = \"manual\"\n\
             [risk]\nmax_open_positions = 1\nmax_position_lamports = 2000000\nmax_aggregate_lamports = 2000000\n\
             max_daily_loss_lamports = 1000000\nmax_priority_fee_microlamports = 1000000\nmax_tip_lamports = 0\nmax_pending_orders = 1\nmax_staleness_slots = 5\n",
            ack.display()
        );
        (toml::from_str(&text).unwrap(), dir)
    }
}
