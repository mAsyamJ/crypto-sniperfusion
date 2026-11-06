//! sniper-dashboard-api: the read-only loopback API of the Control Tower
//! (ADR-0013). Serves exactly what `sniper-observability` verified — never
//! computes, never fetches, never mutates. Money crosses the wire as
//! decimal strings; every response carries `as_of` + `data_status`.

use sniper_dashboard_api::http_api;

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use clap::Parser as ClapParser;
use sniper_observability::discovery::Roots;
use sniper_observability::project::{import_corpus, open_db};

/// Local dashboard API. Binds loopback only; refuses anything else.
#[derive(Debug, ClapParser)]
pub struct Args {
    /// SQLite projection path (disposable cache).
    #[arg(
        long,
        default_value = "harness/runs/dashboard-cache/projection.sqlite3"
    )]
    db: String,
    /// Colon-separated allowlisted corpus roots to import from.
    #[arg(
        long,
        default_value = "harness/runs/wallet-alpha:harness/runs/launch-cohort:harness/runs:harness/runs/harness2"
    )]
    roots: String,
    /// Bind address; loopback only (refused otherwise).
    #[arg(long, default_value = "127.0.0.1:7433")]
    addr: String,
    /// Force a full projection rebuild before serving.
    #[arg(long)]
    rebuild: bool,
}

fn utc_now_rfc3339() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn main() {
    let args = Args::parse();
    let addr: SocketAddr = args.addr.parse().expect("bind address must parse");
    assert!(
        addr.ip().is_loopback(),
        "REFUSED: dashboard API binds loopback only (got {})",
        addr.ip()
    );

    let conn = open_db(std::path::Path::new(&args.db)).expect("projection db opens");
    let empty = conn
        .query_row("SELECT COUNT(*) FROM runs", [], |r| r.get::<_, i64>(0))
        .map(|n| n == 0)
        .unwrap_or(true);
    if args.rebuild || empty {
        let roots = Roots::from_dirs(args.roots.split(':').filter(|s| !s.is_empty()));
        let summary = import_corpus(&conn, &roots, &utc_now_rfc3339()).expect("import");
        eprintln!(
            "projection: {} runs imported, {} quarantined, {} coverage windows, {} registry records",
            summary.runs_imported, summary.runs_quarantined, summary.coverage_windows, summary.registry_records
        );
    }

    let state = Arc::new(Mutex::new(conn));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime.block_on(async move {
        let app = http_api::router(state);
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("loopback bind");
        eprintln!("sniper-dashboard-api listening on http://{addr}/ (GET-only)");
        axum::serve(listener, app).await.expect("server run");
    });
}
