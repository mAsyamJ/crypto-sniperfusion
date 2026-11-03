//! sniper-app: CLI entry point. Safe modes by default; `live` refuses to run
//! unless compiled with the `live` feature AND all four runtime gates pass.

mod decode_events;
mod redact;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use sniper_core::config::{AppConfig, RunMode};
use sniper_runtime::broker::{PaperBrokerConfig, PaperBrokerHook};
use sniper_runtime::rpc_probe::{probe_endpoint, redact_url, resolve_endpoint};
use sniper_runtime::runner::{CountingHook, ReplayRunner};
use sniper_runtime::{acquire_live, Cluster, LiveGateInputs};

#[derive(Debug, Parser)]
#[command(
    name = "sniper",
    version,
    about = "Solana Sniper Fusion (safe modes only by default)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Stream-read only mode. Live streaming arrives at M2; requires a
    /// fixtures file until then (honest error otherwise).
    Observe {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        fixtures: Option<PathBuf>,
    },
    /// Deterministic replay of a JSONL event corpus.
    Replay {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        fixtures: PathBuf,
    },
    /// Replay + recorded (never executed) paper decisions. Simulated fills
    /// arrive at M3.
    Paper {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        fixtures: PathBuf,
    },
    /// Decode fixture payloads (differential harness; prints oracle-shaped JSON).
    DecodeEvents {
        #[arg(long)]
        fixtures: PathBuf,
    },
    /// Read-only cluster RPC probe: resolves the endpoint (operator env
    /// override SNF_{MAINNET,DEVNET,TESTNET}_RPC_URL -> pinned public
    /// cluster URL), verifies cluster identity by genesis hash, and reports
    /// version/slot/epoch. No signer is loaded; this cannot broadcast.
    RpcProbe {
        #[arg(long, value_enum)]
        cluster: ClusterArg,
        #[arg(long, default_value_t = 15)]
        timeout_secs: u64,
    },
    /// Live trading. Requires a `live`-feature build plus four runtime gates.
    Live {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, default_value = "/tmp/snf-kill-switch")]
        kill_switch_path: PathBuf,
    },
}

/// CLI-facing cluster names (clap ValueEnum) mapped onto the runtime enum.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum ClusterArg {
    Mainnet,
    Devnet,
    Testnet,
}

impl From<ClusterArg> for Cluster {
    fn from(v: ClusterArg) -> Self {
        match v {
            ClusterArg::Mainnet => Cluster::MainnetBeta,
            ClusterArg::Devnet => Cluster::Devnet,
            ClusterArg::Testnet => Cluster::Testnet,
        }
    }
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();
}

fn main() -> ExitCode {
    init_tracing();
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Observe { config, fixtures } => {
            run_replay_mode(config, fixtures, RunMode::Observe)
        }
        Command::Replay { config, fixtures } => {
            run_replay_mode(config, Some(fixtures), RunMode::Replay)
        }
        Command::Paper { config, fixtures } => {
            run_replay_mode(config, Some(fixtures), RunMode::Paper)
        }
        Command::DecodeEvents { fixtures } => decode_events::run(fixtures),
        Command::RpcProbe {
            cluster,
            timeout_secs,
        } => run_rpc_probe(cluster.into(), timeout_secs),
        Command::Live {
            config,
            kill_switch_path,
        } => run_live(config, kill_switch_path),
    };

    match result {
        Ok(summary_json) => {
            println!("{summary_json}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            // redact defensively: config paths and errors can echo env content.
            eprintln!("error: {}", redact::redact(&err));
            ExitCode::FAILURE
        }
    }
}

/// Read-only cluster probe (INFRA-2): resolve endpoint (operator env ->
/// pinned public), verify cluster identity by genesis, print JSON report.
fn run_rpc_probe(cluster: Cluster, timeout_secs: u64) -> Result<String, String> {
    let endpoint =
        resolve_endpoint(cluster, |var| std::env::var(var).ok()).map_err(|e| e.to_string())?;
    tracing::info!(
        cluster = %cluster,
        source = ?endpoint.source,
        url = %redact_url(&endpoint.url),
        "probing cluster endpoint (read-only)"
    );
    let report =
        probe_endpoint(&endpoint, Duration::from_secs(timeout_secs)).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}

fn run_replay_mode(
    config_path: PathBuf,
    fixtures: Option<PathBuf>,
    mode: RunMode,
) -> Result<String, String> {
    let cfg = AppConfig::load_toml(&config_path).map_err(|e| e.to_string())?;
    if cfg.runtime.mode != mode {
        return Err(format!(
            "config runtime.mode is {:?}, CLI mode is {mode:?}",
            cfg.runtime.mode
        ));
    }
    if mode == RunMode::Observe && cfg.runtime.live_trading_enabled {
        return Err("observe mode cannot run with live_trading_enabled".into());
    }

    let fixtures = fixtures.ok_or_else(|| {
        "observe mode needs real streaming (M2); until then pass --fixtures for offline replay"
            .to_string()
    })?;

    let events = ReplayRunner::load_fixtures(&fixtures).map_err(|e| e.to_string())?;
    let runner = ReplayRunner::default();

    let report = if mode == RunMode::Paper {
        // H2: paper mode runs the event-sourced PaperBroker — real virtual
        // fills, fees, positions and a reconciled net-P&L receipt.
        let broker_cfg = PaperBrokerConfig {
            slippage_bps: sniper_core::primitives::BasisPoints(cfg.trade.buy_slippage_bps),
            ..PaperBrokerConfig::default()
        };
        let mut hook = PaperBrokerHook::new(
            broker_cfg,
            sniper_core::primitives::Lamports(cfg.trade.buy_amount_lamports),
            "paper-replay-v1".to_string(),
        );
        let summary = runner.run(&events, &mut hook).map_err(|e| e.to_string())?;
        let broker = hook.broker();
        let closed: Vec<serde_json::Value> = broker
            .export_closed_trades_jsonl()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).expect("closed trade parses"))
            .collect();
        tracing::info!(
            mode = ?mode,
            events = summary.events_in,
            fills = hook.fills(),
            unfilled = hook.unfilled(),
            realized_pnl_lamports = broker.realized_pnl(),
            "paper run complete"
        );
        serde_json::to_string_pretty(&PaperRunReport {
            run: RunReport {
                mode: format!("{mode:?}"),
                events_in: summary.events_in,
                duplicates: summary.duplicates,
                published: summary.published,
                shed_speculative: summary.shed_speculative,
                decisions: summary.decisions,
                buys_proposed: hook.fills(),
            },
            fills: hook.fills(),
            unfilled: hook.unfilled(),
            cash_lamports: broker.cash().0,
            realized_pnl_lamports: broker.realized_pnl(),
            equity_mark_to_exit_lamports: broker.equity_mark_to_exit(),
            max_drawdown_lamports: broker.max_drawdown(),
            open_positions: broker.position_count(),
            closed_trades: closed,
        })
        .map_err(|e| e.to_string())?
    } else {
        let mut hook = CountingHook::default();
        let summary = runner.run(&events, &mut hook).map_err(|e| e.to_string())?;
        tracing::info!(
            mode = ?mode,
            events = summary.events_in,
            duplicates = summary.duplicates,
            shed = summary.shed_speculative,
            decisions = summary.decisions,
            "run complete"
        );
        serde_json::to_string_pretty(&RunReport {
            mode: format!("{mode:?}"),
            events_in: summary.events_in,
            duplicates: summary.duplicates,
            published: summary.published,
            shed_speculative: summary.shed_speculative,
            decisions: summary.decisions,
            buys_proposed: hook.buys_proposed,
        })
        .map_err(|e| e.to_string())?
    };
    Ok(report)
}

#[derive(serde::Serialize)]
struct PaperRunReport {
    run: RunReport,
    fills: u64,
    unfilled: u64,
    cash_lamports: u64,
    realized_pnl_lamports: i128,
    equity_mark_to_exit_lamports: i128,
    max_drawdown_lamports: i128,
    open_positions: usize,
    closed_trades: Vec<serde_json::Value>,
}

#[derive(serde::Serialize)]
struct RunReport {
    mode: String,
    events_in: u64,
    duplicates: u64,
    published: u64,
    shed_speculative: u64,
    decisions: u64,
    buys_proposed: u64,
}

fn run_live(config_path: PathBuf, kill_switch_path: PathBuf) -> Result<String, String> {
    let cfg = AppConfig::load_toml(&config_path).map_err(|e| e.to_string())?;
    let inputs = LiveGateInputs {
        env: std::env::vars().collect(),
        kill_switch_path,
    };
    let _cap = acquire_live(&cfg, &inputs).map_err(|e| e.to_string())?;
    // Capability acquired — but no live routes exist before M7. Refuse rather
    // than pretend: an honest error beats a half-live runtime.
    Err("live capability gates passed, but live routes are not implemented until M7; refusing to continue".into())
}
