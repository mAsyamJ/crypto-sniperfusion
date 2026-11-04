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
    /// version/slot/epoch. Optionally reads one address's balance
    /// (--address). No signer is loaded; this cannot broadcast.
    RpcProbe {
        #[arg(long, value_enum)]
        cluster: ClusterArg,
        #[arg(long, default_value_t = 15)]
        timeout_secs: u64,
        /// Optional base58 address whose balance is read (watch-only).
        #[arg(long, value_name = "PUBKEY")]
        address: Option<String>,
    },
    /// Offline multi-strategy research batch (H7): runs the S00..S10
    /// registry over a frozen observation dataset with identical inputs,
    /// evaluates the full portfolio, and writes an immutable receipt.
    /// Permanently nonpromotable; never touches a network or signer.
    Research {
        #[arg(long, value_name = "JSONL")]
        events: PathBuf,
        #[arg(long, value_name = "JSON")]
        out: PathBuf,
        /// Optional market-snapshot fixture (JSONL) — when present, also
        /// runs Engine B (ProtocolEconomicV1) with the RANDOM_ELIGIBLE
        /// baseline and emits the P&L V2 receipt bundle here.
        #[arg(long, value_name = "JSONL")]
        snapshots: Option<PathBuf>,
        #[arg(long, value_name = "DIR")]
        out_dir: Option<PathBuf>,
    },
    /// WA1/WA2: normalize wallet transaction receipts through the Rust
    /// wallet_observer and compute the wallet's historical P&L through
    /// the independent PF1 oracle, emitting a PnlRunReceiptV2 bundle.
    /// Read-only offline analysis of already-fetched receipts.
    WalletPnl {
        #[arg(long, value_name = "JSON")]
        transactions: PathBuf,
        #[arg(long, value_name = "PUBKEY")]
        wallet: String,
        #[arg(long, value_name = "DIR")]
        out_dir: PathBuf,
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
            address,
        } => run_rpc_probe(cluster.into(), timeout_secs, address.as_deref()),
        Command::Research {
            events,
            out,
            snapshots,
            out_dir,
        } => run_research(events, out, snapshots, out_dir),
        Command::WalletPnl {
            transactions,
            wallet,
            out_dir,
        } => run_wallet_pnl(transactions, wallet, out_dir),
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
/// An optional watch address adds one read-only getBalance call.
fn run_rpc_probe(
    cluster: Cluster,
    timeout_secs: u64,
    address: Option<&str>,
) -> Result<String, String> {
    let endpoint =
        resolve_endpoint(cluster, |var| std::env::var(var).ok()).map_err(|e| e.to_string())?;
    tracing::info!(
        cluster = %cluster,
        source = ?endpoint.source,
        url = %redact_url(&endpoint.url),
        watch_address = address,
        "probing cluster endpoint (read-only)"
    );
    let report = probe_endpoint(&endpoint, Duration::from_secs(timeout_secs), address)
        .map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}

/// Offline research batch (H7): S00..S10 over one dataset, identical
/// inputs, full-portfolio evaluation, immutable write-once receipt.
/// Deterministic by construction: no wall clock, no RNG, no network.
fn run_research(
    events: PathBuf,
    out: PathBuf,
    snapshots: Option<PathBuf>,
    out_dir: Option<PathBuf>,
) -> Result<String, String> {
    use sniper_strategy::engine::legacy::{self, LegacyParams};
    use sniper_strategy::eval;
    use sniper_strategy::{Observation, RiskConstraints, StrategyKnobs};

    let live = std::env::var("LIVE_TRADING_ENABLED").unwrap_or_default();
    if !matches!(live.as_str(), "" | "false" | "0") {
        return Err("research refuses LIVE_TRADING_ENABLED".into());
    }

    let text =
        std::fs::read_to_string(&events).map_err(|e| format!("read {}: {e}", events.display()))?;
    let dataset: Vec<Observation> =
        sniper_strategy::load_events(&text).map_err(|e| format!("invalid dataset: {e}"))?;
    let dataset_sha = {
        use sha2::Digest;
        let digest = sha2::Sha256::digest(text.as_bytes());
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };

    let knobs = StrategyKnobs::default();
    let risk = RiskConstraints::default();
    let params = LegacyParams::default();
    let cash = 100_000_000u64;

    let mut results = Vec::new();
    let mut traces = Vec::new();
    for mut strategy in sniper_strategy::registry(knobs) {
        let (result, trace) =
            legacy::run_with_trace(&dataset, strategy.as_mut(), cash, &risk, &params);
        results.push(result);
        traces.push(trace);
    }
    let evaluation = eval::evaluate_batch(&results, &traces);

    let receipt = serde_json::json!({
        "schema_version": 1,
        "command": "research",
        "engine": "LegacySurrogateV1",
        "evidence_class": evaluation.evidence_class,
        "promotable": false,
        "live_trading_enabled": false,
        "dataset": {
            "path": events.display().to_string(),
            "sha256": dataset_sha,
            "events": dataset.len(),
            "mints": dataset.iter().map(|e| e.mint.clone()).collect::<std::collections::BTreeSet<_>>().len(),
        },
        "parameters": {
            "cash_lamports": cash,
            "spend_lamports": risk.spend_lamports,
            "knobs": { "min_buyers": knobs.min_buyers, "liquidity_multiple": knobs.liquidity_multiple },
            "legacy": {
                "slip_bps": params.slip_bps,
                "tx_cost_lamports": params.tx_cost_lamports,
                "ata_rent_lamports": params.ata_rent_lamports,
                "max_hold_ms": params.max_hold_ms,
            },
        },
        "no_trade_baseline": "S00",
        "verdict": evaluation.verdict,
        "results": results,
        "metrics": evaluation.metrics.into_iter().collect::<std::collections::BTreeMap<_, _>>(),
        "limitations": evaluation.limitations,
        "determinism": "bit-identical for identical dataset + parameters (no clock, no RNG)",
    });
    let body = serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())? + "\n";

    // Write-once immutable receipt (exclusive create + fsync + 0444).
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&out)
        .map_err(|e| format!("refusing to overwrite {}: {e}", out.display()))?;
    file.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())?;
    drop(file);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o444))
            .map_err(|e| e.to_string())?;
    }

    let mut summary = serde_json::json!({
        "out": out.display().to_string(),
        "dataset_sha256": dataset_sha,
        "verdict": evaluation.verdict,
        "promotable": false,
        "strategies": results.len(),
    });

    // P&L V2 bundle (PF1): Engine B on the observation + snapshot pair,
    // oracle-reconciled, per-strategy receipts + matched comparison.
    if let (Some(snapshots_path), Some(bundle_dir)) = (snapshots, out_dir) {
        let snapshots_text = std::fs::read_to_string(&snapshots_path)
            .map_err(|e| format!("read {}: {e}", snapshots_path.display()))?;
        let book = sniper_strategy::engine::protocol::MarketBook::load_jsonl(&snapshots_text)
            .map_err(|e| format!("invalid snapshots: {e}"))?;
        let frictions = sniper_strategy::engine::protocol::ExecutionFrictions::default();
        let params = sniper_strategy::engine::protocol::ProtocolParams::default();

        let mut strategies: Vec<Box<dyn sniper_strategy::Strategy>> =
            sniper_strategy::registry(knobs);
        strategies.push(Box::new(
            sniper_strategy::policies::EntryPolicy::random_eligible(knobs),
        ));

        let mut receipts = Vec::new();
        let mut comparisons = Vec::new();
        let mut audit = serde_json::Map::new();
        let mut trades_jsonl = String::new();
        let mut equity_jsonl = String::new();
        for mut strategy in strategies {
            let id = strategy.id().as_str().to_string();
            let outcome = sniper_strategy::engine::protocol::run(
                &dataset,
                &book,
                strategy.as_mut(),
                cash,
                &risk,
                &frictions,
                &params,
            );
            // Incremental ingest for the per-record cash curve.
            let mut step_oracle = sniper_strategy::oracle::AccountingOracle::new(cash);
            for record in &outcome.records {
                step_oracle
                    .ingest(&id, std::slice::from_ref(record))
                    .map_err(|e| e.to_string())?;
                equity_jsonl.push_str(&format!(
                    "{{\"strategy\": \"{id}\", \"cash_lamports\": {}}}\n",
                    step_oracle.cash()
                ));
                trades_jsonl.push_str(&format!(
                    "{{\"strategy\": \"{id}\", \"record\": {}}}\n",
                    serde_json::to_string(record).unwrap()
                ));
            }
            let oracle = sniper_strategy::oracle::AccountingOracle::from_run(&outcome, cash)
                .map_err(|e| e.to_string())?;
            let conserved = oracle.verify_conservation().is_ok();
            let cash_match = oracle.cash() == outcome.ending_cash_lamports;
            audit.insert(
                id.clone(),
                serde_json::json!({"conservation_ok": conserved, "cash_matches_engine": cash_match}),
            );
            if !conserved || !cash_match {
                return Err(format!("oracle reconciliation failed for {id}"));
            }
            let marks = std::collections::HashMap::new();
            let net = oracle.net_pnl(cash, &marks);
            comparisons.push(sniper_strategy::receipt::ComparisonRow {
                strategy: id.clone(),
                net_pnl_lamports: net,
                incremental_net_edge_vs_no_trade: net, // vs S00's 0 on this synthetic pair
                max_drawdown_lamports: 0,
                open_positions: outcome.open_lots.len() as u64,
                status: if net <= 0 {
                    "LOSING"
                } else {
                    "CANDIDATE_FOR_INDEPENDENT_REVIEW"
                }
                .into(),
            });
            let receipt = sniper_strategy::receipt::build_receipt(
                "ProtocolEconomicV1",
                sniper_strategy::engine::legacy::EVIDENCE_CLASS,
                "SYNTHETIC_COMPLETE_FIXTURE",
                &dataset_sha,
                cash,
                &oracle,
                &marks,
                *oracle
                    .trial_balance()
                    .get(&sniper_strategy::oracle::Account::RealizedPnl)
                    .unwrap_or(&0) as i64,
                0,
                outcome.open_lots.len() as u64,
                0,
                Vec::new(),
                vec![
                    "synthetic fixture cohort; not market evidence".into(),
                    "fees: OBSERVED on-chain curve fee config (95+30 bps) at slot 465329418; execution frictions synthetic".into(),
                ],
            );
            receipts.push((id, receipt));
        }

        let bundle = std::path::Path::new(&bundle_dir);
        for (id, receipt) in &receipts {
            let dir = bundle.join("strategies").join(id);
            sniper_strategy::receipt::write_bundle(&dir, receipt, "", "", "{}")
                .map_err(|e| format!("bundle {id}: {e}"))?;
        }
        // Root run.json: S00 baseline receipt with the full comparison.
        let mut root = receipts
            .iter()
            .find(|(id, _)| id == "S00")
            .map(|(_, r)| r.clone())
            .expect("S00 present");
        root.comparison = comparisons.clone();
        root = root.seal();
        sniper_strategy::receipt::write_bundle(
            bundle,
            &root,
            &trades_jsonl,
            &equity_jsonl,
            &serde_json::to_string(&serde_json::Value::Object(audit)).unwrap(),
        )
        .map_err(|e| format!("root bundle: {e}"))?;
        summary["pnl_v2_bundle"] = serde_json::json!({
            "dir": bundle_dir.display().to_string(),
            "receipt_sha256": root.receipt_sha256,
            "engine": "ProtocolEconomicV1",
        });
    }
    Ok(serde_json::to_string_pretty(&summary).unwrap())
}

/// WA1/WA2 wallet historical P&L: normalize receipts (Rust wallet_observer)
/// -> observed-amount ExecutionRecords -> independent PF1 oracle -> V2 bundle.
fn run_wallet_pnl(
    transactions: PathBuf,
    wallet: String,
    out_dir: PathBuf,
) -> Result<String, String> {
    use sniper_runtime::wallet_observer::{normalize_transaction, Side};
    use sniper_strategy::engine::protocol::ExecutionRecord;
    use sniper_strategy::oracle::AccountingOracle;
    use sniper_strategy::receipt::{build_receipt, write_bundle, ComparisonRow};

    let live = std::env::var("LIVE_TRADING_ENABLED").unwrap_or_default();
    if !matches!(live.as_str(), "" | "false" | "0") {
        return Err("wallet-pnl refuses LIVE_TRADING_ENABLED".into());
    }
    let text = std::fs::read_to_string(&transactions)
        .map_err(|e| format!("read {}: {e}", transactions.display()))?;
    let txs: Vec<serde_json::Value> = serde_json::from_str(&text)
        .map_err(|e| format!("transactions file must be a JSON array: {e}"))?;

    let starting_cash: u64 = 0; // observed amounts flow; opening balances unknown
    let mut records: Vec<ExecutionRecord> = Vec::new();
    let mut skipped = 0u64;
    let mut incomplete = 0u64;
    for tx in &txs {
        for event in normalize_transaction(tx, &wallet).map_err(|e| e.to_string())? {
            if event.coverage != "COMPLETE" {
                incomplete += 1;
                continue;
            }
            let sig = event.signature.clone();
            match event.side {
                Side::Buy => records.push(ExecutionRecord::BuyFill {
                    event_id: sig,
                    mint: event.mint.clone(),
                    slot: event.slot,
                    spend_lamports: (-event.raw_quote_delta) as u64,
                    tokens_out: event.raw_token_delta as u64,
                    protocol_fee_lamports: 0,
                    creator_fee_lamports: 0,
                    lp_fee_lamports: 0,
                    network_fee_lamports: event.network_fee,
                    priority_fee_lamports: 0,
                    tip_lamports: 0,
                    rent_lamports: 0,
                    slippage_lamports: 0,
                    fee_synthetic_fallback: false,
                }),
                Side::Sell => records.push(ExecutionRecord::SellFill {
                    event_id: sig,
                    mint: event.mint.clone(),
                    slot: event.slot,
                    tokens_in: (-event.raw_token_delta) as u64,
                    quote_out_net: event.raw_quote_delta as u64,
                    protocol_fee_lamports: 0,
                    creator_fee_lamports: 0,
                    lp_fee_lamports: 0,
                    network_fee_lamports: event.network_fee,
                    priority_fee_lamports: 0,
                    tip_lamports: 0,
                    rent_recovered_lamports: 0,
                    slippage_lamports: 0,
                    fee_synthetic_fallback: false,
                }),
                _ => {
                    skipped += 1;
                }
            }
        }
    }
    // Observed amounts: records already net of venue fees; ingest
    // sequentially with deposit-backed cash (opening balances unknown ->
    // start 0; deposits represent unobserved earlier capital, excluded
    // from P&L by net_pnl).
    let mut ledger = AccountingOracle::new(starting_cash);
    let mut deposits_total = 0u64;
    let mut opening_inventory_events = 0u64;
    for record in &records {
        let needed = match record {
            ExecutionRecord::BuyFill {
                spend_lamports,
                network_fee_lamports,
                ..
            } => spend_lamports + network_fee_lamports,
            _ => 0,
        };
        if (ledger.cash() as u128) < needed as u128 {
            let top_up = needed - ledger.cash();
            ledger
                .post_deposit(top_up, "unobserved opening capital")
                .map_err(|e| e.to_string())?;
            deposits_total += top_up;
        }
        // Sells of tokens acquired BEFORE the observed window: the oracle
        // refuses (fail closed) — record the shortfall as unobserved
        // opening inventory (equity-backed), never as a invented trade.
        if let ExecutionRecord::SellFill {
            mint,
            tokens_in,
            quote_out_net,
            ..
        } = record
        {
            let held: u64 = ledger.lots_for(mint).iter().map(|l| l.tokens).sum();
            if held < *tokens_in {
                // Unknown basis: assume basis == proceeds so this sell
                // contributes ZERO realized P&L (disclosed in limitations).
                ledger
                    .post_opening_inventory(mint, tokens_in - held, *quote_out_net)
                    .map_err(|e| e.to_string())?;
                opening_inventory_events += 1;
            }
        }
        ledger
            .ingest("leader", std::slice::from_ref(record))
            .map_err(|e| e.to_string())?;
    }
    if !ledger.verify_conservation().is_ok() {
        return Err("oracle conservation failed".into());
    }
    let marks = std::collections::HashMap::new();
    let realized = *ledger
        .trial_balance()
        .get(&sniper_strategy::oracle::Account::RealizedPnl)
        .unwrap_or(&0) as i64;
    let open_lots: Vec<String> = ledger
        .trial_balance()
        .keys()
        .filter_map(|a| match a {
            sniper_strategy::oracle::Account::TokenInventory { mint } => Some(mint.clone()),
            _ => None,
        })
        .collect();
    let net = ledger.net_pnl(starting_cash, &marks);
    let receipt = build_receipt(
        "wallet_observer_v1",
        "MAINNET_READ_ONLY_OBSERVED",
        if incomplete == 0 && skipped == 0 {
            "OBSERVED_WINDOW"
        } else {
            "OBSERVED_WINDOW_PARTIAL"
        },
        "live-tx-receipts",
        starting_cash,
        &ledger,
        &marks,
        realized,
        0,
        open_lots.len() as u64,
        0,
        vec![ComparisonRow {
            strategy: "LEADER".into(),
            net_pnl_lamports: net,
            incremental_net_edge_vs_no_trade: net,
            max_drawdown_lamports: 0,
            open_positions: open_lots.len() as u64,
            status: "OBSERVED".into(),
        }],
        vec![
            "observed-amount accounting: venue fees are embedded in the observed quote flow (disclosed, not re-subtracted)".into(),
            "opening balances unknown: unobserved capital appears as deposits (excluded from P&L)".into(),
            format!("open positions marked at ZERO (conservative): {open_lots:?}"),
            format!("skipped non-swap events: {skipped}; incomplete-history events: {incomplete}; unobserved opening-inventory adjustments: {opening_inventory_events}"),
        ],
    );
    let run = write_bundle(&out_dir, &receipt, "", "", "{}").map_err(|e| e.to_string())?;
    let summary = serde_json::json!({
        "out": run.display().to_string(),
        "wallet": wallet,
        "transactions": txs.len(),
        "records": records.len(),
        "skipped": skipped,
        "incomplete": incomplete,
        "unobserved_deposits_lamports": deposits_total,
        "unobserved_opening_inventory_events": opening_inventory_events,
        "net_pnl_lamports": net,
        "receipt_sha256": receipt.receipt_sha256,
        "nonpromotable": true,
    });
    Ok(serde_json::to_string_pretty(&summary).unwrap())
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
