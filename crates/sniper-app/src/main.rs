//! sniper-app: CLI entry point. Safe modes by default; `live` refuses to run
//! unless compiled with the `live` feature AND all four runtime gates pass.

mod decode_events;
mod redact;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use sniper_core::config::{AppConfig, RunMode};
use sniper_runtime::runner::{CountingHook, ReplayRunner};
use sniper_runtime::{acquire_live, LiveGateInputs};

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
    /// Live trading. Requires a `live`-feature build plus four runtime gates.
    Live {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, default_value = "/tmp/snf-kill-switch")]
        kill_switch_path: PathBuf,
    },
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
    .map_err(|e| e.to_string())
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
