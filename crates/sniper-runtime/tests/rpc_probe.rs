//! rpc_probe contract tests (INFRA-2): operator-private RPC endpoints are
//! integrated via environment variables with a pinned public fallback per
//! cluster, cluster identity is verified against pinned genesis hashes, and
//! every printable URL is redacted so provider path credentials (e.g. Infura
//! keys embedded in the URL path) never reach output.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use sniper_runtime::rpc_probe::{
    redact_url, resolve_endpoint, verify_genesis, Cluster, EndpointSource, ProbeError,
};

fn env_with(v: Option<&str>) -> impl Fn(&str) -> Option<String> + '_ {
    move |_var: &str| v.map(str::to_string)
}

#[test]
fn env_override_wins_and_is_validated() {
    let ep = resolve_endpoint(
        Cluster::Devnet,
        env_with(Some("https://example-rpc.dev/v3/SECRET")),
    )
    .expect("https env url resolves");
    assert_eq!(ep.url, "https://example-rpc.dev/v3/SECRET");
    assert!(matches!(ep.source, EndpointSource::OperatorEnv { .. }));

    // non-http(s) schemes are rejected outright, never silently fallen back
    assert!(matches!(
        resolve_endpoint(Cluster::Devnet, env_with(Some("ftp://example.com"))),
        Err(ProbeError::InvalidUrl { .. })
    ));
    // embedded userinfo credentials are rejected (secrets must live in env, not URLs)
    assert!(matches!(
        resolve_endpoint(
            Cluster::Devnet,
            env_with(Some("https://user:pass@example.com"))
        ),
        Err(ProbeError::InvalidUrl { .. })
    ));
    // a set-but-empty env var is treated as unset (public fallback), not as ""
    let ep = resolve_endpoint(Cluster::Devnet, env_with(Some(""))).expect("empty env falls back");
    assert!(matches!(ep.source, EndpointSource::PublicClusterDefault));
}

#[test]
fn missing_env_falls_back_to_pinned_public_url() {
    let ep = resolve_endpoint(Cluster::MainnetBeta, env_with(None)).expect("fallback resolves");
    assert_eq!(ep.url, "https://api.mainnet-beta.solana.com");
    assert!(matches!(ep.source, EndpointSource::PublicClusterDefault));
    assert_eq!(
        Cluster::Devnet.public_rpc_url(),
        "https://api.devnet.solana.com"
    );
    assert_eq!(
        Cluster::Testnet.public_rpc_url(),
        "https://api.testnet.solana.com"
    );
}

#[test]
fn genesis_constants_are_verified() {
    verify_genesis(
        Cluster::MainnetBeta,
        "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d",
    )
    .expect("mainnet-beta genesis");
    verify_genesis(
        Cluster::Devnet,
        "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG",
    )
    .expect("devnet genesis");
    verify_genesis(
        Cluster::Testnet,
        "4uhcVJyU9pJkvQyS88uRDiswHXSCkY3zQawwpjk2NsNY",
    )
    .expect("testnet genesis");
    // devnet endpoint answering with the mainnet genesis is an identity error
    assert!(matches!(
        verify_genesis(
            Cluster::Devnet,
            "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d"
        ),
        Err(ProbeError::GenesisMismatch { .. })
    ));
}

#[test]
fn redact_url_strips_path_and_query() {
    // Infura-style keys live in the URL path — the printable form keeps only
    // scheme + host.
    assert_eq!(
        redact_url("https://solana-devnet.infura.io/v3/00000000000000000000000000000000"),
        "https://solana-devnet.infura.io"
    );
    assert_eq!(
        redact_url("https://api.devnet.solana.com"),
        "https://api.devnet.solana.com"
    );
    assert_eq!(redact_url("https://h.example/x?a=1"), "https://h.example");
}

#[test]
fn cluster_from_str_round_trips() {
    assert_eq!("devnet".parse::<Cluster>().unwrap(), Cluster::Devnet);
    assert_eq!("mainnet".parse::<Cluster>().unwrap(), Cluster::MainnetBeta);
    assert_eq!("testnet".parse::<Cluster>().unwrap(), Cluster::Testnet);
    assert!("mainnet-beta".parse::<Cluster>().is_err());
}

#[test]
fn probe_cluster_verifies_genesis_against_mock_rpc() {
    // happy path: mock serves the devnet genesis -> verified report
    let (url, handle) = spawn_mock_rpc(Some(Cluster::Devnet.expected_genesis()), 4);
    let report = sniper_runtime::rpc_probe::probe_cluster(
        &url,
        Cluster::Devnet,
        Duration::from_secs(5),
        None,
    )
    .expect("probe succeeds against mock");
    assert!(report.genesis_verified);
    assert_eq!(report.genesis_hash, Cluster::Devnet.expected_genesis());
    assert_eq!(report.solana_core_version, "4.4.0");
    assert_eq!(report.feature_set, 3_722_100_051);
    assert_eq!(report.slot, 518_298_547);
    assert_eq!(report.epoch, 1199);
    assert_eq!(report.block_height, 505_461_001);
    // the printable URL never carries the path/credential part
    assert_eq!(report.url_redacted, redact_url(&url));
    handle.join().expect("mock server thread");

    // hostile path: endpoint answers with mainnet's genesis -> hard failure
    // (the probe stops after the first call, so the mock expects exactly 1)
    let (url, handle) = spawn_mock_rpc(Some(Cluster::MainnetBeta.expected_genesis()), 1);
    let err = sniper_runtime::rpc_probe::probe_cluster(
        &url,
        Cluster::Devnet,
        Duration::from_secs(5),
        None,
    )
    .expect_err("genesis mismatch must fail");
    assert!(matches!(err, ProbeError::GenesisMismatch { .. }));
    handle.join().expect("mock server thread");

    // watch-address path: a 5th read-only call (getBalance) lands in the report
    let (url, handle) = spawn_mock_rpc(Some(Cluster::Devnet.expected_genesis()), 5);
    let report = sniper_runtime::rpc_probe::probe_cluster(
        &url,
        Cluster::Devnet,
        Duration::from_secs(5),
        Some("32qjUtSaDQy7t8AseCg4QGzu7wLFVV3uGBFcpXcsrYTM"),
    )
    .expect("probe with watch address succeeds");
    assert_eq!(
        report.watch_address.as_deref(),
        Some("32qjUtSaDQy7t8AseCg4QGzu7wLFVV3uGBFcpXcsrYTM")
    );
    assert_eq!(report.watch_balance_lamports, Some(998_995_000));
    handle.join().expect("mock server thread");
}

#[test]
fn invalid_watch_address_fails_before_any_network_call() {
    // unreachable endpoint: if address parsing did not fail first, the test
    // would still fail fast (connection refused) with the WRONG error kind
    let err = sniper_runtime::rpc_probe::probe_cluster(
        "http://127.0.0.1:1",
        Cluster::MainnetBeta,
        Duration::from_secs(2),
        Some("not-a-base58-pubkey!!"),
    )
    .expect_err("invalid watch address must be rejected");
    assert!(matches!(err, ProbeError::InvalidAddress(_)));
}

/// Spawns a single-threaded mock JSON-RPC server answering the read-only
/// probe calls in fixed order (genesis, version, slot, epoch-info).
/// `genesis` overrides the getGenesisHash result (None = devnet default);
/// `expected_calls` bounds how many connections the server will serve (the
/// genesis-mismatch path stops after the first call by design).
fn spawn_mock_rpc(
    genesis: Option<&'static str>,
    expected_calls: usize,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock rpc");
    let addr = listener.local_addr().expect("mock addr");
    let handle = thread::spawn(move || {
        for method in [
            "getGenesisHash",
            "getVersion",
            "getSlot",
            "getEpochInfo",
            "getBalance",
        ]
        .into_iter()
        .take(expected_calls)
        {
            let (mut sock, _) = listener.accept().expect("mock accept");
            let mut buf = [0u8; 8192];
            let n = sock.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).into_owned();
            assert!(
                req.contains(method),
                "unexpected/unordered RPC call, wanted {method}, got: {req}"
            );
            let body = match method {
                "getGenesisHash" => format!(
                    r#"{{"jsonrpc":"2.0","result":"{}","id":1}}"#,
                    genesis.unwrap_or("EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG")
                ),
                "getVersion" => {
                    r#"{"jsonrpc":"2.0","result":{"solana-core":"4.4.0","feature-set":3722100051},"id":1}"#.to_string()
                }
                "getSlot" => r#"{"jsonrpc":"2.0","result":518298547,"id":1}"#.to_string(),
                "getBalance" => {
                    r#"{"jsonrpc":"2.0","result":{"context":{"slot":518298560},"value":998995000},"id":1}"#.to_string()
                }
                _ => r#"{"jsonrpc":"2.0","result":{"epoch":1199,"absoluteSlot":518298551,"blockHeight":505461001,"slotsInEpoch":432000,"slotIndex":1,"transactionCount":null},"id":1}"#.to_string(),
            };
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            sock.write_all(resp.as_bytes()).unwrap();
        }
    });
    (format!("http://{addr}"), handle)
}
