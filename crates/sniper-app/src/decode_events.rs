//! `sniper decode-events` — decode fixture payloads and emit JSON shaped for
//! differential comparison against the chainstack oracle output
//! (`scripts/gen_differential_fixtures.py` + `scripts/compare_differential.py`).

use std::path::PathBuf;

use serde_json::{json, Map, Value};
use sniper_protocols::events::{PumpEvent, DISC_POST_COMPLETE_BUY_EVENT};
use sniper_protocols::logs::base64_decode;
use solana_sdk::pubkey::Pubkey;

pub fn run(fixtures: PathBuf) -> Result<String, String> {
    let text = std::fs::read_to_string(&fixtures)
        .map_err(|e| format!("read {}: {e}", fixtures.display()))?;
    let mut out_lines = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let rec: Value =
            serde_json::from_str(line).map_err(|e| format!("bad fixture line: {e}"))?;
        let index = rec["index"].as_u64().unwrap_or(0);
        let payload_b64 = rec["payload_b64"].as_str().unwrap_or("");
        let bytes = base64_decode(payload_b64).map_err(|e| format!("fixture {index}: {e}"))?;
        let result = match PumpEvent::decode(&bytes) {
            Some(event) => event_to_oracle_json(&event),
            None => Value::String("__decode_failed__".into()),
        };
        out_lines.push(json!({"index": index, "rust": result}).to_string());
    }
    Ok(out_lines.join("\n"))
}

/// Map a decoded event into the oracle's `fields` shape: IDL field names,
/// pubkeys as base58 strings, exact value types.
fn event_to_oracle_json(event: &PumpEvent) -> Value {
    let mut m = Map::new();
    let pk = |v: &Pubkey| Value::String(v.to_string());
    match event {
        PumpEvent::Create(e) => {
            m.insert("name".into(), json!(e.name));
            m.insert("symbol".into(), json!(e.symbol));
            m.insert("uri".into(), json!(e.uri));
            m.insert("mint".into(), pk(&e.mint));
            m.insert("bonding_curve".into(), pk(&e.bonding_curve));
            m.insert("user".into(), pk(&e.user));
            m.insert("creator".into(), pk(&e.creator));
            m.insert("timestamp".into(), json!(e.timestamp));
            m.insert(
                "virtual_token_reserves".into(),
                json!(e.virtual_token_reserves),
            );
            m.insert("virtual_sol_reserves".into(), json!(e.virtual_sol_reserves));
            m.insert("real_token_reserves".into(), json!(e.real_token_reserves));
            m.insert("token_total_supply".into(), json!(e.token_total_supply));
            m.insert("token_program".into(), pk(&e.token_program));
            m.insert("is_mayhem_mode".into(), json!(e.is_mayhem_mode));
            m.insert("is_cashback_enabled".into(), json!(e.is_cashback_enabled));
            // Emit each trailing field only when it actually decoded.
            if e.decoded_fields > 15 {
                m.insert("quote_mint".into(), pk(&e.quote_mint));
            }
            if e.decoded_fields > 16 {
                m.insert(
                    "virtual_quote_reserves".into(),
                    json!(e.virtual_quote_reserves),
                );
            }
            if e.decoded_fields > 17 {
                m.insert("creator_fee_bps".into(), json!(e.creator_fee_bps));
            }
            if e.decoded_fields > 18 {
                m.insert("is_holder_reward".into(), json!(e.is_holder_reward));
            }
            m.insert("__truncated__".into(), json!(e.is_truncated()));
        }
        PumpEvent::Trade(e) => {
            m.insert("mint".into(), pk(&e.mint));
            m.insert("sol_amount".into(), json!(e.sol_amount));
            m.insert("token_amount".into(), json!(e.token_amount));
            m.insert("is_buy".into(), json!(e.is_buy));
            m.insert("user".into(), pk(&e.user));
            m.insert("timestamp".into(), json!(e.timestamp));
            m.insert("virtual_sol_reserves".into(), json!(e.virtual_sol_reserves));
            m.insert(
                "virtual_token_reserves".into(),
                json!(e.virtual_token_reserves),
            );
            m.insert("real_sol_reserves".into(), json!(e.real_sol_reserves));
            m.insert("real_token_reserves".into(), json!(e.real_token_reserves));
            m.insert("fee_recipient".into(), pk(&e.fee_recipient));
            m.insert("fee_basis_points".into(), json!(e.fee_basis_points));
            m.insert("fee".into(), json!(e.fee));
            m.insert("creator".into(), pk(&e.creator));
            m.insert(
                "creator_fee_basis_points".into(),
                json!(e.creator_fee_basis_points),
            );
            m.insert("creator_fee".into(), json!(e.creator_fee));
            m.insert("track_volume".into(), json!(e.track_volume));
            m.insert(
                "total_unclaimed_tokens".into(),
                json!(e.total_unclaimed_tokens),
            );
            m.insert("total_claimed_tokens".into(), json!(e.total_claimed_tokens));
            m.insert("current_sol_volume".into(), json!(e.current_sol_volume));
            m.insert(
                "last_update_timestamp".into(),
                json!(e.last_update_timestamp),
            );
            m.insert("ix_name".into(), json!(e.ix_name));
            m.insert("mayhem_mode".into(), json!(e.mayhem_mode));
            m.insert(
                "cashback_fee_basis_points".into(),
                json!(e.cashback_fee_basis_points),
            );
            m.insert("cashback".into(), json!(e.cashback));
            m.insert(
                "buyback_fee_basis_points".into(),
                json!(e.buyback_fee_basis_points),
            );
            m.insert("buyback_fee".into(), json!(e.buyback_fee));
            m.insert(
                "shareholders".into(),
                Value::Array(
                    e.shareholders
                        .iter()
                        .map(
                            |s| json!({"address": s.address.to_string(), "share_bps": s.share_bps}),
                        )
                        .collect(),
                ),
            );
            if e.decoded_fields > 28 {
                m.insert("quote_mint".into(), pk(&e.quote_mint));
            }
            if e.decoded_fields > 29 {
                m.insert("quote_amount".into(), json!(e.quote_amount));
            }
            if e.decoded_fields > 30 {
                m.insert(
                    "virtual_quote_reserves".into(),
                    json!(e.virtual_quote_reserves),
                );
            }
            if e.decoded_fields > 31 {
                m.insert("real_quote_reserves".into(), json!(e.real_quote_reserves));
            }
            if e.decoded_fields > 32 {
                m.insert("holder_rewards_bps".into(), json!(e.holder_rewards_bps));
            }
            if e.decoded_fields > 33 {
                m.insert("holder_rewards".into(), json!(e.holder_rewards));
            }
            m.insert("__truncated__".into(), json!(e.is_truncated()));
        }
        PumpEvent::Complete(e) => {
            m.insert("user".into(), pk(&e.user));
            m.insert("mint".into(), pk(&e.mint));
            m.insert("bonding_curve".into(), pk(&e.bonding_curve));
            m.insert("timestamp".into(), json!(e.timestamp));
            if e.decoded_fields > 4 {
                m.insert("quote_mint".into(), pk(&e.quote_mint));
            }
            m.insert("__truncated__".into(), json!(e.is_truncated()));
        }
        PumpEvent::MigrateCreator(e) => {
            m.insert("timestamp".into(), json!(e.timestamp));
            m.insert("mint".into(), pk(&e.mint));
            m.insert("bonding_curve".into(), pk(&e.bonding_curve));
            m.insert("sharing_config".into(), pk(&e.sharing_config));
            m.insert("old_creator".into(), pk(&e.old_creator));
            m.insert("new_creator".into(), pk(&e.new_creator));
            m.insert("__truncated__".into(), json!(e.is_truncated()));
        }
        PumpEvent::PostCompleteBuyDetected => {
            m.insert(
                "__detected__".into(),
                json!(hex_of(&DISC_POST_COMPLETE_BUY_EVENT)),
            );
        }
        PumpEvent::UnknownDiscriminator => {
            m.insert("__unknown__".into(), json!(true));
        }
    }
    Value::Object(m)
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
