//! DASH-02 Task 6 — boundary/security tests on a REAL loopback socket
//! (T-SEC1, T-SEC2, T-SEC3, T-SEC4) plus the dependency deny-list audit
//! (T-SEC5). Raw TcpStream HTTP so no HTTP-client dependency is added.

use std::sync::{Arc, Mutex};

use sniper_dashboard_api::http_api::router;
use sniper_observability::discovery::Roots;
use sniper_observability::project::{import_corpus, open_db};

async fn spawn_server() -> String {
    let tmp = tempfile::tempdir().unwrap();
    let corpus = tmp.path().join("corpus");
    std::fs::create_dir_all(corpus.join("runA")).unwrap();
    let oracle = sniper_strategy::oracle::AccountingOracle::new(0);
    let receipt = sniper_strategy::receipt::build_receipt(
        "engine_t",
        "SYNTHETIC",
        "SYNTHETIC_COMPLETE_FIXTURE",
        "x".repeat(64).as_str(),
        0,
        &oracle,
        &Default::default(),
        0,
        0,
        0,
        0,
        vec![sniper_strategy::receipt::ComparisonRow {
            strategy: "S00".into(),
            net_pnl_lamports: 0,
            incremental_net_edge_vs_no_trade: 0,
            max_drawdown_lamports: 0,
            open_positions: 0,
            status: "NO_TRADE_BASELINE".into(),
        }],
        vec![],
    );
    sniper_strategy::receipt::write_bundle(&corpus.join("runA"), &receipt, "", "", "{}").unwrap();
    std::mem::forget(tmp);

    let conn = open_db(&corpus.join("c.sqlite3")).unwrap();
    import_corpus(&conn, &Roots::from_dirs([&corpus]), "t").unwrap();
    let db: Arc<Mutex<rusqlite::Connection>> = Arc::new(Mutex::new(conn));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router(db)).await.unwrap();
    });
    format!("http://{addr}")
}

/// Minimal raw HTTP/1.1 client (no external deps).
async fn raw_request(base: &str, raw: &str) -> (u16, String) {
    let addr: std::net::SocketAddr = base.trim_start_matches("http://").parse().unwrap();
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    stream.write_all(raw.as_bytes()).await.unwrap();
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf).await;
    let text = String::from_utf8_lossy(&buf).to_string();
    let status: u16 = text
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    (status, text)
}

fn get_req(path: &str, headers: &[(&str, &str)]) -> String {
    let mut req = format!("GET {path} HTTP/1.1\r\n");
    for (k, v) in headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str("connection: close\r\n\r\n");
    req
}

/// T-SEC1: anything but GET (and HEAD) is 405.
#[tokio::test]
async fn t_sec1_non_get_rejected() {
    let base = spawn_server().await;
    for method in ["POST", "DELETE", "PUT", "PATCH"] {
        let req = format!(
            "{method} /api/v1/runs HTTP/1.1\r\nhost: 127.0.0.1\r\nconnection: close\r\n\r\n"
        );
        let (status, _) = raw_request(&base, &req).await;
        assert_eq!(status, 405, "{method} not rejected");
    }
}

/// T-SEC2: foreign Host → 403; foreign Origin → 403; absent Origin + loopback Host → 200.
/// (A DNS-rebinding page arrives with the attacker's Host — blocked here.)
#[tokio::test]
async fn t_sec2_host_origin_enforced() {
    let base = spawn_server().await;
    let (status, _) = raw_request(
        &base,
        &get_req("/api/v1/health", &[("host", "evil.example")]),
    )
    .await;
    assert_eq!(status, 403, "foreign host accepted");
    let (status, _) = raw_request(
        &base,
        &get_req(
            "/api/v1/health",
            &[("host", "127.0.0.1"), ("origin", "https://evil.example")],
        ),
    )
    .await;
    assert_eq!(status, 403, "foreign origin accepted");
    let (status, _) =
        raw_request(&base, &get_req("/api/v1/health", &[("host", "localhost")])).await;
    assert_eq!(status, 200, "loopback host without origin refused");
    // Same-machine Origin passes.
    let (status, _) = raw_request(
        &base,
        &get_req(
            "/api/v1/health",
            &[("host", "127.0.0.1"), ("origin", "http://127.0.0.1:7433")],
        ),
    )
    .await;
    assert_eq!(status, 200);
}

/// T-SEC3: traversal-shaped ids never read files — unknown id → 404, and
/// the body contains no file contents.
#[tokio::test]
async fn t_sec3_path_traversal_rejected() {
    let base = spawn_server().await;
    for probe in [
        "/api/v1/runs/..%2F..%2Fetc%2Fpasswd",
        "/api/v1/runs/../../../etc/passwd",
        "/api/v1/runs/%2e%2e%2f%2e%2e%2fCargo.toml",
    ] {
        let (status, body) = raw_request(&base, &get_req(probe, &[("host", "127.0.0.1")])).await;
        assert!(status == 404 || status == 400, "{probe} → {status}");
        assert!(
            !body.contains("[package]"),
            "file content leaked for {probe}"
        );
    }
}

/// T-SEC4: /meta exposes no filesystem paths beyond allowlisted roots and
/// no environment values.
#[tokio::test]
async fn t_sec4_meta_hygiene() {
    let base = spawn_server().await;
    let (_, body) = raw_request(&base, &get_req("/api/v1/meta", &[("host", "127.0.0.1")])).await;
    assert!(!body.contains("/home/"), "absolute paths leaked: {body}");
    assert!(!body.contains("SNF_"), "env variable names leaked");
    assert!(
        body.contains("\"live_trading_enabled\":false"),
        "live flag not pinned false"
    );
}

/// T-SEC5: capability deny-list — neither new crate may depend on
/// sniper-runtime (signers/authz/journal live there), and the API never
/// links a market/RPC client.
#[test]
fn t_sec5_dependency_denylist() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for manifest in [
        root.join("crates/sniper-dashboard-api/Cargo.toml"),
        root.join("crates/sniper-observability/Cargo.toml"),
    ] {
        let text = std::fs::read_to_string(&manifest).unwrap();
        // Strip comment lines so prose mentions of the deny-list cannot
        // false-positive; only actual dependency entries are checked.
        let deps_section: String = text
            .split("[dev-dependencies]")
            .next()
            .unwrap()
            .lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");
        for forbidden in ["sniper-runtime", "solana-client", "yellowstone", "reqwest"] {
            assert!(
                !deps_section.contains(forbidden),
                "{} links forbidden dependency {forbidden}",
                manifest.display()
            );
        }
    }
}
