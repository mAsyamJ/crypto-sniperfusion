//! pump.fun Anchor event decoding (borsh) with Chainstack-oracle parity.
//!
//! Layout authority: the IDL `types` section (CreateEvent/TradeEvent/… field
//! order), discriminators from the official IDL @ 639b87f. Log semantics
//! mirror the Apache-2.0 chainstack parser: `Program data:` payloads are
//! attributed to the invoking program via invoke/success/failed frames, and
//! the create gate matches whole `Instruction: Create`/`Instruction: CreateV2`
//! lines so other programs' `Create*` instructions cannot pass.

use solana_sdk::pubkey::Pubkey;

/// Anchor event discriminator: first 8 bytes of `sha256("event:<Name>")`.
/// Runtime helper used by tests to keep the constants below honest.
pub fn event_discriminator(name: &str) -> [u8; 8] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"event:");
    hasher.update(name.as_bytes());
    let d = hasher.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&d[..8]);
    out
}

/// Discriminators from the official IDL @ 639b87f (they equal the vendored
/// chainstack IDL's). The test below re-derives each via sha256("event:…").
pub const DISC_CREATE_EVENT: [u8; 8] = [0x1b, 0x72, 0xa9, 0x4d, 0xde, 0xeb, 0x63, 0x76];
pub const DISC_TRADE_EVENT: [u8; 8] = [0xbd, 0xdb, 0x7f, 0xd3, 0x4e, 0xe6, 0x61, 0xee];
pub const DISC_COMPLETE_EVENT: [u8; 8] = [0x5f, 0x72, 0x61, 0x9c, 0xd4, 0x2e, 0x98, 0x08];
pub const DISC_MIGRATE_CREATOR_EVENT: [u8; 8] = [0x9b, 0xa7, 0x68, 0xdc, 0xd5, 0x6c, 0xf3, 0x03];
/// Official-IDL-only (post-synthetic-migration buy). Layout lands at M3 with
/// the synthetic-migration quoting work; detection exists so those events are
/// counted, never silently dropped.
pub const DISC_POST_COMPLETE_BUY_EVENT: [u8; 8] = [0x6f, 0xb0, 0x6d, 0x8b, 0x31, 0x6c, 0xd5, 0xfb];

/// Checked borsh reader: every read validates bounds; no panics on input.
#[derive(Debug)]
pub struct BorshReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> BorshReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    pub fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.remaining() < n {
            None
        } else {
            let s = &self.buf[self.pos..self.pos + n];
            self.pos += n;
            Some(s)
        }
    }

    pub fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|s| s[0])
    }

    pub fn u16(&mut self) -> Option<u16> {
        self.take(2).map(|s| u16::from_le_bytes([s[0], s[1]]))
    }

    pub fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    pub fn u64(&mut self) -> Option<u64> {
        self.take(8).map(|s| {
            let mut a = [0u8; 8];
            a.copy_from_slice(s);
            u64::from_le_bytes(a)
        })
    }

    pub fn i64(&mut self) -> Option<i64> {
        self.u64().map(|v| v as i64)
    }

    pub fn bool(&mut self) -> Option<bool> {
        self.u8().map(|b| b != 0)
    }

    pub fn pubkey(&mut self) -> Option<Pubkey> {
        self.take(32).map(|s| {
            let mut a = [0u8; 32];
            a.copy_from_slice(s);
            Pubkey::from(a)
        })
    }

    pub fn string(&mut self) -> Option<String> {
        let len = self.u32()? as usize;
        // Sanity bound: log payloads are bounded by transaction log size;
        // anything huge here is a malformed event.
        if len > 1_048_576 {
            return None;
        }
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).ok()
    }

    pub fn vec_len(&mut self) -> Option<usize> {
        let len = self.u32()? as usize;
        // Shareholder lists are tiny; bound to keep adversarial payloads sane.
        if len > 1_048_576 {
            return None;
        }
        Some(len)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Shareholder {
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub address: Pubkey,
    pub share_bps: u16,
}

/// CreateEvent per IDL field order. Trailing fields (quote_mint onward) are
/// the non-SOL-quote era additions; older on-chain events may omit them —
/// `truncated` records that.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateEvent {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub mint: Pubkey,
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub bonding_curve: Pubkey,
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub user: Pubkey,
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub creator: Pubkey,
    pub timestamp: i64,
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub token_total_supply: u64,
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub token_program: Pubkey,
    pub is_mayhem_mode: bool,
    pub is_cashback_enabled: bool,
    // Trailing (may be absent on older events):
    #[serde(default, with = "sniper_core::serde_keys::pubkey_str")]
    pub quote_mint: Pubkey,
    pub virtual_quote_reserves: u64,
    pub creator_fee_bps: u64,
    pub is_holder_reward: bool,
    /// How many of the 19 IDL fields were decoded (oracle-parity: any
    /// field-boundary EOF keeps the prefix). `is_truncated()` reports the gap.
    pub decoded_fields: u16,
}

impl CreateEvent {
    pub const TOTAL_FIELDS: u16 = 19;
    pub fn is_truncated(&self) -> bool {
        self.decoded_fields < Self::TOTAL_FIELDS
    }
}

/// TradeEvent per IDL field order (current program layout).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TradeEvent {
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub is_buy: bool,
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub user: Pubkey,
    pub timestamp: i64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub fee_recipient: Pubkey,
    pub fee_basis_points: u64,
    pub fee: u64,
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub creator: Pubkey,
    pub creator_fee_basis_points: u64,
    pub creator_fee: u64,
    pub track_volume: bool,
    pub total_unclaimed_tokens: u64,
    pub total_claimed_tokens: u64,
    pub current_sol_volume: u64,
    pub last_update_timestamp: i64,
    pub ix_name: String,
    pub mayhem_mode: bool,
    pub cashback_fee_basis_points: u64,
    pub cashback: u64,
    pub buyback_fee_basis_points: u64,
    pub buyback_fee: u64,
    pub shareholders: Vec<Shareholder>,
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub quote_mint: Pubkey,
    pub quote_amount: u64,
    pub virtual_quote_reserves: u64,
    pub real_quote_reserves: u64,
    pub holder_rewards_bps: u64,
    pub holder_rewards: u64,
    /// How many of the 35 IDL fields were decoded (shareholders counts as
    /// one field when its vec decoded completely).
    pub decoded_fields: u16,
}

impl TradeEvent {
    pub const TOTAL_FIELDS: u16 = 34;
    pub fn is_truncated(&self) -> bool {
        self.decoded_fields < Self::TOTAL_FIELDS
    }
}

/// CompleteEvent (curve graduation).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CompleteEvent {
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub user: Pubkey,
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub mint: Pubkey,
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub bonding_curve: Pubkey,
    pub timestamp: i64,
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub quote_mint: Pubkey,
    /// Decoded count of the 5 IDL fields (4 = old format without quote_mint).
    pub decoded_fields: u16,
}

impl CompleteEvent {
    pub const TOTAL_FIELDS: u16 = 5;
    pub fn is_truncated(&self) -> bool {
        self.decoded_fields < Self::TOTAL_FIELDS
    }
}

/// MigrateBondingCurveCreatorEvent — the creator-fee-sharing creator rewrite
/// in the coin's creation transaction; the canonical creator for risk checks.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MigrateBondingCurveCreatorEvent {
    pub timestamp: i64,
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub mint: Pubkey,
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub bonding_curve: Pubkey,
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub sharing_config: Pubkey,
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub old_creator: Pubkey,
    #[serde(with = "sniper_core::serde_keys::pubkey_str")]
    pub new_creator: Pubkey,
    pub decoded_fields: u16,
}

impl MigrateBondingCurveCreatorEvent {
    pub const TOTAL_FIELDS: u16 = 6;
    pub fn is_truncated(&self) -> bool {
        self.decoded_fields < Self::TOTAL_FIELDS
    }
}

/// Decoded pump.fun event, discriminator-dispatched.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PumpEvent {
    Create(CreateEvent),
    Trade(TradeEvent),
    Complete(CompleteEvent),
    MigrateCreator(MigrateBondingCurveCreatorEvent),
    /// PostCompleteBuyEvent (synthetic migration pool leg). Layout pending M3.
    PostCompleteBuyDetected,
    UnknownDiscriminator,
}

impl PumpEvent {
    /// Decode one raw `Program data:` payload (discriminator + borsh fields).
    /// Total function: malformed input → None, never a panic.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 8 {
            return None;
        }
        let (disc, body) = bytes.split_at(8);
        match disc {
            d if d == DISC_CREATE_EVENT => Self::decode_create(body).map(PumpEvent::Create),
            d if d == DISC_TRADE_EVENT => Self::decode_trade(body).map(PumpEvent::Trade),
            d if d == DISC_COMPLETE_EVENT => Self::decode_complete(body).map(PumpEvent::Complete),
            d if d == DISC_MIGRATE_CREATOR_EVENT => {
                Self::decode_migrate_creator(body).map(PumpEvent::MigrateCreator)
            }
            d if d == DISC_POST_COMPLETE_BUY_EVENT => Some(PumpEvent::PostCompleteBuyDetected),
            _ => Some(PumpEvent::UnknownDiscriminator),
        }
    }

    fn decode_create(b: &[u8]) -> Option<CreateEvent> {
        let mut r = BorshReader::new(b);
        // Oracle-parity: decode sequentially; a field that hits EOF ends the
        // decode with the fields decoded so far (prefix-tolerant). Any prefix
        // shorter than the required identity fields (mint/creator/name/symbol/
        // uri) is not a usable event.
        let name = r.string()?;
        let symbol = r.string()?;
        let uri = r.string()?;
        let mint = r.pubkey()?;
        let bonding_curve = r.pubkey()?;
        let user = r.pubkey()?;
        let creator = r.pubkey()?;
        let timestamp = r.i64()?;
        let mut ev = CreateEvent {
            name,
            symbol,
            uri,
            mint,
            bonding_curve,
            user,
            creator,
            timestamp,
            virtual_token_reserves: 0,
            virtual_sol_reserves: 0,
            real_token_reserves: 0,
            token_total_supply: 0,
            token_program: Pubkey::default(),
            is_mayhem_mode: false,
            is_cashback_enabled: false,
            quote_mint: Pubkey::default(),
            virtual_quote_reserves: 0,
            creator_fee_bps: 0,
            is_holder_reward: false,
            decoded_fields: 8,
        };
        let mut n = 8u16;
        macro_rules! next {
            ($v:expr) => {{
                let Some(v) = $v else {
                    ev.decoded_fields = n;
                    return Some(ev);
                };
                n += 1;
                v
            }};
        }
        ev.virtual_token_reserves = next!(r.u64());
        ev.virtual_sol_reserves = next!(r.u64());
        ev.real_token_reserves = next!(r.u64());
        ev.token_total_supply = next!(r.u64());
        ev.token_program = next!(r.pubkey());
        ev.is_mayhem_mode = next!(r.bool());
        ev.is_cashback_enabled = next!(r.bool());
        ev.quote_mint = next!(r.pubkey());
        ev.virtual_quote_reserves = next!(r.u64());
        ev.creator_fee_bps = next!(r.u64());
        ev.is_holder_reward = next!(r.bool());
        ev.decoded_fields = n;
        Some(ev)
    }

    fn decode_trade(b: &[u8]) -> Option<TradeEvent> {
        let mut r = BorshReader::new(b);
        // First 28 fields are required as a block (identity + fees).
        let mint = r.pubkey()?;
        let sol_amount = r.u64()?;
        let token_amount = r.u64()?;
        let is_buy = r.bool()?;
        let user = r.pubkey()?;
        let timestamp = r.i64()?;
        let vsr = r.u64()?;
        let vtr = r.u64()?;
        let rsr = r.u64()?;
        let rtr = r.u64()?;
        let fee_recipient = r.pubkey()?;
        let fee_basis_points = r.u64()?;
        let fee = r.u64()?;
        let creator = r.pubkey()?;
        let creator_fee_basis_points = r.u64()?;
        let creator_fee = r.u64()?;
        let track_volume = r.bool()?;
        let total_unclaimed_tokens = r.u64()?;
        let total_claimed_tokens = r.u64()?;
        let current_sol_volume = r.u64()?;
        let last_update_timestamp = r.i64()?;
        let ix_name = r.string()?;
        let mayhem_mode = r.bool()?;
        let cashback_fee_basis_points = r.u64()?;
        let cashback = r.u64()?;
        let buyback_fee_basis_points = r.u64()?;
        let buyback_fee = r.u64()?;
        let mut ev = TradeEvent {
            mint,
            sol_amount,
            token_amount,
            is_buy,
            user,
            timestamp,
            virtual_sol_reserves: vsr,
            virtual_token_reserves: vtr,
            real_sol_reserves: rsr,
            real_token_reserves: rtr,
            fee_recipient,
            fee_basis_points,
            fee,
            creator,
            creator_fee_basis_points,
            creator_fee,
            track_volume,
            total_unclaimed_tokens,
            total_claimed_tokens,
            current_sol_volume,
            last_update_timestamp,
            ix_name,
            mayhem_mode,
            cashback_fee_basis_points,
            cashback,
            buyback_fee_basis_points,
            buyback_fee,
            shareholders: Vec::new(),
            quote_mint: Pubkey::default(),
            quote_amount: 0,
            virtual_quote_reserves: 0,
            real_quote_reserves: 0,
            holder_rewards_bps: 0,
            holder_rewards: 0,
            decoded_fields: 0,
        };
        // Shareholders vec. NOTE: the chainstack ORACLE cannot decode
        // vec<defined> and silently skips it WITHOUT advancing its offset —
        // its post-shareholder fields are misaligned garbage when the vec is
        // non-empty. We decode correctly per IDL; the comparator excludes
        // oracle post-shareholder fields when the vec was non-empty. See
        // docs/evidence/M2.md discrepancy D-1.
        let Some(count) = r.vec_len() else {
            return Some(ev);
        };
        let mut shareholders = Vec::with_capacity(count.min(64));
        let mut vec_ok = true;
        for _ in 0..count {
            let Some(address) = r.pubkey() else {
                vec_ok = false;
                break;
            };
            let Some(share_bps) = r.u16() else {
                vec_ok = false;
                break;
            };
            shareholders.push(Shareholder { address, share_bps });
        }
        ev.decoded_fields = 27;
        if !vec_ok {
            return Some(ev);
        }
        ev.shareholders = shareholders;
        let mut n = 28u16;
        macro_rules! next {
            ($v:expr) => {{
                let Some(v) = $v else {
                    ev.decoded_fields = n;
                    return Some(ev);
                };
                n += 1;
                v
            }};
        }
        ev.quote_mint = next!(r.pubkey());
        ev.quote_amount = next!(r.u64());
        ev.virtual_quote_reserves = next!(r.u64());
        ev.real_quote_reserves = next!(r.u64());
        ev.holder_rewards_bps = next!(r.u64());
        ev.holder_rewards = next!(r.u64());
        ev.decoded_fields = n;
        Some(ev)
    }

    fn decode_complete(b: &[u8]) -> Option<CompleteEvent> {
        let mut r = BorshReader::new(b);
        let user = r.pubkey()?;
        let mint = r.pubkey()?;
        let bonding_curve = r.pubkey()?;
        let timestamp = r.i64()?;
        let mut ev = CompleteEvent {
            user,
            mint,
            bonding_curve,
            timestamp,
            quote_mint: Pubkey::default(),
            decoded_fields: 4,
        };
        if let Some(qm) = r.pubkey() {
            ev.quote_mint = qm;
            ev.decoded_fields = 5;
        }
        Some(ev)
    }

    fn decode_migrate_creator(b: &[u8]) -> Option<MigrateBondingCurveCreatorEvent> {
        let mut r = BorshReader::new(b);
        Some(MigrateBondingCurveCreatorEvent {
            timestamp: r.i64()?,
            mint: r.pubkey()?,
            bonding_curve: r.pubkey()?,
            sharing_config: r.pubkey()?,
            old_creator: r.pubkey()?,
            new_creator: r.pubkey()?,
            decoded_fields: 6,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Discriminators asserted against both IDLs (official pump-docs @ 639b87f
    // and chainstack vendored — they agree) AND re-derived via sha256.
    #[test]
    fn discriminators_match_idl_constants() {
        assert_eq!(DISC_CREATE_EVENT, event_discriminator("CreateEvent"));
        assert_eq!(DISC_TRADE_EVENT, event_discriminator("TradeEvent"));
        assert_eq!(DISC_COMPLETE_EVENT, event_discriminator("CompleteEvent"));
        assert_eq!(
            DISC_MIGRATE_CREATOR_EVENT,
            event_discriminator("MigrateBondingCurveCreatorEvent")
        );
        assert_eq!(
            DISC_POST_COMPLETE_BUY_EVENT,
            event_discriminator("PostCompleteBuyEvent")
        );
        assert_eq!(
            DISC_CREATE_EVENT,
            [0x1b, 0x72, 0xa9, 0x4d, 0xde, 0xeb, 0x63, 0x76]
        );
        assert_eq!(
            DISC_TRADE_EVENT,
            [0xbd, 0xdb, 0x7f, 0xd3, 0x4e, 0xe6, 0x61, 0xee]
        );
        assert_eq!(
            DISC_COMPLETE_EVENT,
            [0x5f, 0x72, 0x61, 0x9c, 0xd4, 0x2e, 0x98, 0x08]
        );
        assert_eq!(
            DISC_POST_COMPLETE_BUY_EVENT,
            [0x6f, 0xb0, 0x6d, 0x8b, 0x31, 0x6c, 0xd5, 0xfb]
        );
        assert_eq!(
            DISC_MIGRATE_CREATOR_EVENT,
            [0x9b, 0xa7, 0x68, 0xdc, 0xd5, 0x6c, 0xf3, 0x03]
        );
    }

    fn put_string(buf: &mut Vec<u8>, s: &str) {
        buf.extend_from_slice(&(s.len() as u32).to_le_bytes());
        buf.extend_from_slice(s.as_bytes());
    }

    fn put_pubkey(buf: &mut Vec<u8>, seed: u8) {
        buf.extend_from_slice(&[seed; 32]);
    }

    #[test]
    fn decode_create_event_roundtrip_full() {
        let mut body = Vec::new();
        put_string(&mut body, "Test Coin");
        put_string(&mut body, "TST");
        put_string(&mut body, "https://example.com/m.json");
        put_pubkey(&mut body, 1); // mint
        put_pubkey(&mut body, 2); // bonding_curve
        put_pubkey(&mut body, 3); // user
        put_pubkey(&mut body, 4); // creator
        body.extend_from_slice(&79i64.to_le_bytes());
        body.extend_from_slice(&1_000u64.to_le_bytes());
        body.extend_from_slice(&2_000u64.to_le_bytes());
        body.extend_from_slice(&3_000u64.to_le_bytes());
        body.extend_from_slice(&1_000_000_000_000u64.to_le_bytes());
        put_pubkey(&mut body, 5); // token_program
        body.push(1); // is_mayhem_mode
        body.push(0); // is_cashback_enabled
        put_pubkey(&mut body, 6); // quote_mint
        body.extend_from_slice(&7_000u64.to_le_bytes()); // virtual_quote_reserves
        body.extend_from_slice(&50u64.to_le_bytes()); // creator_fee_bps
        body.push(0); // is_holder_reward

        let mut payload = DISC_CREATE_EVENT.to_vec();
        payload.extend_from_slice(&body);
        let Some(PumpEvent::Create(ev)) = PumpEvent::decode(&payload) else {
            panic!("must decode as CreateEvent");
        };
        assert_eq!(ev.name, "Test Coin");
        assert_eq!(ev.symbol, "TST");
        assert_eq!(ev.creator, Pubkey::from([4u8; 32]));
        assert!(ev.is_mayhem_mode);
        assert!(!ev.is_cashback_enabled);
        assert_eq!(ev.creator_fee_bps, 50);
        assert!(!ev.is_truncated(), "all trailing fields present");
        assert_eq!(ev.decoded_fields, 19);
    }

    #[test]
    fn decode_create_event_tolerates_missing_trailing_fields() {
        // Older event: ends after is_cashback_enabled.
        let mut body = Vec::new();
        put_string(&mut body, "N");
        put_string(&mut body, "S");
        put_string(&mut body, "u");
        put_pubkey(&mut body, 1);
        put_pubkey(&mut body, 2);
        put_pubkey(&mut body, 3);
        put_pubkey(&mut body, 4);
        body.extend_from_slice(&1i64.to_le_bytes());
        body.extend_from_slice(&1u64.to_le_bytes());
        body.extend_from_slice(&1u64.to_le_bytes());
        body.extend_from_slice(&1u64.to_le_bytes());
        body.extend_from_slice(&1u64.to_le_bytes());
        put_pubkey(&mut body, 5);
        body.push(0);
        body.push(0);

        let mut payload = DISC_CREATE_EVENT.to_vec();
        payload.extend_from_slice(&body);
        let Some(PumpEvent::Create(ev)) = PumpEvent::decode(&payload) else {
            panic!("must decode");
        };
        assert!(ev.is_truncated());
        assert_eq!(ev.decoded_fields, 15);
        assert_eq!(ev.quote_mint, Pubkey::default());
        assert!(!ev.is_mayhem_mode);
    }

    #[test]
    fn decode_trade_event_roundtrip() {
        let mut body = Vec::new();
        put_pubkey(&mut body, 1); // mint
        body.extend_from_slice(&100_000u64.to_le_bytes()); // sol_amount
        body.extend_from_slice(&250_000u64.to_le_bytes()); // token_amount
        body.push(1); // is_buy
        put_pubkey(&mut body, 2); // user
        body.extend_from_slice(&99i64.to_le_bytes()); // timestamp
        body.extend_from_slice(&10u64.to_le_bytes());
        body.extend_from_slice(&20u64.to_le_bytes());
        body.extend_from_slice(&30u64.to_le_bytes());
        body.extend_from_slice(&40u64.to_le_bytes());
        put_pubkey(&mut body, 3); // fee_recipient
        body.extend_from_slice(&100u64.to_le_bytes()); // fee_basis_points
        body.extend_from_slice(&10u64.to_le_bytes()); // fee
        put_pubkey(&mut body, 4); // creator
        body.extend_from_slice(&50u64.to_le_bytes()); // creator_fee_basis_points
        body.extend_from_slice(&5u64.to_le_bytes()); // creator_fee
        body.push(1); // track_volume
        body.extend_from_slice(&0u64.to_le_bytes());
        body.extend_from_slice(&0u64.to_le_bytes());
        body.extend_from_slice(&0u64.to_le_bytes());
        body.extend_from_slice(&0i64.to_le_bytes());
        put_string(&mut body, "buy_v3"); // ix_name
        body.push(0); // mayhem_mode
        body.extend_from_slice(&0u64.to_le_bytes()); // cashback bps
        body.extend_from_slice(&0u64.to_le_bytes()); // cashback
        body.extend_from_slice(&0u64.to_le_bytes()); // buyback bps
        body.extend_from_slice(&0u64.to_le_bytes()); // buyback fee
        body.extend_from_slice(&2u32.to_le_bytes()); // shareholders.len
        put_pubkey(&mut body, 7);
        body.extend_from_slice(&5_000u16.to_le_bytes());
        put_pubkey(&mut body, 8);
        body.extend_from_slice(&2_500u16.to_le_bytes());
        put_pubkey(&mut body, 9); // quote_mint
        body.extend_from_slice(&111u64.to_le_bytes()); // quote_amount
        body.extend_from_slice(&222u64.to_le_bytes());
        body.extend_from_slice(&333u64.to_le_bytes());
        body.extend_from_slice(&44u64.to_le_bytes());
        body.extend_from_slice(&55u64.to_le_bytes());

        let mut payload = DISC_TRADE_EVENT.to_vec();
        payload.extend_from_slice(&body);
        let Some(PumpEvent::Trade(ev)) = PumpEvent::decode(&payload) else {
            panic!("must decode as TradeEvent");
        };
        assert!(ev.is_buy);
        assert_eq!(ev.ix_name, "buy_v3");
        assert_eq!(ev.shareholders.len(), 2);
        assert_eq!(ev.shareholders[1].share_bps, 2_500);
        assert_eq!(ev.quote_amount, 111);
        assert!(!ev.is_truncated());
        assert_eq!(ev.decoded_fields, 34);
    }

    #[test]
    fn decode_never_panics_on_garbage() {
        // Discriminator + truncated bodies of every length 0..40.
        for disc in [
            DISC_CREATE_EVENT,
            DISC_TRADE_EVENT,
            DISC_COMPLETE_EVENT,
            DISC_MIGRATE_CREATOR_EVENT,
        ] {
            for len in 0..40usize {
                let payload: Vec<u8> = disc
                    .iter()
                    .copied()
                    .chain(std::iter::repeat_n(0xAB, len))
                    .collect();
                // Must return Some/None without panicking; truncation is data.
                let _ = PumpEvent::decode(&payload);
            }
        }
        // Random discriminators.
        assert_eq!(
            PumpEvent::decode(&[0u8; 8]),
            Some(PumpEvent::UnknownDiscriminator)
        );
        assert_eq!(PumpEvent::decode(&[0u8; 4]), None);
    }
}
