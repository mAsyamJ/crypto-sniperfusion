//! WSS pubsub adapter (logsSubscribe) — the chainstack "logs" listener path.
//!
//! Mapping is a pure function (unit-tested). The live connection path is
//! opt-in only (`SNF_NETWORK_TESTS=1` + `SNF_WSS_ENDPOINT`), skipping with a
//! printed reason otherwise.
//!
//! Structure: `logs_subscribe` returns a stream borrowing the client, so the
//! client+stream live inside a dedicated task that forwards notifications
//! over a channel — the provider owns only the receiver (no self-referential
//! borrow; the supervisor stays the single owner of reconnection).

use solana_pubsub_client::nonblocking::pubsub_client::PubsubClient;
use solana_rpc_client_types::config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_rpc_client_types::response::{Response as RpcResponse, RpcLogsResponse};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;

use sniper_core::event::{Commitment, ProviderKind};
use sniper_core::time::MonotonicClock;

use crate::ingest::frame::{RawObservation, RawPayload};
use crate::ingest::provider::{EventProvider, ProviderError};

type Inbox = tokio::sync::mpsc::Receiver<Result<RpcResponse<RpcLogsResponse>, String>>;

/// Map one logs notification to a raw observation. Pure function.
/// Oracle parity (`universal_logs_listener.py`): `processed` commitment,
/// `value.err` gate (failed txs marked here, dropped at decode).
pub fn map_logs_response(
    value: RpcLogsResponse,
    program: &Pubkey,
    clock: &MonotonicClock,
) -> Option<RawObservation> {
    let signature: solana_sdk::signature::Signature = value.signature.parse().ok()?;
    Some(RawObservation {
        provider: ProviderKind::Logs,
        signature: Some(signature),
        // logsSubscribe carries no slot; the pipeline treats slot as unknown.
        slot: None,
        commitment: Commitment::Processed,
        received: clock.now(),
        failed: value.err.is_some(),
        payload: RawPayload::Logs {
            program: *program,
            logs: value.logs,
        },
    })
}

/// WSS logs-listener over one program.
pub struct LogsProvider {
    wss_url: String,
    program: Pubkey,
    inbox: Option<Inbox>,
    clock: MonotonicClock,
}

impl LogsProvider {
    pub fn new(wss_url: String, program: Pubkey) -> Self {
        Self {
            wss_url,
            program,
            inbox: None,
            clock: MonotonicClock::new(),
        }
    }
}

impl EventProvider for LogsProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Logs
    }

    async fn connect(&mut self) -> Result<(), ProviderError> {
        let (tx, rx) = tokio::sync::mpsc::channel(256);
        let url = self.wss_url.clone();
        let program = self.program;
        tokio::spawn(async move {
            let client = match PubsubClient::new(&url).await {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(Err(format!("wss connect: {e}"))).await;
                    return;
                }
            };
            let (stream, _unsubscribe) = match client
                .logs_subscribe(
                    RpcTransactionLogsFilter::Mentions(vec![program.to_string()]),
                    RpcTransactionLogsConfig {
                        commitment: Some(CommitmentConfig::processed()),
                    },
                )
                .await
            {
                Ok(pair) => pair,
                Err(e) => {
                    let _ = tx.send(Err(format!("logs_subscribe: {e}"))).await;
                    return;
                }
            };
            let mut stream = stream;
            while let Some(item) = futures::StreamExt::next(&mut stream).await {
                if tx.send(Ok(item)).await.is_err() {
                    break; // receiver dropped: shutdown
                }
            }
            // Channel closes -> provider sees stream end on next recv.
        });
        self.inbox = Some(rx);
        Ok(())
    }

    async fn next_observation(&mut self) -> Result<RawObservation, ProviderError> {
        let inbox = self
            .inbox
            .as_mut()
            .ok_or_else(|| ProviderError::NotConfigured("not connected".into()))?;
        match inbox.recv().await {
            Some(Ok(response)) => map_logs_response(response.value, &self.program, &self.clock)
                .ok_or_else(|| ProviderError::Transport("unparseable logs notification".into())),
            Some(Err(message)) => {
                self.inbox = None; // subscription failed: force reconnect
                Err(ProviderError::Rejected(message))
            }
            None => {
                self.inbox = None; // stream ended
                Err(ProviderError::Transport("logs stream closed".into()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sniper_core::venue::Venue;
    use sniper_protocols::registry::program_id;
    use solana_sdk::signature::{Keypair, Signer};

    fn response(
        sig: &str,
        err: Option<solana_sdk::transaction::TransactionError>,
        logs: Vec<&str>,
    ) -> RpcLogsResponse {
        RpcLogsResponse {
            signature: sig.to_string(),
            err,
            logs: logs.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn maps_successful_logs_notification() {
        let pump = program_id(Venue::PumpFun).unwrap();
        let clock = MonotonicClock::new();
        let sig = Keypair::new().sign_message(b"x").to_string();
        let obs = map_logs_response(
            response(&sig, None, vec!["Program log: Instruction: Buy"]),
            &pump,
            &clock,
        )
        .expect("maps");
        assert!(!obs.failed);
        assert_eq!(obs.slot, None);
        assert_eq!(obs.commitment, Commitment::Processed);
        assert!(matches!(obs.payload, RawPayload::Logs { .. }));
    }

    #[test]
    fn failed_tx_marked_not_dropped_here() {
        let pump = program_id(Venue::PumpFun).unwrap();
        let clock = MonotonicClock::new();
        let sig = Keypair::new().sign_message(b"y").to_string();
        let obs = map_logs_response(
            response(
                &sig,
                Some(solana_sdk::transaction::TransactionError::AccountInUse),
                vec![],
            ),
            &pump,
            &clock,
        )
        .expect("maps");
        assert!(obs.failed);
    }

    #[test]
    fn invalid_signature_is_rejected() {
        let pump = program_id(Venue::PumpFun).unwrap();
        let clock = MonotonicClock::new();
        assert!(map_logs_response(response("not-a-sig", None, vec![]), &pump, &clock).is_none());
    }

    #[tokio::test]
    async fn inbox_channel_drives_provider() {
        let pump = program_id(Venue::PumpFun).unwrap();
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        let mut provider = LogsProvider {
            wss_url: String::new(),
            program: pump,
            inbox: Some(rx),
            clock: MonotonicClock::new(),
        };
        let sig = Keypair::new().sign_message(b"z").to_string();
        let ctx = solana_rpc_client_types::response::RpcResponseContext {
            slot: 0,
            api_version: None,
        };
        tx.send(Ok(RpcResponse {
            context: ctx,
            value: response(&sig, None, vec!["Program log: a"]),
        }))
        .await
        .unwrap();
        tx.send(Err("boom".into())).await.unwrap();
        drop(tx);

        let obs = provider.next_observation().await.expect("maps inbox item");
        assert!(!obs.failed);
        let err = provider
            .next_observation()
            .await
            .expect_err("failure surfaces");
        assert!(matches!(err, ProviderError::Rejected(_)));
        // After a rejected subscription the inbox is cleared: subsequent calls
        // report NotConfigured until the supervisor reconnects.
        let end = provider
            .next_observation()
            .await
            .expect_err("cleared inbox surfaces");
        assert!(matches!(end, ProviderError::NotConfigured { .. }));
    }
}
