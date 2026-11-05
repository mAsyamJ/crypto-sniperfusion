//! Read models over the projection. These are the ONLY shapes the API
//! serves. Money stays integer here; the API layer serializes to decimal
//! strings. Every query returns a typed status — never a silent default.

use rusqlite::{params, Connection};
use sniper_strategy::calendar::{bucket_daily, CalendarProjection, CalendarRecord};
use sniper_strategy::receipt::ComparisonRow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataStatus {
    Ok,
    Unavailable,
    Stale,
    Blocked,
    Reconciling,
    SourceMismatch,
    InsufficientSample,
}

impl DataStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataStatus::Ok => "OK",
            DataStatus::Unavailable => "UNAVAILABLE",
            DataStatus::Stale => "STALE",
            DataStatus::Blocked => "BLOCKED",
            DataStatus::Reconciling => "RECONCILING",
            DataStatus::SourceMismatch => "SOURCE_MISMATCH",
            DataStatus::InsufficientSample => "INSUFFICIENT_SAMPLE",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Stamped<T> {
    pub as_of: String,
    pub status: DataStatus,
    pub reason: Option<String>,
    pub value: Option<T>,
}

impl<T> Stamped<T> {
    pub fn ok(as_of: String, value: T) -> Self {
        Self {
            as_of,
            status: DataStatus::Ok,
            reason: None,
            value: Some(value),
        }
    }
    pub fn err(as_of: String, status: DataStatus, reason: impl Into<String>) -> Self {
        Self {
            as_of,
            status,
            reason: Some(reason.into()),
            value: None,
        }
    }
}

/// Presentation-level trading mode (operator's DASH-03 contract). Derived
/// ONLY from the sealed `proof_state`: `LIVE_CONFIRMED` is the sole proof
/// state that means real money (evidence ladder, `docs/GLOSSARY.md`).
/// Everything else — SYNTHETIC, FORK_MECHANICS, SHADOW_ESTIMATE — is paper.
/// The mapping is a display classification; the underlying evidence labels
/// are never rewritten.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeMode {
    Paper,
    RealMoney,
}

impl TradeMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            TradeMode::Paper => "PAPER",
            TradeMode::RealMoney => "REAL_MONEY",
        }
    }

    /// Parse the API parameter form; anything else is a client error.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "PAPER" => Some(TradeMode::Paper),
            "REAL_MONEY" => Some(TradeMode::RealMoney),
            _ => None,
        }
    }

    pub fn of_proof_state(proof_state: &str) -> Self {
        if proof_state == "LIVE_CONFIRMED" {
            TradeMode::RealMoney
        } else {
            TradeMode::Paper
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunIndexEntry {
    pub mode: TradeMode,
    pub id: String,
    pub engine: String,
    pub evidence_class: String,
    pub proof_state: String,
    pub completeness: String,
    pub verdict: String,
    pub artifact_sha256: String,
}

pub fn built_at(conn: &Connection) -> String {
    conn.query_row(
        "SELECT value FROM projection_meta WHERE key='built_at'",
        [],
        |r| r.get(0),
    )
    .unwrap_or_default()
}

pub fn list_runs(conn: &Connection) -> Vec<RunIndexEntry> {
    list_runs_in(conn, None)
}

/// Mode-scoped run index (DASH-03: isolation enforced at the query layer).
pub fn list_runs_in(conn: &Connection, mode: Option<TradeMode>) -> Vec<RunIndexEntry> {
    let mut stmt = match conn.prepare(
        "SELECT id, engine, evidence_class, proof_state, completeness, verdict, artifact_sha256
         FROM runs ORDER BY id",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = stmt.query_map([], |r| {
        let proof_state: String = r.get(3)?;
        Ok((
            RunIndexEntry {
                mode: TradeMode::of_proof_state(&proof_state),
                id: r.get(0)?,
                engine: r.get(1)?,
                evidence_class: r.get(2)?,
                proof_state,
                completeness: r.get(4)?,
                verdict: r.get(5)?,
                artifact_sha256: r.get(6)?,
            },
            (),
        ))
    });
    rows.map(|it| {
        it.flatten()
            .filter(|(entry, _)| mode.is_none_or(|m| entry.mode == m))
            .map(|(entry, _)| entry)
            .collect()
    })
    .unwrap_or_default()
}

/// Mode guard for run-scoped queries (DASH-03 T-MODE*): a run whose sealed
/// proof state belongs to the other mode is REFUSED — paper records can
/// never be served into a REAL_MONEY scope, and vice versa.
pub fn ensure_run_mode(
    conn: &Connection,
    run_id: &str,
    mode: TradeMode,
) -> Result<(), (DataStatus, String)> {
    let proof_state: Option<String> = conn
        .query_row(
            "SELECT proof_state FROM runs WHERE id = ?1",
            params![run_id],
            |r| r.get(0),
        )
        .ok();
    match proof_state {
        None => Err((DataStatus::Unavailable, format!("unknown run id {run_id}"))),
        Some(ps) => {
            let actual = TradeMode::of_proof_state(&ps);
            if actual == mode {
                Ok(())
            } else {
                Err((
                    DataStatus::Unavailable,
                    format!(
                        "run {run_id} is sealed {ps} → {actual:?} scope; refused in {} scope                          (mode isolation: paper and real money never mix)",
                        mode.as_str()
                    ),
                ))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunDetail {
    pub entry: RunIndexEntry,
    pub limitations: Vec<String>,
    /// None when audit.json was empty (wallet-* bundles, D-4) → UNAVAILABLE.
    pub audit_ok: Option<bool>,
    pub dataset_sha256: String,
    pub dataset_sha_is_hash: bool,
}

pub fn run_detail(conn: &Connection, as_of: &str, run_id: &str) -> Stamped<RunDetail> {
    let entries = list_runs(conn);
    let Some(entry) = entries.into_iter().find(|e| e.id == run_id) else {
        return Stamped::err(
            as_of.to_string(),
            DataStatus::Unavailable,
            format!("unknown run id {run_id}"),
        );
    };
    let row = conn.query_row(
        "SELECT limitations_json, audit_json, dataset_sha256, dataset_sha_is_hash FROM runs WHERE id = ?1",
        params![run_id],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
            ))
        },
    );
    let (limitations_json, audit_json, dataset_sha256, sha_is_hash) = match row {
        Ok(v) => v,
        Err(e) => {
            return Stamped::err(as_of.to_string(), DataStatus::SourceMismatch, e.to_string())
        }
    };
    let limitations: Vec<String> = serde_json::from_str(&limitations_json).unwrap_or_default();
    let audit_ok = if audit_json.is_empty() || audit_json.trim() == "{}" {
        None // wallet-* bundles emit {} (D-4): audit verdict UNAVAILABLE
    } else {
        // Research bundles: map of strategy -> {conservation_ok, cash_matches_engine}.
        serde_json::from_str::<serde_json::Value>(&audit_json)
            .ok()
            .and_then(|v| {
                v.as_object().map(|obj| {
                    obj.values().all(|s| {
                        s.get("conservation_ok")
                            .and_then(|x| x.as_bool())
                            .unwrap_or(false)
                            && s.get("cash_matches_engine")
                                .and_then(|x| x.as_bool())
                                .unwrap_or(false)
                    })
                })
            })
    };
    Stamped::ok(
        as_of.to_string(),
        RunDetail {
            entry,
            limitations,
            audit_ok,
            dataset_sha256,
            dataset_sha_is_hash: sha_is_hash != 0,
        },
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct PnlSummary {
    pub run_id: String,
    pub engine: String,
    pub proof_state: String,
    pub currency: String,
    pub starting_nav_lamports: i64,
    pub external_deposits_lamports: i64,
    pub external_withdrawals_lamports: i64,
    pub ending_nav_lamports: i64,
    pub realized_pnl_lamports: i64,
    pub unrealized_pnl_lamports: i64,
    pub net_pnl_lamports: i64,
    pub max_drawdown_lamports: i64,
    pub recoverable_rent_lamports: i64,
    pub open_inventory_cost_basis_lamports: i64,
    pub open_positions: i64,
    pub unexitable_exits: i64,
    pub unknown_at_risk_lamports: i64,
    pub fees_by_kind_lamports: Vec<(String, i64)>,
    pub comparison: Vec<ComparisonRow>,
    pub verdict: String,
}

/// T-S001: a run whose comparison lacks an S00 row is a broken receipt —
/// fail closed with SOURCE_MISMATCH (a missing baseline voids the view).
pub fn pnl_summary(conn: &Connection, as_of: &str, run_id: &str) -> Stamped<PnlSummary> {
    let row = conn.query_row(
        "SELECT engine, proof_state, starting_nav_lamports, external_deposits_lamports,
                external_withdrawals_lamports, ending_nav_lamports, realized_pnl_lamports,
                unrealized_pnl_lamports, net_pnl_lamports, max_drawdown_lamports,
                recoverable_rent_lamports, open_inventory_cost_basis_lamports, open_positions,
                unexitable_exits, unknown_at_risk_lamports, verdict
         FROM runs WHERE id = ?1",
        params![run_id],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, i64>(7)?,
                r.get::<_, i64>(8)?,
                r.get::<_, i64>(9)?,
                r.get::<_, i64>(10)?,
                r.get::<_, i64>(11)?,
                r.get::<_, i64>(12)?,
                r.get::<_, i64>(13)?,
                r.get::<_, i64>(14)?,
                r.get::<_, String>(15)?,
            ))
        },
    );
    let (
        engine,
        proof_state,
        starting,
        deposits,
        withdrawals,
        ending,
        realized,
        unrealized,
        net,
        maxdd,
        rent,
        basis,
        open_pos,
        unexit,
        unknown,
        verdict,
    ) = match row {
        Ok(v) => v,
        Err(_) => {
            return Stamped::err(
                as_of.to_string(),
                DataStatus::Unavailable,
                format!("unknown run id {run_id}"),
            )
        }
    };
    let mut fees = Vec::new();
    if let Ok(mut stmt) =
        conn.prepare("SELECT kind, lamports FROM run_fees WHERE run_id = ?1 ORDER BY kind")
    {
        if let Ok(rows) = stmt.query_map(params![run_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        }) {
            rows.for_each(|r| {
                if let Ok(v) = r {
                    fees.push(v);
                }
            });
        }
    }
    let mut comparison = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT strategy, net_pnl_lamports, incremental_net_edge, max_drawdown_lamports, open_positions, status
         FROM run_comparisons WHERE run_id = ?1 ORDER BY ord",
    ) {
        if let Ok(rows) = stmt.query_map(params![run_id], |r| {
            Ok(ComparisonRow {
                strategy: r.get(0)?,
                net_pnl_lamports: r.get(1)?,
                incremental_net_edge_vs_no_trade: r.get(2)?,
                max_drawdown_lamports: r.get::<_, i64>(3)? as u64,
                open_positions: r.get::<_, i64>(4)? as u64,
                status: r.get(5)?,
            })
        }) {
            rows.for_each(|r| {
                if let Ok(v) = r { comparison.push(v); }
            });
        }
    }
    if !comparison.iter().any(|c| c.strategy == "S00") {
        return Stamped::err(
            as_of.to_string(),
            DataStatus::SourceMismatch,
            "comparison lacks the mandatory S00 NO_TRADE baseline row (a missing baseline voids the view)",
        );
    }
    Stamped::ok(
        as_of.to_string(),
        PnlSummary {
            run_id: run_id.to_string(),
            engine,
            proof_state,
            currency: "SOL-lamports".into(),
            starting_nav_lamports: starting,
            external_deposits_lamports: deposits,
            external_withdrawals_lamports: withdrawals,
            ending_nav_lamports: ending,
            realized_pnl_lamports: realized,
            unrealized_pnl_lamports: unrealized,
            net_pnl_lamports: net,
            max_drawdown_lamports: maxdd,
            recoverable_rent_lamports: rent,
            open_inventory_cost_basis_lamports: basis,
            open_positions: open_pos,
            unexitable_exits: unexit,
            unknown_at_risk_lamports: unknown,
            fees_by_kind_lamports: fees,
            comparison,
            verdict,
        },
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalendarResponse {
    pub run_id: String,
    pub strategy: String,
    pub projection: CalendarProjection,
    /// Canonical scope target when the strategy is a comparison arm: the
    /// arm's `net_pnl_lamports` from the sealed receipt.
    pub comparison_row_net_pnl_lamports: Option<i64>,
    pub reconciles_to_comparison: Option<bool>,
    /// There is no NAV/mark series producer (P8): these are UNAVAILABLE by
    /// construction, never zero.
    pub nav_series: &'static str,
    pub daily_returns: &'static str,
}

/// Fixed-offset allowlist for the `tz` parameter (whole-hour, no DST).
const TZ_ALLOWLIST: &[(&str, i64)] = &[("UTC", 0), ("Asia/Jakarta", 7 * 3600)];

pub fn tz_offset_for(name: &str) -> Option<i64> {
    TZ_ALLOWLIST
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, off)| *off)
}

pub fn calendar(
    conn: &Connection,
    as_of: &str,
    run_id: &str,
    strategy: &str,
    tz_offset_secs: i64,
) -> Stamped<CalendarResponse> {
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM runs WHERE id = ?1",
            params![run_id],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false);
    if !exists {
        return Stamped::err(
            as_of.to_string(),
            DataStatus::Unavailable,
            format!("unknown run id {run_id}"),
        );
    }
    let mut records = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT at_ms, net_pnl_lamports, open FROM run_trades
         WHERE run_id = ?1 AND strategy = ?2 ORDER BY ord",
    ) {
        if let Ok(rows) = stmt.query_map(params![run_id, strategy], |r| {
            Ok(CalendarRecord {
                exit_at_ms: r.get(0)?,
                net_pnl_lamports: r.get(1)?,
            })
        }) {
            rows.for_each(|r| {
                if let Ok(v) = r {
                    records.push(v);
                }
            });
        }
    }
    // Open rows (open=1, at_ms NULL, net NULL) become open positions by the
    // calendar's own (None, None) rule — no special-casing needed.
    let projection = bucket_daily(strategy, tz_offset_secs, &records);
    if projection.status == sniper_strategy::calendar::CalendarStatus::Unavailable {
        let reason = if records.is_empty() {
            format!("no trade records for strategy {strategy} in this run")
        } else {
            "records carry slots but no producer timestamps (NEEDS_NEW_TELEMETRY)".to_string()
        };
        return Stamped::err(as_of.to_string(), DataStatus::Unavailable, reason);
    }
    let comparison_row_net: Option<i64> = conn
        .query_row(
            "SELECT net_pnl_lamports FROM run_comparisons WHERE run_id = ?1 AND strategy = ?2",
            params![run_id, strategy],
            |r| r.get(0),
        )
        .ok();
    let reconciles = comparison_row_net.map(|canonical| {
        projection.reconciled_total_lamports + open_position_pnl_exclusion(&projection) == canonical
            || projection.closed_total_lamports == canonical
    });
    Stamped::ok(
        as_of.to_string(),
        CalendarResponse {
            run_id: run_id.to_string(),
            strategy: strategy.to_string(),
            projection,
            comparison_row_net_pnl_lamports: comparison_row_net,
            reconciles_to_comparison: reconciles,
            nav_series: "UNAVAILABLE",
            daily_returns: "UNAVAILABLE",
        },
    )
}

/// Open positions are excluded from realized buckets; when the canonical
/// comparison row covers them this stays 0 and reconciliation is direct.
fn open_position_pnl_exclusion(_p: &CalendarProjection) -> i64 {
    0
}

#[derive(Debug, Clone, PartialEq)]
pub struct Positions {
    pub run_id: String,
    pub open_positions: i64,
    pub open_inventory_cost_basis_lamports: i64,
    pub recoverable_rent_lamports: i64,
    pub unknown_at_risk_lamports: i64,
    pub unexitable_exits: i64,
    /// Open follower entries (still held; unpriced by construction).
    pub open_entries: Vec<OpenEntry>,
    pub ending_nav_lamports: i64,
    pub nav_series: &'static str,
    pub mark_model: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenEntry {
    pub strategy: String,
    pub mint: String,
    pub entry_at_ms: Option<u64>,
    pub entry_effective_lamports: Option<i64>,
    pub tokens: Option<i64>,
}

pub fn positions(conn: &Connection, as_of: &str, run_id: &str) -> Stamped<Positions> {
    let row = conn.query_row(
        "SELECT open_positions, open_inventory_cost_basis_lamports, recoverable_rent_lamports,
                unknown_at_risk_lamports, unexitable_exits, ending_nav_lamports
         FROM runs WHERE id = ?1",
        params![run_id],
        |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
            ))
        },
    );
    let (open_positions, basis, rent, unknown, unexit, ending_nav) = match row {
        Ok(v) => v,
        Err(_) => {
            return Stamped::err(
                as_of.to_string(),
                DataStatus::Unavailable,
                format!("unknown run id {run_id}"),
            )
        }
    };
    let mut open_entries = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT strategy, mint, raw FROM run_trades
         WHERE run_id = ?1 AND open = 1 ORDER BY ord",
    ) {
        if let Ok(rows) = stmt.query_map(params![run_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        }) {
            rows.for_each(|r| {
                if let Ok((strategy, mint, raw)) = r {
                    let v: serde_json::Value =
                        serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
                    open_entries.push(OpenEntry {
                        strategy,
                        mint,
                        entry_at_ms: v.get("entry_at_ms").and_then(|x| x.as_u64()),
                        entry_effective_lamports: v
                            .get("entry_effective_lamports")
                            .and_then(|x| x.as_i64()),
                        tokens: v.get("tokens").and_then(|x| x.as_i64()),
                    });
                }
            });
        }
    }
    Stamped::ok(
        as_of.to_string(),
        Positions {
            run_id: run_id.to_string(),
            open_positions,
            open_inventory_cost_basis_lamports: basis,
            recoverable_rent_lamports: rent,
            unknown_at_risk_lamports: unknown,
            unexitable_exits: unexit,
            open_entries,
            ending_nav_lamports: ending_nav,
            nav_series: "UNAVAILABLE",
            mark_model: "UNAVAILABLE",
        },
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct TradePage {
    pub run_id: String,
    pub rows: Vec<TradeRowView>,
    pub next_cursor: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TradeRowView {
    pub ord: i64,
    pub strategy: String,
    pub kind: String,
    pub mint: String,
    pub slot: Option<i64>,
    pub at_ms: Option<i64>,
    pub net_pnl_lamports: Option<i64>,
    pub open: bool,
}

pub fn trades_page(
    conn: &Connection,
    as_of: &str,
    run_id: &str,
    cursor: i64,
    limit: i64,
) -> Stamped<TradePage> {
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM runs WHERE id = ?1",
            params![run_id],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false);
    if !exists {
        return Stamped::err(
            as_of.to_string(),
            DataStatus::Unavailable,
            format!("unknown run id {run_id}"),
        );
    }
    let limit = limit.clamp(1, 500);
    let mut rows = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT ord, strategy, record_kind, mint, slot, at_ms, net_pnl_lamports, open
         FROM run_trades WHERE run_id = ?1 AND ord >= ?2 ORDER BY ord LIMIT ?3",
    ) {
        if let Ok(mapped) = stmt.query_map(params![run_id, cursor, limit], |r| {
            Ok(TradeRowView {
                ord: r.get(0)?,
                strategy: r.get(1)?,
                kind: r.get(2)?,
                mint: r.get(3)?,
                slot: r.get(4)?,
                at_ms: r.get(5)?,
                net_pnl_lamports: r.get(6)?,
                open: r.get::<_, i64>(7)? != 0,
            })
        }) {
            mapped.for_each(|r| {
                if let Ok(v) = r {
                    rows.push(v);
                }
            });
        }
    }
    let next_cursor = if rows.len() as i64 == limit {
        rows.last().map(|r| r.ord + 1)
    } else {
        None
    };
    Stamped::ok(
        as_of.to_string(),
        TradePage {
            run_id: run_id.to_string(),
            rows,
            next_cursor,
        },
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct StrategyStanding {
    pub strategy: String,
    pub latest_net_pnl_lamports: i64,
    pub latest_run_id: String,
    pub status: String,
    pub appearances: i64,
}

/// Corpus-level standings from machine receipts only. S00 is always pinned
/// first (champion until dethroned — SOUL Law 5).
pub fn strategies(conn: &Connection) -> Vec<StrategyStanding> {
    let mut out = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT c.strategy, c.net_pnl_lamports, c.run_id, c.status, COUNT(*) AS n
         FROM run_comparisons c JOIN runs r ON r.id = c.run_id
         GROUP BY c.strategy ORDER BY c.strategy",
    ) {
        if let Ok(rows) = stmt.query_map([], |r| {
            Ok(StrategyStanding {
                strategy: r.get(0)?,
                latest_net_pnl_lamports: r.get(1)?,
                latest_run_id: r.get(2)?,
                status: r.get(3)?,
                appearances: r.get(4)?,
            })
        }) {
            rows.for_each(|r| {
                if let Ok(v) = r {
                    out.push(v);
                }
            });
        }
    }
    out.sort_by(|a, b| {
        let a_s00 = a.strategy == "S00";
        let b_s00 = b.strategy == "S00";
        b_s00.cmp(&a_s00).then(a.strategy.cmp(&b.strategy))
    });
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct CorpusStatus {
    /// No heartbeat producer exists (P10): OFFLINE is the honest state.
    pub runtime_state: &'static str,
    pub runtime_note: &'static str,
    pub runs: i64,
    pub quarantined_artifacts: i64,
    pub coverage_windows: i64,
}

pub fn corpus_status(conn: &Connection) -> CorpusStatus {
    let count = |sql: &str| conn.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap_or(0);
    CorpusStatus {
        runtime_state: "OFFLINE",
        runtime_note:
            "no heartbeat producer exists (DASH-04); historical receipts never prove liveness",
        runs: count("SELECT COUNT(*) FROM runs"),
        quarantined_artifacts: count("SELECT COUNT(*) FROM source_artifacts WHERE status != 'OK'"),
        coverage_windows: count("SELECT COUNT(*) FROM coverage_windows"),
    }
}

pub fn quarantined(conn: &Connection) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT path, kind, status FROM source_artifacts WHERE status != 'OK' ORDER BY path",
    ) {
        if let Ok(rows) = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))) {
            rows.for_each(|r| {
                if let Ok(v) = r {
                    out.push(v);
                }
            });
        }
    }
    out
}

impl<T: std::fmt::Debug> Stamped<T> {
    /// Panics unless the status is OK — test/bin helper, never used to
    /// fabricate values.
    pub fn expect(self) -> T {
        let Self {
            as_of,
            status,
            reason,
            value,
        } = self;
        value.unwrap_or_else(|| panic!("expected OK, got {status:?} ({reason:?}) as_of={as_of}"))
    }
}
