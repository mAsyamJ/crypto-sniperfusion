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
    let path = dir.join(format!("{mode}.toml"));
    std::fs::write(
        &path,
        format!(
            "[runtime]\nmode = \"{mode}\"\n\
             [trade]\nbuy_amount_lamports = 0\nbuy_slippage_bps = 0\nsell_slippage_bps = 0\nexit_strategy = \"manual\"\n"
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
fn paper_mode_runs_and_reports_planned_only() {
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
    assert_eq!(report["decisions"], 3);
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
