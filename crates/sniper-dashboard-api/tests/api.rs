//! DASH-02 Task 5 — API contract tests (in-process, tower oneshot):
//! T-RUN1, T-PNL1, T-NAV1, T-CAL7, T-ISO1, T-ISO2, T-CC1, T-EVT1.
//! Fixtures are built through the real producer builders (oracle → sealed
//! receipt → bundle), imported via the real discovery path.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use sniper_dashboard_api::http_api::{router, SharedDb};
use sniper_observability::discovery::Roots;
use sniper_observability::project::{import_corpus, open_db};
use sniper_strategy::follower::{FollowerEntry, FollowerRun};
use sniper_strategy::oracle::AccountingOracle;
use sniper_strategy::receipt::{build_receipt, write_bundle, ComparisonRow};

const BUILT_AT: &str = "2026-11-05T00:00:00Z";

fn s00() -> ComparisonRow {
    ComparisonRow {
        strategy: "S00".into(),
        net_pnl_lamports: 0,
        incremental_net_edge_vs_no_trade: 0,
        max_drawdown_lamports: 0,
        open_positions: 0,
        status: "NO_TRADE_BASELINE".into(),
    }
}

fn arm(strategy: &str, net: i64) -> ComparisonRow {
    ComparisonRow {
        strategy: strategy.into(),
        net_pnl_lamports: net,
        incremental_net_edge_vs_no_trade: net,
        max_drawdown_lamports: 0,
        open_positions: 0,
        status: "LOSING".into(),
    }
}

fn follower_bundle(dir: &std::path::Path, delay_ms: u64, entry_net: i64, at_ms: u64) {
    let runs = vec![FollowerRun {
        delay_ms,
        signals: 1,
        followed_entries: 1,
        refused_invalid: 0,
        refused_no_quote: 0,
        duplicates_dropped: 0,
        closed_exits: 1,
        open_positions: 0,
        net_pnl_lamports: entry_net,
        entries: vec![FollowerEntry {
            signal_id: "s1".into(),
            mint: "mintX".into(),
            entry_at_ms: at_ms,
            entry_effective_lamports: 987_500,
            tokens: 900,
            exit_at_ms: Some(at_ms + 500),
            exit_net_lamports: Some(987_500 - 83_436),
            net_pnl_lamports: Some(entry_net),
            refusal_reason: None,
        }],
    }];
    let oracle = AccountingOracle::new(0);
    let strategy_name = format!("FOLLOW_DELAY_{delay_ms}MS");
    let receipt = build_receipt(
        "wallet_follower_v1",
        "SYNTHETIC_OR_OBSERVED_QUOTES",
        "QUOTES_AS_PROVIDED",
        "signals+quotes",
        0,
        &oracle,
        &Default::default(),
        0,
        0,
        0,
        0,
        vec![arm(&strategy_name, entry_net), s00()],
        vec!["api test fixture".into()],
    );
    let trades = serde_json::to_string_pretty(&runs).unwrap();
    write_bundle(dir, &receipt, &trades, "", "{}").unwrap();
}

async fn fixture_app() -> (SharedDb, Vec<String>) {
    let tmp = tempfile::tempdir().unwrap();
    let corpus = tmp.path().join("corpus");
    std::fs::create_dir_all(corpus.join("runA")).unwrap();
    std::fs::create_dir_all(corpus.join("runB")).unwrap();
    follower_bundle(&corpus.join("runA"), 500, -83_436, 1_772_236_000_000);
    follower_bundle(&corpus.join("runB"), 60_000, -91_000, 1_772_240_000_000);
    // Keep the tempdir alive for the process lifetime (small local test).
    std::mem::forget(tmp);

    let db_path = corpus.join("cache.sqlite3");
    let conn = open_db(&db_path).unwrap();
    import_corpus(&conn, &Roots::from_dirs([&corpus]), BUILT_AT).unwrap();
    // Select runs by their strategy arms — content-hash id order is not
    // semantically meaningful and must never be assumed.
    let id_for = |strategy: &str| -> String {
        let id: String = conn
            .query_row(
                "SELECT run_id FROM run_comparisons WHERE strategy = ?1 LIMIT 1",
                rusqlite::params![strategy],
                |r| r.get(0),
            )
            .unwrap();
        id
    };
    let id_a = id_for("FOLLOW_DELAY_500MS");
    let id_b = id_for("FOLLOW_DELAY_60000MS");
    (Arc::new(Mutex::new(conn)), vec![id_a, id_b])
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let host_only = Request::builder()
        .method("GET")
        .uri(uri)
        .header("host", "127.0.0.1:7433")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(host_only).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// T-RUN1: run index + detail serve the verified fields.
#[tokio::test]
async fn t_run1_index_and_detail() {
    let (db, ids) = fixture_app().await;
    let app = router(db);
    let (code, body) = get(&app, "/api/v1/runs").await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(body["data_status"], "OK");
    let runs = body["payload"].as_array().unwrap();
    assert_eq!(runs.len(), 2);
    assert!(runs.iter().all(|r| r["engine"] == "wallet_follower_v1"));
    assert!(runs.iter().all(|r| r["verdict"] == "EDGE_NOT_DEMONSTRATED"));

    let (code, detail) = get(&app, &format!("/api/v1/runs/{}", ids[0])).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(detail["payload"]["oracle_audit"], "UNAVAILABLE"); // audit.json is {} (D-4)
    assert!(detail["payload"]["limitations"].as_array().unwrap().len() == 1);
}

/// T-PNL1: money is decimal strings, values exact vs the sealed receipt,
/// S00 row present with NO_TRADE_BASELINE.
#[tokio::test]
async fn t_pnl1_summary_money_strings_exact() {
    let (db, ids) = fixture_app().await;
    let app = router(db);
    let (code, body) = get(&app, &format!("/api/v1/pnl/summary?run={}", ids[0])).await;
    assert_eq!(code, StatusCode::OK);
    let p = &body["payload"];
    assert_eq!(p["net_pnl_lamports"].as_str().unwrap(), "0"); // run-level oracle zeroed (wallet-follow)
    assert!(p["deposits_are_not_pnl"].as_bool().unwrap());
    let comp = p["comparison"].as_array().unwrap();
    let s00 = comp.iter().find(|c| c["strategy"] == "S00").unwrap();
    assert_eq!(s00["status"], "NO_TRADE_BASELINE");
    assert_eq!(s00["net_pnl_lamports"].as_str().unwrap(), "0");
    let arm500 = comp
        .iter()
        .find(|c| c["strategy"] == "FOLLOW_DELAY_500MS")
        .unwrap();
    assert_eq!(arm500["net_pnl_lamports"].as_str().unwrap(), "-83436");
    // Every *_lamports value is a JSON string (T-MON1 at the wire level).
    assert!(p["ending_nav_lamports"].is_string());
    assert!(p["unknown_at_risk_lamports"].is_string());
}

/// T-NAV1: no NAV-series field on summary; portfolio carries explicit
/// UNAVAILABLE markers.
#[tokio::test]
async fn t_nav1_no_nav_series_anywhere() {
    let (db, ids) = fixture_app().await;
    let app = router(db);
    let (_, summary) = get(&app, &format!("/api/v1/pnl/summary?run={}", ids[0])).await;
    let raw = serde_json::to_string(&summary).unwrap();
    assert!(
        !raw.contains("nav_series"),
        "summary must not imply a NAV series"
    );

    let (_, portfolio) = get(&app, &format!("/api/v1/portfolio?run={}", ids[0])).await;
    assert_eq!(portfolio["payload"]["nav_series"], "UNAVAILABLE");
    assert_eq!(portfolio["payload"]["mark_model"], "UNAVAILABLE");
    assert_eq!(portfolio["payload"]["usd_value"], "UNAVAILABLE");
}

/// T-CAL7 (API): calendar for the twin arm reconciles; tz shifts labels
/// without changing sums; research-style absent → UNAVAILABLE envelope.
#[tokio::test]
async fn t_cal7_calendar_endpoint() {
    let (db, ids) = fixture_app().await;
    let app = router(db);
    let uri = format!(
        "/api/v1/pnl/calendar?run={}&strategy=FOLLOW_DELAY_500MS",
        ids[0]
    );
    let (code, cal) = get(&app, &uri).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(cal["data_status"], "OK");
    assert_eq!(
        cal["payload"]["scope"],
        "realized_pnl_closed_positions_only"
    );
    assert_eq!(
        cal["payload"]["reconciled_total_lamports"]
            .as_str()
            .unwrap(),
        "-83436"
    );
    assert_eq!(
        cal["payload"]["comparison_row_net_pnl_lamports"]
            .as_str()
            .unwrap(),
        "-83436"
    );
    assert_eq!(cal["payload"]["reconciles_to_comparison"], true);
    assert_eq!(cal["payload"]["nav_series"], "UNAVAILABLE");
    assert_eq!(cal["payload"]["daily_returns"], "UNAVAILABLE");

    // Jakarta offset shifts the 23:xxZ entry across the local date line.
    let (_, jkt) = get(&app, &format!("{uri}&tz=Asia/Jakarta")).await;
    assert_eq!(jkt["payload"]["tz_label"], "UTC+07:00");
    assert_eq!(
        jkt["payload"]["reconciled_total_lamports"]
            .as_str()
            .unwrap(),
        "-83436"
    );

    // Unknown tz → 400; missing strategy → 400.
    let (code, _) = get(&app, &format!("{uri}&tz=Europe/Berlin")).await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    let (code, _) = get(&app, &format!("/api/v1/pnl/calendar?run={}", ids[0])).await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
}

/// T-ISO1/T-ISO2: runs are isolated; no endpoint exposes a cross-run total.
#[tokio::test]
async fn t_iso_runs_never_summed() {
    let (db, ids) = fixture_app().await;
    let app = router(db);
    // Missing run param → 400 with the explicit isolation note.
    let (code, err) = get(&app, "/api/v1/pnl/summary").await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert!(err["error"].as_str().unwrap().contains("isolated"));

    // Unknown run id → 404.
    let (code, _) = get(&app, "/api/v1/pnl/summary?run=deadbeefdeadbeef").await;
    assert_eq!(code, StatusCode::NOT_FOUND);

    // Strategies view: per-run rows, no corpus total key anywhere.
    let (_, strategies) = get(&app, "/api/v1/strategies").await;
    let raw = serde_json::to_string(&strategies).unwrap();
    assert!(!raw.contains("total_net_pnl"), "cross-run total leaked");
    assert!(!raw.contains("aggregate"), "cross-run aggregate leaked");
    let rows = strategies["payload"]["strategies"].as_array().unwrap();
    let s00 = rows.iter().find(|r| r["strategy"] == "S00").unwrap();
    assert_eq!(s00["latest_net_pnl_lamports"].as_str().unwrap(), "0");

    // Trades paging: runA rows only.
    let (_, trades) = get(&app, &format!("/api/v1/trades?run={}", ids[0])).await;
    let rows = trades["payload"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["strategy"], "FOLLOW_DELAY_500MS");
}

/// T-CC1: runtime status is honestly OFFLINE with the producer note.
#[tokio::test]
async fn t_cc1_status_offline_truth() {
    let (db, _) = fixture_app().await;
    let app = router(db);
    let (code, body) = get(&app, "/api/v1/status").await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(body["payload"]["runtime_state"], "OFFLINE");
    assert!(body["payload"]["runtime_note"]
        .as_str()
        .unwrap()
        .contains("heartbeat"));
    let (_, health) = get(&app, "/api/v1/health").await;
    assert_eq!(health["status"], "OK");
}

/// T-EVT1: SSE endpoint is 501 until a heartbeat producer exists.
#[tokio::test]
async fn t_evt1_events_not_implemented() {
    let (db, _) = fixture_app().await;
    let app = router(db);
    let (code, body) = get(&app, "/api/v1/events").await;
    assert_eq!(code, StatusCode::NOT_IMPLEMENTED);
    assert!(body["reason"].as_str().unwrap().contains("heartbeat"));
}

/// Path-style aliases serve identically to query-style (REQUIREMENTS §5).
#[tokio::test]
async fn path_aliases_match_query_style() {
    let (db, ids) = fixture_app().await;
    let app = router(db);
    let (_, a) = get(&app, &format!("/api/v1/pnl/summary?run={}", ids[0])).await;
    let (_, b) = get(&app, &format!("/api/v1/runs/{}/pnl/summary", ids[0])).await;
    assert_eq!(a, b);
    let _ = HashMap::<String, String>::new();
}
