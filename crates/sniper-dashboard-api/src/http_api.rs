//! HTTP surface: GET-only routes, Host/Origin enforcement, typed envelopes.
//! Money serializes as decimal strings; unknown runs 404; bad params 400;
//! data-level issues stay 200 with an honest `data_status`.

use std::sync::{Arc, Mutex};

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::routing::get;
use axum::{Json, Router};
use rusqlite::Connection;
use serde_json::{json, Value};

use sniper_observability::queries::{self, tz_offset_for, DataStatus, Stamped, TradeMode};

pub type SharedDb = Arc<Mutex<Connection>>;

pub fn router(db: SharedDb) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/status", get(status))
        .route("/api/v1/meta", get(meta))
        .route("/api/v1/runs", get(runs))
        .route("/api/v1/runs/{id}", get(run_detail))
        .route("/api/v1/pnl/summary", get(pnl_summary_query))
        .route("/api/v1/runs/{id}/pnl/summary", get(pnl_summary_path))
        .route("/api/v1/pnl/calendar", get(calendar_query))
        .route("/api/v1/runs/{id}/pnl/calendar", get(calendar_path))
        .route("/api/v1/portfolio", get(portfolio_query))
        .route("/api/v1/runs/{id}/portfolio", get(portfolio_path))
        .route("/api/v1/positions", get(positions_query))
        .route("/api/v1/runs/{id}/positions", get(positions_path))
        .route("/api/v1/trades", get(trades_query))
        .route("/api/v1/runs/{id}/trades", get(trades_path))
        .route("/api/v1/strategies", get(strategies))
        .route("/api/v1/events", get(events_not_implemented))
        .layer(middleware::from_fn(host_origin_guard))
        .with_state(db)
}

/// DA-1/DA-2: loopback trust boundary. Host must be a loopback authority;
/// Origin/Referer, when present, must be same-machine. No CORS headers are
/// ever emitted (browsers refuse cross-origin reads by default).
async fn host_origin_guard(
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Result<axum::response::Response, StatusCode> {
    let host_ok = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .map(|h| {
            let authority = h.rsplit_once(':').map_or(h, |(a, _)| a);
            authority == "127.0.0.1" || authority == "localhost" || authority == "[::1]"
        })
        .unwrap_or(false);
    if !host_ok {
        return Err(StatusCode::FORBIDDEN);
    }
    for header in ["origin", "referer"] {
        if let Some(v) = headers.get(header).and_then(|v| v.to_str().ok()) {
            let same_machine = v.starts_with("http://127.0.0.1")
                || v.starts_with("http://localhost")
                || v.starts_with("http://[::1]");
            if !same_machine {
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }
    Ok(next.run(request).await)
}

fn envelope(as_of: &str, status: DataStatus, reason: Option<String>, payload: Value) -> Value {
    json!({
        "as_of": as_of,
        "data_status": status.as_str(),
        "data_status_reason": reason,
        "payload": payload,
    })
}

fn stamped_envelope<T>(s: Stamped<T>, to_payload: impl FnOnce(T) -> Value) -> (StatusCode, Value) {
    let as_of = s.as_of.clone();
    match s.value {
        Some(v) => (
            StatusCode::OK,
            envelope(&as_of, DataStatus::Ok, None, to_payload(v)),
        ),
        None => (
            StatusCode::OK,
            envelope(&as_of, s.status, s.reason, Value::Null),
        ),
    }
}

fn with_db<T>(db: &SharedDb, f: impl FnOnce(&Connection) -> T) -> T {
    let conn = db.lock().expect("projection db mutex");
    f(&conn)
}

// ---- handlers ----

async fn health(State(db): State<SharedDb>) -> (StatusCode, Json<Value>) {
    let schema = with_db(&db, |c| {
        c.query_row(
            "SELECT value FROM projection_meta WHERE key='schema_version'",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_default()
    });
    (
        StatusCode::OK,
        Json(json!({
            "status": "OK",
            "service": "sniper-dashboard-api",
            "projection_schema_version": schema,
            "note": "API process liveness only; SniperFusion runtime state is /api/v1/status"
        })),
    )
}

async fn status(State(db): State<SharedDb>) -> (StatusCode, Json<Value>) {
    let (as_of, corpus) = with_db(&db, |c| (queries::built_at(c), queries::corpus_status(c)));
    (
        StatusCode::OK,
        Json(envelope(
            &as_of,
            DataStatus::Ok,
            None,
            json!({
                "runtime_state": corpus.runtime_state,
                "runtime_note": corpus.runtime_note,
                "runs": corpus.runs,
                "quarantined_artifacts": corpus.quarantined_artifacts,
                "coverage_windows": corpus.coverage_windows,
            }),
        )),
    )
}

async fn meta(State(db): State<SharedDb>) -> (StatusCode, Json<Value>) {
    let (as_of, roots, runs) = with_db(&db, |c| {
        let roots = c
            .query_row(
                "SELECT value FROM projection_meta WHERE key='roots'",
                [],
                |r| r.get::<_, String>(0),
            )
            .unwrap_or_default();
        (queries::built_at(c), roots, queries::corpus_status(c).runs)
    });
    (
        StatusCode::OK,
        Json(envelope(
            &as_of,
            DataStatus::Ok,
            None,
            json!({
                "live_trading_enabled": false,
                "currency": "SOL-lamports",
                "money_format": "decimal string of integer lamports",
                "allowlisted_roots": roots.split(':').collect::<Vec<_>>(),
                "runs": runs,
            }),
        )),
    )
}

async fn runs(
    State(db): State<SharedDb>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let mode = match parse_mode(&params) {
        Ok(m) => m,
        Err(resp) => return resp,
    };
    let (as_of, list) = with_db(&db, |c| {
        (queries::built_at(c), queries::list_runs_in(c, mode))
    });
    (
        StatusCode::OK,
        Json(envelope(
            &as_of,
            DataStatus::Ok,
            None,
            json!(list
                .into_iter()
                .map(|r| json!({
                    "id": r.id, "mode": r.mode.as_str(), "engine": r.engine,
                    "evidence_class": r.evidence_class,
                    "proof_state": r.proof_state, "completeness": r.completeness,
                    "verdict": r.verdict,
                    // Per-run scalars for the labeled all-runs overview;
                    // each row stands alone (no cross-run sums, ever).
                    "net_pnl_lamports": r.net_pnl_lamports.to_string(),
                    "ending_nav_lamports": r.ending_nav_lamports.to_string(),
                    "external_deposits_lamports": r.external_deposits_lamports.to_string(),
                    "open_positions": r.open_positions,
                    "has_s00_baseline": r.has_s00_baseline,
                }))
                .collect::<Vec<_>>()),
        )),
    )
}

fn run_detail_inner(db: &SharedDb, id: &str) -> (StatusCode, Json<Value>) {
    let stamped = with_db(db, |c| queries::run_detail(c, &queries::built_at(c), id));
    let (code, body) = stamped_envelope(stamped, |d| {
        json!({
            "id": d.entry.id,
            "mode": d.entry.mode.as_str(),
            "engine": d.entry.engine,
            "evidence_class": d.entry.evidence_class,
            "proof_state": d.entry.proof_state,
            "completeness": d.entry.completeness,
            "verdict": d.entry.verdict,
            "artifact_sha256": d.entry.artifact_sha256,
            "dataset_sha256": d.dataset_sha256,
            "dataset_sha_is_hash": d.dataset_sha_is_hash,
            "limitations": d.limitations,
            "oracle_audit": match d.audit_ok {
                Some(true) => "PASS".to_string(),
                Some(false) => "FAIL".to_string(),
                None => "UNAVAILABLE".to_string(),
            },
            "provenance": {"source_path": format!("runs/{id}/run.json"), "source_sha256": d.entry.artifact_sha256},
        })
    });
    // Unknown ids are 404 at the transport layer.
    if body["data_status"] == "UNAVAILABLE"
        && body["data_status_reason"]
            .as_str()
            .map(|r| r.contains("unknown run id"))
            .unwrap_or(false)
    {
        return (StatusCode::NOT_FOUND, Json(body));
    }
    (code, Json(body))
}

async fn run_detail(
    State(db): State<SharedDb>,
    Path(id): Path<String>,
) -> (StatusCode, Json<Value>) {
    run_detail_inner(&db, &id)
}

fn resolve_run_id(
    db: &SharedDb,
    q_run: Option<String>,
    mode: Option<TradeMode>,
) -> Result<String, (StatusCode, Json<Value>)> {
    match q_run {
        Some(id) => {
            if let Some(mode) = mode {
                // DASH-03 T-MODE*: mode isolation enforced at the API
                // boundary — a run of the other scope is refused outright.
                let guard = with_db(db, |c| queries::ensure_run_mode(c, &id, mode));
                if let Err((status, reason)) = guard {
                    let body = json!({
                        "error": "mode isolation refused this run",
                        "data_status": status.as_str(),
                        "reason": reason,
                        "requested_mode": mode.as_str(),
                    });
                    return Err((StatusCode::NOT_FOUND, Json(body)));
                }
            }
            let exists = with_db(db, |c| queries::list_runs(c).iter().any(|r| r.id == id));
            if exists {
                Ok(id)
            } else {
                let body = json!({
                    "error": "unknown or missing run id",
                    "available_runs": with_db(db, |c| queries::list_runs(c).into_iter().map(|r| r.id).collect::<Vec<_>>()),
                });
                Err((StatusCode::NOT_FOUND, Json(body)))
            }
        }
        None => {
            let ids = with_db(db, |c| {
                queries::list_runs(c)
                    .into_iter()
                    .map(|r| r.id)
                    .collect::<Vec<_>>()
            });
            Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "run parameter required (?run=<id>); runs are isolated — there is no corpus-level default",
                    "available_runs": ids,
                })),
            ))
        }
    }
}

/// Parse the optional `mode` parameter; invalid values are client errors
/// (never silently ignored — that would blur the isolation contract).
fn parse_mode(
    params: &std::collections::HashMap<String, String>,
) -> Result<Option<TradeMode>, (StatusCode, Json<Value>)> {
    match params.get("mode").map(|s| s.as_str()) {
        None => Ok(None),
        Some("PAPER") => Ok(Some(TradeMode::Paper)),
        Some("REAL_MONEY") => Ok(Some(TradeMode::RealMoney)),
        Some(other) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("mode must be PAPER or REAL_MONEY, got {other}")})),
        )),
    }
}

fn pnl_summary_payload(s: queries::PnlSummary) -> Value {
    json!({
        "run_id": s.run_id,
        "engine": s.engine,
        "proof_state": s.proof_state,
        "currency": s.currency,
        "starting_nav_lamports": s.starting_nav_lamports.to_string(),
        "external_deposits_lamports": s.external_deposits_lamports.to_string(),
        "external_withdrawals_lamports": s.external_withdrawals_lamports.to_string(),
        "ending_nav_lamports": s.ending_nav_lamports.to_string(),
        "realized_pnl_lamports": s.realized_pnl_lamports.to_string(),
        "unrealized_pnl_lamports": s.unrealized_pnl_lamports.to_string(),
        "net_pnl_lamports": s.net_pnl_lamports.to_string(),
        "max_drawdown_lamports": s.max_drawdown_lamports.to_string(),
        "recoverable_rent_lamports": s.recoverable_rent_lamports.to_string(),
        "open_inventory_cost_basis_lamports": s.open_inventory_cost_basis_lamports.to_string(),
        "open_positions": s.open_positions,
        "unexitable_exits": s.unexitable_exits,
        "unknown_at_risk_lamports": s.unknown_at_risk_lamports.to_string(),
        "deposits_are_not_pnl": true,
        "fees_by_kind_lamports": s.fees_by_kind_lamports.iter()
            .map(|(k, v)| (k.clone(), Value::String(v.to_string()))).collect::<serde_json::Map<String, Value>>(),
        "comparison": s.comparison.iter().map(|c| json!({
            "strategy": c.strategy,
            "net_pnl_lamports": c.net_pnl_lamports.to_string(),
            "incremental_net_edge_vs_no_trade": c.incremental_net_edge_vs_no_trade.to_string(),
            "max_drawdown_lamports": c.max_drawdown_lamports.to_string(),
            "open_positions": c.open_positions,
            "status": c.status,
        })).collect::<Vec<_>>(),
        "verdict": s.verdict,
    })
}

fn pnl_summary_for(
    db: &SharedDb,
    run: Option<String>,
    mode: Option<TradeMode>,
) -> (StatusCode, Json<Value>) {
    let id = match resolve_run_id(db, run, mode) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let stamped = with_db(db, |c| queries::pnl_summary(c, &queries::built_at(c), &id));
    let (code, body) = stamped_envelope(stamped, pnl_summary_payload);
    (code, Json(body))
}

async fn pnl_summary_query(
    State(db): State<SharedDb>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let mode = match parse_mode(&params) {
        Ok(m) => m,
        Err(resp) => return resp,
    };
    pnl_summary_for(&db, params.get("run").cloned(), mode)
}

async fn pnl_summary_path(
    State(db): State<SharedDb>,
    Path(id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let mode = match parse_mode(&params) {
        Ok(m) => m,
        Err(resp) => return resp,
    };
    pnl_summary_for(&db, Some(id), mode)
}

fn calendar_for(
    db: &SharedDb,
    run: Option<String>,
    strategy: Option<String>,
    tz: Option<String>,
    mode: Option<TradeMode>,
) -> (StatusCode, Json<Value>) {
    let id = match resolve_run_id(db, run, mode) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let Some(strategy) = strategy else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "strategy parameter required (?strategy=FOLLOW_DELAY_500MS)"})),
        );
    };
    let tz_name = tz.as_deref().unwrap_or("UTC");
    let Some(offset) = tz_offset_for(tz_name) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                json!({"error": format!("tz must be one of the allowlisted fixed-offset zones, got {tz_name}")}),
            ),
        );
    };
    let stamped = with_db(db, |c| {
        queries::calendar(c, &queries::built_at(c), &id, &strategy, offset)
    });
    let (code, body) = stamped_envelope(stamped, |cal| {
        json!({
            "run_id": cal.run_id,
            "strategy": cal.strategy,
            "tz": tz_name,
            "tz_label": cal.projection.tz_label,
            "scope": cal.projection.scope,
            "scope_note": "realized PnL of closed positions ONLY — not full economic PnL",
            "days": cal.projection.buckets.iter().map(|b| json!({
                "day": b.day,
                "realized_net_pnl_lamports": b.realized_net_pnl_lamports.to_string(),
                "closed_count": b.closed_count,
            })).collect::<Vec<_>>(),
            "closed_timestamped": cal.projection.closed_timestamped,
            "untimestamped_count": cal.projection.untimestamped_count,
            "open_positions": cal.projection.open_positions,
            "reconciled_total_lamports": cal.projection.reconciled_total_lamports.to_string(),
            "closed_total_lamports": cal.projection.closed_total_lamports.to_string(),
            "comparison_row_net_pnl_lamports": cal.comparison_row_net_pnl_lamports.map(|v| v.to_string()),
            "reconciles_to_comparison": cal.reconciles_to_comparison,
            "nav_series": cal.nav_series,
            "nav_series_reason": "no NAV/mark producer exists (NEEDS_NEW_TELEMETRY)",
            "daily_returns": cal.daily_returns,
            "daily_returns_reason": "requires NAV marks per period; none emitted",
        })
    });
    (code, Json(body))
}

async fn calendar_query(
    State(db): State<SharedDb>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let mode = match parse_mode(&params) {
        Ok(m) => m,
        Err(resp) => return resp,
    };
    calendar_for(
        &db,
        params.get("run").cloned(),
        params.get("strategy").cloned(),
        params.get("tz").cloned(),
        mode,
    )
}

async fn calendar_path(
    State(db): State<SharedDb>,
    Path(id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let mode = match parse_mode(&params) {
        Ok(m) => m,
        Err(resp) => return resp,
    };
    calendar_for(
        &db,
        Some(id),
        params.get("strategy").cloned(),
        params.get("tz").cloned(),
        mode,
    )
}

fn portfolio_for(
    db: &SharedDb,
    run: Option<String>,
    mode: Option<TradeMode>,
) -> (StatusCode, Json<Value>) {
    let id = match resolve_run_id(db, run, mode) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let stamped = with_db(db, |c| queries::positions(c, &queries::built_at(c), &id));
    let (code, body) = stamped_envelope(stamped, |p| {
        json!({
            "run_id": p.run_id,
            "ending_nav_lamports": p.ending_nav_lamports.to_string(),
            "open_positions": p.open_positions,
            "open_inventory_cost_basis_lamports": p.open_inventory_cost_basis_lamports.to_string(),
            "open_inventory_note": "inventory at risk, valued at cost basis; NOT cash and NOT exit value",
            "recoverable_rent_lamports": p.recoverable_rent_lamports.to_string(),
            "unknown_at_risk_lamports": p.unknown_at_risk_lamports.to_string(),
            "unexitable_exits": p.unexitable_exits,
            "open_entries": p.open_entries.iter().map(|e| json!({
                "strategy": e.strategy, "mint": e.mint,
                "entry_at_ms": e.entry_at_ms,
                "entry_effective_lamports": e.entry_effective_lamports.map(|v| v.to_string()),
                "tokens": e.tokens,
            })).collect::<Vec<_>>(),
            "nav_series": p.nav_series,
            "mark_model": p.mark_model,
            "usd_value": "UNAVAILABLE",
            "usd_reason": "no FX/price source exists in SniperFusion; USD display is prohibited by construction",
        })
    });
    (code, Json(body))
}

async fn portfolio_query(
    State(db): State<SharedDb>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let mode = match parse_mode(&params) {
        Ok(m) => m,
        Err(resp) => return resp,
    };
    portfolio_for(&db, params.get("run").cloned(), mode)
}

async fn portfolio_path(
    State(db): State<SharedDb>,
    Path(id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let mode = match parse_mode(&params) {
        Ok(m) => m,
        Err(resp) => return resp,
    };
    portfolio_for(&db, Some(id), mode)
}

fn positions_for(
    db: &SharedDb,
    run: Option<String>,
    mode: Option<TradeMode>,
) -> (StatusCode, Json<Value>) {
    portfolio_for(db, run, mode)
}

async fn positions_query(
    State(db): State<SharedDb>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let mode = match parse_mode(&params) {
        Ok(m) => m,
        Err(resp) => return resp,
    };
    positions_for(&db, params.get("run").cloned(), mode)
}

async fn positions_path(
    State(db): State<SharedDb>,
    Path(id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let mode = match parse_mode(&params) {
        Ok(m) => m,
        Err(resp) => return resp,
    };
    positions_for(&db, Some(id), mode)
}

fn trades_for(
    db: &SharedDb,
    run: Option<String>,
    cursor: i64,
    limit: i64,
    mode: Option<TradeMode>,
) -> (StatusCode, Json<Value>) {
    let id = match resolve_run_id(db, run, mode) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let stamped = with_db(db, |c| {
        queries::trades_page(c, &queries::built_at(c), &id, cursor, limit)
    });
    let (code, body) = stamped_envelope(stamped, |page| {
        json!({
            "run_id": page.run_id,
            "rows": page.rows.iter().map(|r| json!({
                "ord": r.ord, "strategy": r.strategy, "kind": r.kind, "mint": r.mint,
                "slot": r.slot, "at_ms": r.at_ms,
                "net_pnl_lamports": r.net_pnl_lamports.map(|v| v.to_string()),
                "open": r.open,
            })).collect::<Vec<_>>(),
            "next_cursor": page.next_cursor,
        })
    });
    (code, Json(body))
}

async fn trades_query(
    State(db): State<SharedDb>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let mode = match parse_mode(&params) {
        Ok(m) => m,
        Err(resp) => return resp,
    };
    let cursor = params
        .get("cursor")
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let limit = params
        .get("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(200);
    trades_for(&db, params.get("run").cloned(), cursor, limit, mode)
}

async fn trades_path(
    State(db): State<SharedDb>,
    Path(id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let mode = match parse_mode(&params) {
        Ok(m) => m,
        Err(resp) => return resp,
    };
    let cursor = params
        .get("cursor")
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let limit = params
        .get("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(200);
    trades_for(&db, Some(id), cursor, limit, mode)
}

async fn strategies(
    State(db): State<SharedDb>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let mode = match parse_mode(&params) {
        Ok(m) => m,
        Err(resp) => return resp,
    };
    let (as_of, list) = with_db(&db, |c| (queries::built_at(c), queries::strategies(c)));
    // Mode scoping is a display classification over sealed proof states.
    let allowed: Vec<_> = match mode {
        None => list,
        Some(m) => list
            .into_iter()
            .filter(|srow| {
                let proof = with_db(&db, |c| {
                    c.query_row(
                        "SELECT proof_state FROM runs WHERE id = ?1",
                        rusqlite::params![srow.latest_run_id],
                        |r| r.get::<_, String>(0),
                    )
                    .unwrap_or_default()
                });
                TradeMode::of_proof_state(&proof) == m
            })
            .collect(),
    };
    let list = allowed;
    (
        StatusCode::OK,
        Json(envelope(
            &as_of,
            DataStatus::Ok,
            None,
            json!({
                "note": "per-strategy standings from machine receipts; rows are per-run, never summed across runs",
                "strategies": list.iter().map(|s| json!({
                    "strategy": s.strategy,
                    "latest_net_pnl_lamports": s.latest_net_pnl_lamports.to_string(),
                    "latest_run_id": s.latest_run_id,
                    "latest_status": s.status,
                    "appearances": s.appearances,
                })).collect::<Vec<_>>(),
            }),
        )),
    )
}

/// SSE stream is DASH-04 work: no heartbeat producer exists, so the honest
/// answer today is 501 — never a fabricated live stream.
async fn events_not_implemented() -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "error": "NOT_IMPLEMENTED",
            "reason": "no runtime heartbeat producer exists (DASH-04); historical receipts never prove liveness",
        })),
    )
}
