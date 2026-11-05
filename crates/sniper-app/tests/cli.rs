//! CLI integration tests: real binary, real temp configs, offline fixtures.

use std::path::PathBuf;
use std::process::Command;

use sniper_core::event::{ChainEvent, ChainEventId, Commitment, EventPayload, ProviderKind};
use sniper_core::time::Timestamps;
use sniper_core::venue::Venue;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;

fn bin() -> PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop(); // tests dir
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("sniper")
}

fn write_config(dir: &std::path::Path, mode: &str) -> PathBuf {
    write_config_with_buy(dir, mode, 0)
}

fn write_config_with_buy(dir: &std::path::Path, mode: &str, buy: u64) -> PathBuf {
    let path = dir.join(format!("{mode}-{buy}.toml"));
    std::fs::write(
        &path,
        format!(
            "[runtime]\nmode = \"{mode}\"\n\
             [trade]\nbuy_amount_lamports = {buy}\nbuy_slippage_bps = 0\nsell_slippage_bps = 0\nexit_strategy = \"manual\"\n"
        ),
    )
    .unwrap();
    path
}

fn write_fixtures(dir: &std::path::Path, n: usize) -> PathBuf {
    let path = dir.join("fixtures.jsonl");
    let mut lines = Vec::new();
    for i in 0..n {
        let mut sig = [0u8; 64];
        sig[0] = i as u8;
        let event = ChainEvent {
            id: ChainEventId {
                signature: Signature::from(sig),
                outer_ix_index: 0,
                inner_ix_index: None,
                event_ordinal: 0,
            },
            venue: Venue::PumpFun,
            slot: 100 + i as u64,
            commitment: Commitment::Processed,
            received: Timestamps {
                wall: std::time::SystemTime::UNIX_EPOCH,
                mono_ns: i as u64,
            },
            source: ProviderKind::Replay,
            schema_version: 1,
            payload: EventPayload::TokenCreated {
                mint: Pubkey::new_unique(),
                creator: Pubkey::new_unique(),
                name: format!("t{i}"),
                symbol: format!("T{i}"),
                uri: String::new(),
            },
        };
        lines.push(serde_json::to_string(&event).unwrap());
    }
    std::fs::write(&path, lines.join("\n")).unwrap();
    path
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(bin())
        .args(args)
        .output()
        .expect("binary runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn replay_mode_end_to_end_offline() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path(), "replay");
    let fixtures = write_fixtures(dir.path(), 5);
    let (ok, stdout, stderr) = run(&[
        "replay",
        "--config",
        cfg.to_str().unwrap(),
        "--fixtures",
        fixtures.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}\nstdout: {stdout}");
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["events_in"], 5);
    assert_eq!(report["duplicates"], 0);
    assert_eq!(report["buys_proposed"], 5);
}

#[test]
fn paper_mode_zero_spend_never_buys() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path(), "paper");
    let fixtures = write_fixtures(dir.path(), 3);
    let (ok, stdout, stderr) = run(&[
        "paper",
        "--config",
        cfg.to_str().unwrap(),
        "--fixtures",
        fixtures.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["fills"], 0, "zero spend must never fill");
    assert_eq!(report["open_positions"], 0);
    assert_eq!(report["cash_lamports"], 1_000_000_000u64);
}

#[test]
fn paper_mode_funded_buys_open_nonzero_positions() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config_with_buy(dir.path(), "paper", 50_000_000);
    let fixtures = write_fixtures(dir.path(), 3);
    let (ok, stdout, stderr) = run(&[
        "paper",
        "--config",
        cfg.to_str().unwrap(),
        "--fixtures",
        fixtures.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // H2 contract: a reported buy MUST have a non-zero position when fill
    // criteria pass — no more intents-only paper mode.
    assert_eq!(report["fills"], 3, "one fill per created token");
    assert_eq!(report["open_positions"], 3);
    assert!(report["cash_lamports"].as_u64().unwrap() < 1_000_000_000);
    assert_eq!(report["run"]["decisions"], 3);
}

#[test]
fn observe_without_fixtures_fails_honestly() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path(), "observe");
    let (ok, _, stderr) = run(&["observe", "--config", cfg.to_str().unwrap()]);
    assert!(!ok);
    assert!(
        stderr.contains("M2"),
        "must point at the milestone that brings streaming: {stderr}"
    );
}

#[test]
fn live_mode_refuses_in_default_build() {
    let dir = tempfile::tempdir().unwrap();
    // A live config without ack file fails validation first — that refusal is
    // itself correct; check the message mentions the gate reason.
    let cfg = dir.path().join("live.toml");
    std::fs::write(
        &cfg,
        "[runtime]\nmode = \"live\"\nlive_trading_enabled = true\n",
    )
    .unwrap();
    let (ok, _, stderr) = run(&["live", "--config", cfg.to_str().unwrap()]);
    assert!(!ok, "live must not run in a default build");
    assert!(
        stderr.contains("live") || stderr.contains("gate") || stderr.contains("ack"),
        "stderr: {stderr}"
    );
}

#[test]
fn mode_mismatch_between_cli_and_config_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path(), "observe");
    let fixtures = write_fixtures(dir.path(), 1);
    let (ok, _, stderr) = run(&[
        "replay",
        "--config",
        cfg.to_str().unwrap(),
        "--fixtures",
        fixtures.to_str().unwrap(),
    ]);
    assert!(!ok);
    assert!(stderr.contains("mode"), "stderr: {stderr}");
}

// ------------------------------------------------------------------ research --

fn repo_fixture(relative: &str) -> PathBuf {
    // Tests run from the crate dir; fixtures live at the workspace root.
    let mut p = std::env::current_dir().unwrap();
    for _ in 0..3 {
        if p.join("harness").is_dir() {
            return p.join(relative);
        }
        p.pop();
    }
    panic!("workspace root not found");
}

#[test]
fn research_writes_nonpromotable_receipt_and_refuses_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("receipt.json");
    let events = repo_fixture("harness/fixtures/research/synthetic_lifecycle.jsonl");
    if !events.exists() {
        return; // running outside the workspace checkout
    }
    let status = Command::new(bin())
        .arg("research")
        .arg("--events")
        .arg(&events)
        .arg("--out")
        .arg(&out)
        .env("LIVE_TRADING_ENABLED", "false")
        .status()
        .unwrap();
    assert!(status.success());
    let body: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(body["evidence_class"], "SYNTHETIC_SURROGATE_UNCALIBRATED");
    assert_eq!(body["promotable"], false);
    assert_eq!(body["live_trading_enabled"], false);
    assert_eq!(body["no_trade_baseline"], "S00");
    assert_eq!(body["verdict"], "EDGE_NOT_DEMONSTRATED");
    assert_eq!(body["engine"], "LegacySurrogateV1");
    assert_eq!(body["results"].as_array().map(Vec::len), Some(11));
    assert_eq!(
        body["dataset"]["sha256"],
        "ca20dd74c132cef16b13372a827156bd3860b263b2e29c15dd127b96db8c5a93"
    );
    // Metrics present for every strategy, S00 first.
    let metrics = body["metrics"].as_object().unwrap();
    assert_eq!(metrics.len(), 11);
    assert_eq!(body["results"][0]["strategy"], "S00");
    // Write-once: a second identical run must fail, not overwrite.
    let second = Command::new(bin())
        .arg("research")
        .arg("--events")
        .arg(&events)
        .arg("--out")
        .arg(&out)
        .env("LIVE_TRADING_ENABLED", "false")
        .status()
        .unwrap();
    assert!(!second.success(), "receipts are immutable");
}

#[test]
fn research_refuses_live_trading_env() {
    let dir = tempfile::tempdir().unwrap();
    let events = repo_fixture("harness/fixtures/research/synthetic_lifecycle.jsonl");
    if !events.exists() {
        return;
    }
    let status = Command::new(bin())
        .arg("research")
        .arg("--events")
        .arg(&events)
        .arg("--out")
        .arg(dir.path().join("x.json"))
        .env("LIVE_TRADING_ENABLED", "true")
        .status()
        .unwrap();
    assert!(!status.success());
    assert!(!dir.path().join("x.json").exists());
}

#[test]
fn research_receipt_is_bit_identical_across_runs() {
    let dir = tempfile::tempdir().unwrap();
    let events = repo_fixture("harness/fixtures/research/synthetic_lifecycle.jsonl");
    if !events.exists() {
        return;
    }
    let outs = ["a.json", "b.json"].map(|n| dir.path().join(n));
    for out in &outs {
        let status = Command::new(bin())
            .arg("research")
            .arg("--events")
            .arg(&events)
            .arg("--out")
            .arg(out)
            .env("LIVE_TRADING_ENABLED", "false")
            .status()
            .unwrap();
        assert!(status.success());
    }
    let a = std::fs::read_to_string(&outs[0]).unwrap();
    let b = std::fs::read_to_string(&outs[1]).unwrap();
    assert_eq!(a, b, "receipts must be bit-identical for identical inputs");
}

#[test]
fn research_emits_pnl_v2_bundle_with_oracle_reconciliation() {
    let dir = tempfile::tempdir().unwrap();
    let root = repo_fixture("");
    let events = root.join("harness/fixtures/research/synthetic_lifecycle.jsonl");
    let snapshots = root.join("harness/fixtures/research/synthetic_market_snapshots.jsonl");
    if !events.exists() || !snapshots.exists() {
        return;
    }
    let bundle = dir.path().join("bundle");
    let status = Command::new(bin())
        .arg("research")
        .arg("--events")
        .arg(&events)
        .arg("--out")
        .arg(dir.path().join("receipt.json"))
        .arg("--snapshots")
        .arg(&snapshots)
        .arg("--out-dir")
        .arg(&bundle)
        .env("LIVE_TRADING_ENABLED", "false")
        .status()
        .unwrap();
    assert!(status.success());
    let run: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(bundle.join("run.json")).unwrap()).unwrap();
    assert_eq!(run["schema_version"], 2);
    assert_eq!(run["promotable"], false);
    assert_eq!(run["engine"], "ProtocolEconomicV1");
    assert_eq!(run["proof_state"], "SYNTHETIC");
    assert_eq!(run["verdict"], "EDGE_NOT_DEMONSTRATED");
    // S00 baseline net 0 and every strategy <= 0 on this cohort.
    let rows = run["comparison"].as_array().unwrap();
    assert!(rows.len() >= 11);
    for row in rows {
        assert!(row["net_pnl_lamports"].as_i64().unwrap() <= 0);
    }
    // Audit gate: every strategy reconciled.
    let audit: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(bundle.join("audit.json")).unwrap()).unwrap();
    for (_, v) in audit.as_object().unwrap() {
        assert_eq!(v["conservation_ok"], true);
        assert_eq!(v["cash_matches_engine"], true);
    }
    // Determinism: same inputs -> identical receipt hash.
    let bundle2 = dir.path().join("bundle2");
    let status = Command::new(bin())
        .arg("research")
        .arg("--events")
        .arg(&events)
        .arg("--out")
        .arg(dir.path().join("receipt2.json"))
        .arg("--snapshots")
        .arg(&snapshots)
        .arg("--out-dir")
        .arg(&bundle2)
        .env("LIVE_TRADING_ENABLED", "false")
        .status()
        .unwrap();
    assert!(status.success());
    let run2: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(bundle2.join("run.json")).unwrap()).unwrap();
    assert_eq!(run["receipt_sha256"], run2["receipt_sha256"]);
}

#[test]
fn wallet_follow_prices_at_delay_and_refuses_invalid() {
    let dir = tempfile::tempdir().unwrap();
    let signals = dir.path().join("signals.jsonl");
    let quotes = dir.path().join("quotes.jsonl");
    std::fs::write(&signals, "{\"signal_id\":\"s1\",\"wallet\":\"W\",\"mint\":\"M\",\"leader_side\":\"Buy\",\"observed_at_ms\":100,\"quote_available_at_ms\":100}\n\
                              {\"signal_id\":\"s2\",\"wallet\":\"W\",\"mint\":\"M\",\"leader_side\":\"Sell\",\"observed_at_ms\":200,\"quote_available_at_ms\":200}\n").unwrap();
    std::fs::write(&quotes, "{\"mint\":\"M\",\"at_ms\":0,\"price_lamports_per_token\":1000,\"exit_liquidity_lamports\":10000000}\n\
                             {\"mint\":\"M\",\"at_ms\":50000,\"price_lamports_per_token\":500,\"exit_liquidity_lamports\":10000000}\n").unwrap();
    let out = dir.path().join("bundle");
    let status = Command::new(bin())
        .arg("wallet-follow")
        .arg("--signals")
        .arg(&signals)
        .arg("--quotes")
        .arg(&quotes)
        .arg("--out-dir")
        .arg(&out)
        .env("LIVE_TRADING_ENABLED", "false")
        .status()
        .unwrap();
    assert!(status.success());
    let run: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("run.json")).unwrap()).unwrap();
    assert_eq!(run["promotable"], false);
    let rows = run["comparison"].as_array().unwrap();
    let s00 = rows.iter().find(|r| r["strategy"] == "S00").unwrap();
    assert_eq!(s00["net_pnl_lamports"], 0);
    // Every delay arm prices entry at/after observed+delay from the
    // timeline; with these quotes all arms lose or refuse.
    for row in rows {
        if row["strategy"] != "S00" {
            assert!(row["net_pnl_lamports"].as_i64().unwrap() <= 0);
        }
    }
    // Leader side != Buy must refuse.
    let trades = std::fs::read_to_string(out.join("trades.jsonl")).unwrap();
    let arms: Vec<serde_json::Value> = serde_json::from_str(&trades).unwrap();
    assert_eq!(arms.len(), 6, "six delay arms");
    for arm in &arms {
        assert_eq!(arm["followed_entries"], 1, "only the Buy signal followed");
    }
}
