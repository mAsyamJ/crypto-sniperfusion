//! Yellowstone gRPC adapter (read-only ingestion).
//!
//! Mapping is a pure function over `SubscribeUpdate` (unit-tested with the
//! client's `test-tools` mock stream — no network needed). The live
//! connection path is exercised only by opt-in tests: without
//! `SNF_NETWORK_TESTS=1` + `SNF_GEYSER_ENDPOINT` they print a skip reason and
//! pass — a pass that never talked to a provider would be fabricated.

use futures::StreamExt;
use solana_sdk::pubkey::Pubkey;
use yellowstone_grpc_client::{GeyserGrpcClient, GeyserStream};
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::prelude::*;

use sniper_core::event::{Commitment, ProviderKind};
use sniper_core::time::MonotonicClock;

use crate::ingest::frame::{RawObservation, RawPayload};
use crate::ingest::provider::{EventProvider, ProviderError};

/// Build the subscription request for one program's transaction stream.
/// Pure function — testable.
pub fn build_request(program: &Pubkey) -> SubscribeRequest {
    let mut transactions = std::collections::HashMap::new();
    transactions.insert(
        "sniper-pump-txs".to_string(),
        SubscribeRequestFilterTransactions {
            account_include: vec![program.to_string()],
            ..Default::default()
        },
    );
    SubscribeRequest {
        transactions,
        commitment: Some(CommitmentLevel::Processed as i32),
        ..Default::default()
    }
}

/// Map one stream update to raw observations. Pure function — testable.
/// Oracle parity: failed transactions (`meta.err` set) are marked failed and
/// dropped at decode time; votes are skipped.
pub fn map_update(
    update: SubscribeUpdate,
    program: &Pubkey,
    provider: ProviderKind,
    clock: &MonotonicClock,
) -> Vec<RawObservation> {
    let tx = match update.update_oneof {
        Some(UpdateOneof::Transaction(tx)) => tx,
        _ => return Vec::new(),
    };
    let info = match tx.transaction {
        Some(info) => info,
        None => return Vec::new(),
    };
    if info.is_vote {
        return Vec::new();
    }
    let Some(meta) = info.meta else {
        return Vec::new();
    };
    let Ok(signature) = solana_sdk::signature::Signature::try_from(info.signature.as_slice())
    else {
        return Vec::new();
    };
    vec![RawObservation {
        provider,
        signature: Some(signature),
        slot: Some(tx.slot),
        // We subscribe at Processed; upgrades flow through the ledger.
        commitment: Commitment::Processed,
        received: clock.now(),
        failed: meta.err.is_some(),
        payload: RawPayload::Logs {
            program: *program,
            logs: meta.log_messages,
        },
    }]
}

/// Yellowstone gRPC ingestion over one program's transaction stream. The
/// supervisor owns reconnection; this adapter is a dumb stream reader.
pub struct GeyserProvider {
    endpoint: String,
    token: Option<String>,
    x_token: bool,
    program: Pubkey,
    stream: Option<GeyserStream>,
    clock: MonotonicClock,
}

impl GeyserProvider {
    pub fn new(endpoint: String, token: Option<String>, x_token: bool, program: Pubkey) -> Self {
        Self {
            endpoint,
            token,
            x_token,
            program,
            stream: None,
            clock: MonotonicClock::new(),
        }
    }

    /// Injection point for tests/fixture replay: drive a prepared stream
    /// (e.g. `GeyserStream::mock(..)` under the client's `test-tools`
    /// feature) through the same mapping as the live path.
    pub fn from_stream(stream: GeyserStream, program: Pubkey) -> Self {
        Self {
            endpoint: String::new(),
            token: None,
            x_token: true,
            program,
            stream: Some(stream),
            clock: MonotonicClock::new(),
        }
    }
}

impl EventProvider for GeyserProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Geyser
    }

    async fn connect(&mut self) -> Result<(), ProviderError> {
        let mut builder = GeyserGrpcClient::build_from_shared(self.endpoint.clone())
            .map_err(|e| ProviderError::NotConfigured(e.to_string()))?;
        if self.x_token {
            if let Some(token) = self.token.clone() {
                builder = builder
                    .x_token(Some(token))
                    .map_err(|e| ProviderError::NotConfigured(e.to_string()))?;
            }
        }
        let mut client = builder
            .connect()
            .await
            .map_err(|e| ProviderError::Transport(e.to_string()))?;
        let stream = client
            .subscribe_once(build_request(&self.program))
            .await
            .map_err(|e| ProviderError::Rejected(e.to_string()))?;
        self.stream = Some(stream);
        Ok(())
    }

    async fn next_observation(&mut self) -> Result<RawObservation, ProviderError> {
        loop {
            let stream = self
                .stream
                .as_mut()
                .ok_or_else(|| ProviderError::NotConfigured("not connected".into()))?;
            let update = match stream.next().await {
                Some(Ok(update)) => update,
                Some(Err(status)) => {
                    self.stream = None; // force reconnect on next use
                    return Err(ProviderError::Transport(status.to_string()));
                }
                None => {
                    self.stream = None;
                    return Err(ProviderError::Transport("stream closed".into()));
                }
            };
            let observations = map_update(update, &self.program, ProviderKind::Geyser, &self.clock);
            // Skip keepalive/heartbeat updates; surface the first real one.
            if let Some(obs) = observations.into_iter().next() {
                return Ok(obs);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sniper_core::venue::Venue;
    use sniper_protocols::registry::program_id;
    use yellowstone_grpc_proto::geyser::{
        SubscribeUpdate, SubscribeUpdateTransaction, SubscribeUpdateTransactionInfo,
    };
    use yellowstone_grpc_proto::solana::storage::confirmed_block::TransactionStatusMeta;

    fn tx_update(
        sig_byte: u8,
        slot: u64,
        err: Option<TransactionError>,
        logs: Vec<String>,
    ) -> SubscribeUpdate {
        let mut signature = vec![0u8; 64];
        signature[0] = sig_byte;
        SubscribeUpdate {
            update_oneof: Some(UpdateOneof::Transaction(SubscribeUpdateTransaction {
                transaction: Some(SubscribeUpdateTransactionInfo {
                    signature,
                    is_vote: false,
                    meta: Some(TransactionStatusMeta {
                        err,
                        log_messages: logs,
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                slot,
                ..Default::default()
            })),
            ..Default::default()
        }
    }

    #[test]
    fn request_targets_program_at_processed() {
        let pump = program_id(Venue::PumpFun).unwrap();
        let req = build_request(&pump);
        assert_eq!(req.commitment, Some(CommitmentLevel::Processed as i32));
        let filter = req
            .transactions
            .get("sniper-pump-txs")
            .expect("filter present");
        assert_eq!(filter.account_include, vec![pump.to_string()]);
    }

    #[test]
    fn map_update_success_and_failure_paths() {
        let pump = program_id(Venue::PumpFun).unwrap();
        let clock = MonotonicClock::new();

        let ok = map_update(
            tx_update(1, 500, None, vec!["Program log: Instruction: Buy".into()]),
            &pump,
            ProviderKind::Geyser,
            &clock,
        );
        assert_eq!(ok.len(), 1);
        assert!(!ok[0].failed);
        assert_eq!(ok[0].slot, Some(500));
        assert_eq!(ok[0].commitment, Commitment::Processed);

        let failed = map_update(
            tx_update(2, 501, Some(TransactionError::default()), vec![]),
            &pump,
            ProviderKind::Geyser,
            &clock,
        );
        assert_eq!(failed.len(), 1);
        assert!(
            failed[0].failed,
            "meta.err set → failed (dropped at decode)"
        );

        let vote = SubscribeUpdate {
            update_oneof: Some(UpdateOneof::Transaction(SubscribeUpdateTransaction {
                transaction: Some(SubscribeUpdateTransactionInfo {
                    signature: vec![0u8; 64],
                    is_vote: true,
                    ..Default::default()
                }),
                ..Default::default()
            })),
            ..Default::default()
        };
        assert!(map_update(vote, &pump, ProviderKind::Geyser, &clock).is_empty());
    }

    #[tokio::test]
    async fn provider_reads_mock_stream_end_to_end() {
        use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof::Ping;
        use yellowstone_grpc_proto::geyser::SubscribeUpdatePing;

        let (tx, rx) = tokio::sync::mpsc::channel(8);
        let stream = GeyserStream::mock(rx);
        let pump = program_id(Venue::PumpFun).unwrap();
        let mut provider = GeyserProvider::from_stream(stream, pump);

        // A keepalive ping first (must be skipped), then a real transaction.
        tx.send(Ok(SubscribeUpdate {
            update_oneof: Some(Ping(SubscribeUpdatePing::default())),
            ..Default::default()
        }))
        .await
        .unwrap();
        tx.send(Ok(tx_update(9, 900, None, vec!["Program log: x".into()])))
            .await
            .unwrap();
        drop(tx);

        let obs = provider.next_observation().await.expect("real observation");
        assert_eq!(obs.slot, Some(900));
        assert!(matches!(obs.payload, RawPayload::Logs { .. }));

        // Stream end must surface as a loud transport error.
        let err = provider
            .next_observation()
            .await
            .expect_err("stream closed");
        assert!(matches!(err, ProviderError::Transport(_)));
    }

    /// OPT-IN live test. Without `SNF_NETWORK_TESTS=1` AND
    /// `SNF_GEYSER_ENDPOINT` (+ optional `SNF_GEYSER_TOKEN`) it prints the
    /// skip reason and passes — an unexecuted provider conversation is never
    /// reported as a pass.
    #[tokio::test]
    async fn live_geyser_subscription_opt_in() {
        let Ok(run) = std::env::var("SNF_NETWORK_TESTS") else {
            eprintln!("[SKIP] live geyser test: SNF_NETWORK_TESTS is not set");
            return;
        };
        if run != "1" {
            eprintln!("[SKIP] live geyser test: SNF_NETWORK_TESTS != 1");
            return;
        }
        let Ok(endpoint) = std::env::var("SNF_GEYSER_ENDPOINT") else {
            eprintln!("[SKIP] live geyser test: SNF_GEYSER_ENDPOINT is not set");
            return;
        };
        let token = std::env::var("SNF_GEYSER_TOKEN").ok();
        let pump = program_id(Venue::PumpFun).unwrap();
        let mut provider = GeyserProvider::new(endpoint, token, true, pump);
        provider
            .connect()
            .await
            .expect("live connect with provided credentials");
        let obs = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            provider.next_observation(),
        )
        .await
        .expect("one observation within 30s")
        .expect("observation ok");
        eprintln!(
            "[OK] live observation: slot={:?} failed={}",
            obs.slot, obs.failed
        );
    }
}
