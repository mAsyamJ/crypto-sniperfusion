//! Raw provider observation → canonical `ChainEvent` mapping.
//!
//! Oracle-parity notes (chainstack listener behaviors):
//! - failed transactions are dropped at ingest (`value.err` / `failed=False`
//!   gates in every upstream listener),
//! - log-derived events are identified by (signature, event ordinal); the
//!   instruction indexes are not available from logs, so they are zeroed and
//!   the ordinal carries the `Program data:` line position.

use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;

use sniper_core::event::{ChainEvent, ChainEventId, Commitment, EventPayload, ProviderKind};
use sniper_core::primitives::{Lamports, TokenAmount};
use sniper_core::time::Timestamps;
use sniper_core::venue::Venue;
use sniper_protocols::events::PumpEvent;

/// pump.fun base token decimals (create_v2 mints are 6-decimals Token-2022;
/// COIN_CREATION.md).
pub const PUMP_BASE_DECIMALS: u8 = 6;

/// One raw observation from a provider, before decoding.
#[derive(Debug, Clone, PartialEq)]
pub struct RawObservation {
    pub provider: ProviderKind,
    /// Transaction signature when the provider gives one (logs/geyser do).
    pub signature: Option<Signature>,
    pub slot: Option<u64>,
    pub commitment: Commitment,
    pub received: Timestamps,
    /// True when the source marked the transaction failed — dropped here.
    pub failed: bool,
    pub payload: RawPayload,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RawPayload {
    /// Log lines for one transaction invocation of `program`.
    Logs { program: Pubkey, logs: Vec<String> },
}

impl RawObservation {
    /// Decode this observation into canonical events. Failed transactions
    /// yield nothing (oracle parity). Returns events in log order.
    pub fn into_events(self) -> Vec<ChainEvent> {
        if self.failed {
            return Vec::new();
        }
        let (program, logs) = match &self.payload {
            RawPayload::Logs { program, logs } => (*program, logs),
        };
        let refs: Vec<&str> = logs.iter().map(|s| s.as_str()).collect();
        let signature = match self.signature {
            Some(sig) => sig,
            None => return Vec::new(),
        };

        let mut out = Vec::new();
        let mut ordinal: u16 = 0;
        for entry in sniper_protocols::logs::extract_program_data(&refs, &program) {
            let Some(event) = PumpEvent::decode(&entry.payload) else {
                continue;
            };
            let payload = match event {
                PumpEvent::Create(ev) => EventPayload::TokenCreated {
                    mint: ev.mint,
                    creator: ev.creator,
                    name: ev.name,
                    symbol: ev.symbol,
                    uri: ev.uri,
                },
                PumpEvent::Trade(ev) => EventPayload::TradeObserved {
                    mint: ev.mint,
                    trader: ev.user,
                    is_buy: ev.is_buy,
                    quote_amount: Lamports(ev.sol_amount),
                    token_amount: TokenAmount::from_base_units(
                        u128::from(ev.token_amount),
                        PUMP_BASE_DECIMALS,
                    ),
                },
                PumpEvent::Complete(ev) => EventPayload::MigrationObserved {
                    mint: ev.mint,
                    to: Venue::PumpSwap,
                },
                PumpEvent::MigrateCreator(ev) => EventPayload::WalletActivity {
                    wallet: ev.new_creator,
                    mint: ev.mint,
                },
                // Detected but not decoded to a payload yet (layout M3).
                PumpEvent::PostCompleteBuyDetected | PumpEvent::UnknownDiscriminator => {
                    continue;
                }
            };
            out.push(ChainEvent {
                id: ChainEventId {
                    signature,
                    outer_ix_index: 0,
                    inner_ix_index: None,
                    event_ordinal: ordinal,
                },
                venue: Venue::PumpFun,
                slot: self.slot.unwrap_or(0),
                commitment: self.commitment,
                received: self.received,
                source: self.provider,
                schema_version: 1,
                payload,
            });
            ordinal = ordinal.saturating_add(1);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sniper_protocols::events::DISC_CREATE_EVENT;
    use sniper_protocols::registry::program_id;
    use solana_sdk::signature::Keypair;
    use solana_sdk::signer::Signer;

    fn b64(data: &[u8]) -> String {
        const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        let mut acc: u32 = 0;
        let mut bits: u32 = 0;
        for b in data {
            acc = (acc << 8) | *b as u32;
            bits += 8;
            while bits >= 6 {
                bits -= 6;
                out.push(A[(acc >> bits) as usize & 0x3f] as char);
                acc &= (1 << bits) - 1;
            }
        }
        if bits > 0 {
            out.push(A[(acc << (6 - bits)) as usize & 0x3f] as char);
        }
        out
    }

    fn create_payload() -> Vec<u8> {
        let mut body = Vec::new();
        let put = |buf: &mut Vec<u8>, s: &str| {
            buf.extend_from_slice(&(s.len() as u32).to_le_bytes());
            buf.extend_from_slice(s.as_bytes());
        };
        put(&mut body, "Coin");
        put(&mut body, "CN");
        put(&mut body, "https://x/y.json");
        body.extend_from_slice(&[1u8; 32]);
        body.extend_from_slice(&[2u8; 32]);
        body.extend_from_slice(&[3u8; 32]);
        body.extend_from_slice(&[4u8; 32]);
        body.extend_from_slice(&1i64.to_le_bytes());
        for _ in 0..4 {
            body.extend_from_slice(&1u64.to_le_bytes());
        }
        body.extend_from_slice(&[5u8; 32]);
        body.push(0);
        body.push(0);
        let mut payload = DISC_CREATE_EVENT.to_vec();
        payload.extend_from_slice(&body);
        payload
    }

    fn observation(logs: Vec<String>, failed: bool) -> RawObservation {
        let pump = program_id(Venue::PumpFun).unwrap();
        RawObservation {
            provider: ProviderKind::Logs,
            signature: Some(Keypair::new().sign_message(b"m")),
            slot: Some(1234),
            commitment: Commitment::Processed,
            received: Timestamps {
                wall: std::time::SystemTime::UNIX_EPOCH,
                mono_ns: 1,
            },
            failed,
            payload: RawPayload::Logs {
                program: pump,
                logs,
            },
        }
    }

    #[test]
    fn failed_transactions_yield_no_events() {
        let pump = program_id(Venue::PumpFun).unwrap();
        let logs = vec![
            format!("Program {pump} invoke [1]"),
            "Program log: Instruction: Create".into(),
            format!("Program data: {}", b64(&create_payload())),
            format!("Program {pump} success"),
        ];
        assert!(observation(logs, true).into_events().is_empty());
    }

    #[test]
    fn create_observation_maps_to_chain_event() {
        let pump = program_id(Venue::PumpFun).unwrap();
        let logs = vec![
            format!("Program {pump} invoke [1]"),
            "Program log: Instruction: Create".into(),
            format!("Program data: {}", b64(&create_payload())),
            format!("Program {pump} success"),
        ];
        let events = observation(logs, false).into_events();
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert!(matches!(ev.payload, EventPayload::TokenCreated { .. }));
        assert_eq!(ev.slot, 1234);
        assert_eq!(ev.commitment, Commitment::Processed);
        assert_eq!(ev.source, ProviderKind::Logs);
        assert_eq!(ev.id.event_ordinal, 0);
    }

    #[test]
    fn multiple_events_get_distinct_ordinals() {
        let pump = program_id(Venue::PumpFun).unwrap();
        let payload = create_payload();
        let encoded = b64(&payload);
        let logs = vec![
            format!("Program {pump} invoke [1]"),
            format!("Program data: {encoded}"),
            format!("Program data: {encoded}"),
            format!("Program {pump} success"),
        ];
        let events = observation(logs, false).into_events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].id.event_ordinal, 0);
        assert_eq!(events[1].id.event_ordinal, 1);
        assert_ne!(events[0].id, events[1].id);
    }

    #[test]
    fn other_programs_payloads_are_ignored() {
        let pump = program_id(Venue::PumpFun).unwrap();
        let other = solana_sdk::pubkey::Pubkey::new_unique();
        let payload = create_payload();
        let encoded = b64(&payload);
        let logs = vec![
            format!("Program {other} invoke [1]"),
            format!("Program data: {encoded}"),
            format!("Program {other} success"),
        ];
        assert!(observation(logs, false).into_events().is_empty());
        let _ = pump;
    }
}
