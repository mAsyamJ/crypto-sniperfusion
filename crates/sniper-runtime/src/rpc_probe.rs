//! Read-only cluster RPC probe (INFRA-2): integrates operator-provided RPC
//! endpoints into the system without ever persisting them.
//!
//! Endpoint resolution: `SNF_MAINNET_RPC_URL` / `SNF_DEVNET_RPC_URL` /
//! `SNF_TESTNET_RPC_URL` (operator-private URLs, supplied per shell) take
//! precedence; otherwise the pinned public cluster endpoint is used. Cluster
//! identity is then verified against pinned genesis hashes so a mis-pointed
//! endpoint (e.g. devnet URL answering with mainnet's genesis) fails loudly.
//!
//! This module is read-only by construction: it performs only the four
//! getter calls (`getGenesisHash`, `getVersion`, `getSlot`, `getEpochInfo`),
//! imports no signer or keypair type, and has no send path. Printable URLs
//! are reduced to scheme + host (`redact_url`) because provider credentials
//! commonly live in the URL path (e.g. Infura `/v3/<key>`).

use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use solana_client::client_error::ClientError;
use solana_client::rpc_client::RpcClient;

/// Env var carrying the operator's private read-only mainnet-beta RPC URL.
pub const ENV_MAINNET_RPC_URL: &str = "SNF_MAINNET_RPC_URL";
/// Env var carrying the operator's private devnet RPC URL.
pub const ENV_DEVNET_RPC_URL: &str = "SNF_DEVNET_RPC_URL";
/// Env var carrying an optional private testnet RPC URL.
pub const ENV_TESTNET_RPC_URL: &str = "SNF_TESTNET_RPC_URL";

/// Solana public clusters. Genesis hashes are stable protocol facts, pinned
/// here so identity verification never depends on a live cross-check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Cluster {
    /// Mainnet-beta. Strictly read-only in every code path here.
    #[serde(rename = "mainnet")]
    MainnetBeta,
    #[serde(rename = "devnet")]
    Devnet,
    #[serde(rename = "testnet")]
    Testnet,
}

impl Cluster {
    /// Pinned genesis hash used to verify endpoint identity.
    pub fn expected_genesis(self) -> &'static str {
        match self {
            Cluster::MainnetBeta => "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d",
            Cluster::Devnet => "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG",
            Cluster::Testnet => "4uhcVJyU9pJkvQyS88uRDiswHXSCkY3zQawwpjk2NsNY",
        }
    }

    /// Public cluster endpoint used when no operator env URL is set.
    pub fn public_rpc_url(self) -> &'static str {
        match self {
            Cluster::MainnetBeta => "https://api.mainnet-beta.solana.com",
            Cluster::Devnet => "https://api.devnet.solana.com",
            Cluster::Testnet => "https://api.testnet.solana.com",
        }
    }

    /// Env var consulted for an operator-private endpoint override.
    pub fn env_var(self) -> &'static str {
        match self {
            Cluster::MainnetBeta => ENV_MAINNET_RPC_URL,
            Cluster::Devnet => ENV_DEVNET_RPC_URL,
            Cluster::Testnet => ENV_TESTNET_RPC_URL,
        }
    }
}

impl FromStr for Cluster {
    type Err = ProbeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "mainnet" => Ok(Cluster::MainnetBeta),
            "devnet" => Ok(Cluster::Devnet),
            "testnet" => Ok(Cluster::Testnet),
            other => Err(ProbeError::UnknownCluster(other.to_string())),
        }
    }
}

impl fmt::Display for Cluster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Cluster::MainnetBeta => "mainnet",
            Cluster::Devnet => "devnet",
            Cluster::Testnet => "testnet",
        })
    }
}

/// Where a resolved endpoint URL came from. `OperatorEnv` records only the
/// env var NAME — the URL itself is never serialized.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EndpointSource {
    /// Operator-private URL supplied via the named environment variable.
    OperatorEnv { var: &'static str },
    /// Pinned public cluster endpoint (no operator URL was set).
    PublicClusterDefault,
}

/// A cluster endpoint chosen by [`resolve_endpoint`].
#[derive(Debug, Clone)]
pub struct ResolvedEndpoint {
    pub cluster: Cluster,
    /// Raw URL — internal only; printable forms must go through
    /// [`redact_url`].
    pub url: String,
    pub source: EndpointSource,
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("invalid RPC endpoint URL for {cluster}: {reason}")]
    InvalidUrl {
        cluster: Cluster,
        reason: &'static str,
    },
    #[error(
        "cluster identity mismatch for {cluster}: endpoint genesis {actual} != expected {expected}"
    )]
    GenesisMismatch {
        cluster: Cluster,
        expected: String,
        actual: String,
    },
    #[error("read-only RPC call failed: {0}")]
    Rpc(String),
    #[error("unknown cluster {0:?} (expected mainnet|devnet|testnet)")]
    UnknownCluster(String),
}

/// Validates an operator-supplied endpoint URL: http(s) only, a host must be
/// present, and embedded userinfo credentials are rejected (secrets belong
/// in env storage, not URL syntax).
pub fn validate_url(cluster: Cluster, url: &str) -> Result<(), ProbeError> {
    let invalid = |reason: &'static str| Err(ProbeError::InvalidUrl { cluster, reason });
    let (scheme, rest) = match url.split_once("://") {
        Some(parts) => parts,
        None => return invalid("missing scheme (want http:// or https://)"),
    };
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return invalid("scheme must be http or https");
    }
    if url.chars().any(char::is_whitespace) {
        return invalid("URL contains whitespace");
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.is_empty() {
        return invalid("missing host");
    }
    if authority.contains('@') {
        return invalid("embedded userinfo credentials are not allowed");
    }
    Ok(())
}

/// Chooses the endpoint for `cluster`: the operator env override when set
/// (and valid), else the pinned public cluster URL. A set-but-empty or
/// whitespace-only env value is treated as unset. An INVALID env value is a
/// hard error — never a silent fallback to the public endpoint.
pub fn resolve_endpoint(
    cluster: Cluster,
    env_lookup: impl Fn(&str) -> Option<String>,
) -> Result<ResolvedEndpoint, ProbeError> {
    let var = cluster.env_var();
    match env_lookup(var).map(|v| v.trim().to_string()) {
        Some(url) if !url.is_empty() => {
            validate_url(cluster, &url)?;
            Ok(ResolvedEndpoint {
                cluster,
                url,
                source: EndpointSource::OperatorEnv { var },
            })
        }
        _ => Ok(ResolvedEndpoint {
            cluster,
            url: cluster.public_rpc_url().to_string(),
            source: EndpointSource::PublicClusterDefault,
        }),
    }
}

/// Verifies a reported genesis hash against the pinned constant for the
/// cluster. Mismatch means the endpoint is not the cluster it claims to be.
pub fn verify_genesis(cluster: Cluster, genesis: &str) -> Result<(), ProbeError> {
    let expected = cluster.expected_genesis();
    if genesis == expected {
        Ok(())
    } else {
        Err(ProbeError::GenesisMismatch {
            cluster,
            expected: expected.to_string(),
            actual: genesis.to_string(),
        })
    }
}

/// Reduces a URL to scheme + host. Provider credentials embedded in the path
/// or query (Infura-style `/v3/<key>`) never survive this.
pub fn redact_url(url: &str) -> String {
    match url.find("://") {
        Some(i) => {
            let rest = &url[i + 3..];
            let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
            format!("{}{}", &url[..i + 3], &rest[..host_end])
        }
        None => "<unparsed-url>".to_string(),
    }
}

/// Result of a successful read-only cluster probe (JSON-shaped for the CLI).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ClusterProbeReport {
    pub cluster: Cluster,
    /// Redacted endpoint (scheme + host only).
    pub url_redacted: String,
    pub endpoint_source: EndpointSource,
    pub genesis_hash: String,
    pub genesis_verified: bool,
    pub solana_core_version: String,
    pub feature_set: u32,
    pub slot: u64,
    pub epoch: u64,
    pub block_height: u64,
}

/// Runs the four read-only getter calls against `url` and verifies cluster
/// identity. Any transport error is sanitized so the raw URL (which may
/// carry a provider key in its path) cannot leak into error strings.
pub fn probe_cluster(
    url: &str,
    cluster: Cluster,
    timeout: Duration,
) -> Result<ClusterProbeReport, ProbeError> {
    let client = RpcClient::new_with_timeout(url.to_string(), timeout);
    let sanitize = |e: ClientError| ProbeError::Rpc(e.to_string().replace(url, &redact_url(url)));

    let genesis = client.get_genesis_hash().map_err(sanitize)?.to_string();
    verify_genesis(cluster, &genesis)?;

    let version = client.get_version().map_err(sanitize)?;
    let slot = client.get_slot().map_err(sanitize)?;
    let epoch_info = client.get_epoch_info().map_err(sanitize)?;

    Ok(ClusterProbeReport {
        cluster,
        url_redacted: redact_url(url),
        endpoint_source: EndpointSource::PublicClusterDefault,
        genesis_hash: genesis,
        genesis_verified: true,
        solana_core_version: version.solana_core,
        feature_set: version.feature_set.unwrap_or(0),
        slot,
        epoch: epoch_info.epoch,
        block_height: epoch_info.block_height,
    })
}

/// Probes a resolved endpoint and stamps the report with the endpoint's true
/// provenance (operator env vs pinned public fallback).
pub fn probe_endpoint(
    endpoint: &ResolvedEndpoint,
    timeout: Duration,
) -> Result<ClusterProbeReport, ProbeError> {
    let mut report = probe_cluster(&endpoint.url, endpoint.cluster, timeout)?;
    report.url_redacted = redact_url(&endpoint.url);
    report.endpoint_source = endpoint.source.clone();
    Ok(report)
}
