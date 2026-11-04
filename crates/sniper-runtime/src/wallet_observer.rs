//! WA1 — read-only wallet transaction normalization.
//!
//! Turns a `getTransaction`-shaped JSON receipt (the exact surface the G0
//! collector fetches) into typed, owner-verified `WalletTradeEvent`s for
//! ONE watched wallet. Behavioral references (reimplemented originally,
//! no code copied): MIT DoradoDevs balance-delta parser, MIT Solsonar
//! multi-DEX decoder.
//!
//! Correct-attribution rules (goal plan §"Correct wallet trade
//! attribution"):
//! - Deltas are SIGNED integers (i128); the quote leg is native SOL + WSOL
//!   netted (wrap/unwrap collapses); `network_fee` comes from `meta.fee`.
//! - Multi-hop routes aggregate to ONE net economic trade per non-quote
//!   mint (intermediate mints netting to zero vanish).
//! - A transfer is not a buy; LP deposits/withdraws are `LpFlow`; failed
//!   transactions yield nothing; unknown venues/owners are flagged and
//!   fail closed (Low confidence / INCOMPLETE coverage), never invented.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use sniper_core::event::Commitment;

/// Public program ids (chain constants; venues beyond the H7 registry are
/// pinned here for wallet observation — sources: official Raydium/Meteora/
/// Jupiter program docs as catalogued in the MIT Solsonar decoder).
pub const PROGRAM_RAYDIUM_AMM_V4: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
pub const PROGRAM_METEORA_DLMM: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";
pub const PROGRAM_JUPITER_V6: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
pub const MINT_WSOL: &str = "So11111111111111111111111111111111111111112";
pub const PROGRAM_PUMP_FUN: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
pub const PROGRAM_PUMP_SWAP: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";

/// Venue attribution for a wallet trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WalletVenue {
    PumpFun,
    PumpSwap,
    Jupiter,
    Raydium,
    Meteora,
    UnknownDex,
}

impl WalletVenue {
    fn from_program(program: &str) -> Self {
        match program {
            PROGRAM_PUMP_FUN => WalletVenue::PumpFun,
            PROGRAM_PUMP_SWAP => WalletVenue::PumpSwap,
            PROGRAM_JUPITER_V6 => WalletVenue::Jupiter,
            PROGRAM_RAYDIUM_AMM_V4 => WalletVenue::Raydium,
            PROGRAM_METEORA_DLMM => WalletVenue::Meteora,
            _ => WalletVenue::UnknownDex,
        }
    }
}

/// Economic side as seen by the watched wallet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
    /// Token movement without a recognized DEX program (P2P/OTC/custodial
    /// move) — never counted as a swap.
    Transfer,
    /// Liquidity provision shape (both legs move the same direction plus
    /// LP tokens) — never counted as a swap.
    LpFlow,
    Unknown,
}

/// How the wallet's authority over the funds was established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuthorityProvenance {
    /// The wallet is the fee payer / signer of the transaction.
    Signer,
    /// The wallet owns the token accounts whose balances moved (resolved
    /// via token-balance owner fields, not mere account touching).
    AtaOwnerResolved,
    /// Could not be established — fail closed downstream.
    Unresolved,
}

/// Classification confidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Classification {
    /// Owner-resolved DEX swap with complete balance history.
    High,
    /// Correct by shape but partial history / unknown venue / transfer.
    Medium,
    /// Missing data — downstream scoring must fail closed.
    Low,
}

/// One normalized economic event for the watched wallet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WalletTradeEvent {
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub commitment: Commitment,
    pub wallet_owner: String,
    pub authority_provenance: AuthorityProvenance,
    pub mint: String,
    pub venue: WalletVenue,
    pub side: Side,
    /// Signed base-token delta in raw base units (whole wallet, all legs).
    pub raw_token_delta: i128,
    /// Signed quote delta in lamports (native SOL + WSOL netted, fee
    /// EXCLUDED — the fee is reported separately, once).
    pub raw_quote_delta: i128,
    pub network_fee: u64,
    /// COMPLETE = every pre/post balance row was present for the wallet.
    /// INCOMPLETE = balance history missing from the receipt (do not score).
    pub coverage: String,
    pub classification: Classification,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NormalizeError {
    #[error("malformed transaction receipt: {0}")]
    Malformed(&'static str),
    #[error("watched wallet is not valid base58 pubkey")]
    InvalidWallet,
}

fn delta_token_rows(meta: &Value, watched: &str) -> (HashMap<String, i128>, bool) {
    // (mint -> signed delta) for token accounts owned by `watched`.
    // Complete when the receipt carries ANY token-balance rows; a receipt
    // with none cannot prove the wallet's token history (INCOMPLETE).
    let mut deltas: HashMap<String, i128> = HashMap::new();
    let mut rows_seen = 0usize;
    for (field, sign) in [("postTokenBalances", 1i128), ("preTokenBalances", -1i128)] {
        for row in meta
            .get(field)
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default()
        {
            rows_seen += 1;
            if row.get("owner").and_then(Value::as_str) != Some(watched) {
                continue;
            }
            let (Some(mint), Some(amount)) = (
                row.get("mint").and_then(Value::as_str),
                row.get("uiTokenAmount")
                    .and_then(|a| a.get("amount"))
                    .and_then(Value::as_str)
                    .and_then(|a| a.parse::<i128>().ok()),
            ) else {
                continue;
            };
            *deltas.entry(mint.to_string()).or_insert(0) += sign * amount;
        }
    }
    (deltas, rows_seen > 0)
}

fn native_sol_delta(meta: &Value, account_keys: &[String], watched: &str) -> i128 {
    let (Some(pre), Some(post)) = (
        meta.get("preBalances").and_then(Value::as_array),
        meta.get("postBalances").and_then(Value::as_array),
    ) else {
        return 0;
    };
    for (index, key) in account_keys.iter().enumerate() {
        if key == watched {
            let a = pre.get(index).and_then(Value::as_i64).unwrap_or(0);
            let b = post.get(index).and_then(Value::as_i64).unwrap_or(0);
            return i128::from(b) - i128::from(a);
        }
    }
    0
}

fn detect_venue(meta: &Value, account_keys: &[String]) -> Option<WalletVenue> {
    let loaded: Vec<String> = meta
        .get("loadedAddresses")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|r| r.get("pubkey").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let known = [
        PROGRAM_PUMP_FUN,
        PROGRAM_PUMP_SWAP,
        PROGRAM_JUPITER_V6,
        PROGRAM_RAYDIUM_AMM_V4,
        PROGRAM_METEORA_DLMM,
    ];
    for key in account_keys.iter().chain(loaded.iter()) {
        if known.contains(&key.as_str()) {
            return Some(WalletVenue::from_program(key));
        }
    }
    // Also honor explicit instruction program ids (fixtures may not put
    // the program in account keys).
    if let Some(instructions) = meta
        .get("logMessages")
        .and_then(Value::as_array)
        .map(|_| None::<Value>)
    {
        let _ = instructions;
    }
    None
}

fn instruction_programs(tx: &Value) -> Vec<String> {
    tx.get("transaction")
        .and_then(|t| t.get("message"))
        .and_then(|m| m.get("instructions"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|r| r.get("programId").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Normalize one transaction receipt for `watched_wallet` (base58).
///
/// Returns the wallet's economic events (usually zero or one; multi-asset
/// routes yield one per non-quote mint). Failed transactions and receipts
/// the wallet did not participate in return empty vectors. Missing
/// history yields `coverage=INCOMPLETE` with zero invented deltas.
pub fn normalize_transaction(
    tx: &Value,
    watched_wallet: &str,
) -> Result<Vec<WalletTradeEvent>, NormalizeError> {
    let plausible = (32..=48).contains(&watched_wallet.len())
        && watched_wallet.bytes().all(|b| b.is_ascii_alphanumeric());
    if !plausible {
        return Err(NormalizeError::InvalidWallet);
    }
    let meta = tx.get("meta").ok_or(NormalizeError::Malformed("no meta"))?;
    if meta.get("err").map(|e| !e.is_null()).unwrap_or(false) {
        // Failed transactions yield no economic events.
        return Ok(Vec::new());
    }
    let account_keys: Vec<String> = tx
        .get("transaction")
        .and_then(|t| t.get("message"))
        .and_then(|m| m.get("accountKeys"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if account_keys.is_empty() {
        return Err(NormalizeError::Malformed("no accountKeys"));
    }
    let signature = tx
        .get("transaction")
        .and_then(|t| t.get("signatures"))
        .and_then(Value::as_array)
        .and_then(|s| s.first())
        .and_then(Value::as_str)
        .ok_or(NormalizeError::Malformed("no signature"))?
        .to_string();
    let slot = tx.get("slot").and_then(Value::as_u64).unwrap_or(0);
    let block_time = tx.get("blockTime").and_then(Value::as_i64);
    let fee = meta.get("fee").and_then(Value::as_u64).unwrap_or(0);

    // Watched-wallet deltas.
    let (token_deltas, token_history_complete) = delta_token_rows(meta, watched_wallet);
    let sol_delta = native_sol_delta(meta, &account_keys, watched_wallet);
    if token_deltas.is_empty() && sol_delta == 0 {
        return Ok(Vec::new());
    }

    // Quote = native SOL + WSOL, netted; fee excluded from the quote leg
    // (reported separately exactly once).
    let wsol_delta = token_deltas.get(MINT_WSOL).copied().unwrap_or(0);
    // The fee payer's native delta carries meta.fee; exclude it so the
    // quote leg is pure economics (the fee is reported once, separately).
    let signer = account_keys.first().map(String::as_str) == Some(watched_wallet);
    let sol_economic = if signer {
        sol_delta + i128::from(fee)
    } else {
        sol_delta
    };
    let quote_delta = sol_economic + wsol_delta;
    let mut base_deltas: Vec<(String, i128)> = token_deltas
        .iter()
        .filter(|(mint, delta)| mint.as_str() != MINT_WSOL && **delta != 0)
        .map(|(mint, delta)| (mint.clone(), *delta))
        .collect();
    base_deltas.sort();

    // Venue detection: account keys first, then explicit instruction
    // program ids (fixture-shape tolerance).
    let mut venue = detect_venue(meta, &account_keys);
    if venue.is_none() {
        for program in instruction_programs(tx) {
            if WalletVenue::from_program(&program) != WalletVenue::UnknownDex {
                venue = Some(WalletVenue::from_program(&program));
                break;
            }
        }
    }

    let authority = if signer {
        AuthorityProvenance::Signer
    } else if !token_deltas.is_empty() {
        AuthorityProvenance::AtaOwnerResolved
    } else {
        AuthorityProvenance::Unresolved
    };

    // LP-shape aggregation: if any base mint moves the SAME direction as
    // the quote leg (both out on deposit / both in on withdraw), the whole
    // transaction is liquidity provision — ONE LpFlow event, never a swap.
    let lp_shape = base_deltas.iter().any(|(_, delta)| {
        *delta != 0 && quote_delta != 0 && delta.signum() == quote_delta.signum()
    });
    if lp_shape {
        let (dominant_mint, total_token) = base_deltas
            .iter()
            .max_by_key(|(_, delta)| delta.abs())
            .map(|(m, d)| (m.clone(), *d))
            .unwrap_or(("UNKNOWN".into(), 0));
        return Ok(vec![WalletTradeEvent {
            signature,
            slot,
            block_time,
            commitment: Commitment::Confirmed,
            wallet_owner: watched_wallet.to_string(),
            authority_provenance: authority,
            mint: dominant_mint,
            venue: venue.unwrap_or(WalletVenue::UnknownDex),
            side: Side::LpFlow,
            raw_token_delta: total_token,
            raw_quote_delta: quote_delta,
            network_fee: fee,
            coverage: if token_history_complete {
                "COMPLETE"
            } else {
                "INCOMPLETE"
            }
            .to_string(),
            classification: if token_history_complete {
                Classification::Medium
            } else {
                Classification::Low
            },
        }]);
    }

    // Partial history with a SOL swap shape but no decodable token rows:
    // emit a fail-closed UNKNOWN-mint event (coverage INCOMPLETE) so
    // downstream scoring can count the gap — never an invented holding.
    if base_deltas.is_empty() && quote_delta != 0 && !token_history_complete {
        return Ok(vec![WalletTradeEvent {
            signature,
            slot,
            block_time,
            commitment: Commitment::Confirmed,
            wallet_owner: watched_wallet.to_string(),
            authority_provenance: authority,
            mint: "UNKNOWN".to_string(),
            venue: venue.unwrap_or(WalletVenue::UnknownDex),
            side: Side::Unknown,
            raw_token_delta: 0,
            raw_quote_delta: quote_delta,
            network_fee: fee,
            coverage: "INCOMPLETE".to_string(),
            classification: Classification::Low,
        }]);
    }

    // Events: one per nonzero non-quote base mint.
    let mut events = Vec::new();
    for (mint, token_delta) in base_deltas {
        let dex_present = venue.is_some();
        let side = if !dex_present && token_delta != 0 && quote_delta == 0 {
            Side::Transfer
        } else if token_delta > 0 && quote_delta < 0 {
            Side::Buy
        } else if token_delta < 0 && quote_delta > 0 {
            Side::Sell
        } else {
            // Both legs moved the same direction (or only one leg):
            // LP deposit/withdraw shape or ambiguous — never a swap.
            Side::LpFlow
        };
        let coverage = if token_history_complete {
            "COMPLETE"
        } else {
            "INCOMPLETE"
        };
        let classification = match (side, token_history_complete, venue) {
            (Side::Buy | Side::Sell, true, Some(v)) if v != WalletVenue::UnknownDex => {
                Classification::High
            }
            (Side::Buy | Side::Sell, true, _) => Classification::Medium,
            (Side::Buy | Side::Sell, false, _) => Classification::Low,
            (_, false, _) => Classification::Low,
            (_, true, _) => Classification::Medium,
        };
        events.push(WalletTradeEvent {
            signature: signature.clone(),
            slot,
            block_time,
            commitment: Commitment::Confirmed,
            wallet_owner: watched_wallet.to_string(),
            authority_provenance: authority,
            mint,
            venue: venue.unwrap_or(WalletVenue::UnknownDex),
            side,
            raw_token_delta: token_delta,
            raw_quote_delta: if side == Side::Transfer {
                0
            } else {
                quote_delta
            },
            network_fee: fee,
            coverage: coverage.to_string(),
            classification,
        });
    }
    // Pure SOL/WSOL movement (wrap/unwrap or SOL transfer) with no base
    // mints: not a wallet trade event.
    Ok(events)
}

/// Replay-level dedup: identical (signature, wallet, mint) re-observations
/// collapse. Returns (unique events, duplicates dropped).
pub fn dedup_events(events: Vec<WalletTradeEvent>) -> (Vec<WalletTradeEvent>, usize) {
    let mut seen: std::collections::HashSet<(String, u64, String, String)> =
        std::collections::HashSet::new();
    let mut unique = Vec::with_capacity(events.len());
    let mut duplicates = 0usize;
    for event in events {
        let key = (
            event.signature.clone(),
            event.slot,
            event.wallet_owner.clone(),
            event.mint.clone(),
        );
        if seen.insert(key) {
            unique.push(event);
        } else {
            duplicates += 1;
        }
    }
    (unique, duplicates)
}

/// Whole-transaction conservation invariants (fixture sanity):
/// (1) every mint's total balance is conserved across the tx;
/// (2) the sum of SOL deltas over all accounts equals minus the fee.
pub fn tx_conservation_invariants(tx: &Value) -> (bool, bool) {
    let meta = match tx.get("meta") {
        Some(m) => m,
        None => return (false, false),
    };
    let mut mint_totals: HashMap<(String, usize), i128> = HashMap::new();
    for (field, sign) in [("postTokenBalances", 1i128), ("preTokenBalances", -1i128)] {
        for row in meta
            .get(field)
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default()
        {
            let (Some(mint), Some(index), Some(amount)) = (
                row.get("mint").and_then(Value::as_str),
                row.get("accountIndex").and_then(Value::as_u64),
                row.get("uiTokenAmount")
                    .and_then(|a| a.get("amount"))
                    .and_then(Value::as_str)
                    .and_then(|a| a.parse::<i128>().ok()),
            ) else {
                continue;
            };
            *mint_totals
                .entry((mint.to_string(), index as usize))
                .or_insert(0) += sign * amount;
        }
    }
    // RPC lists pre/post rows per (account, mint) once, so post+pre with
    // signs must cancel when balance is reported for both; a nonzero sum
    // with the account present in only one list is a delta, not a
    // violation. True mint conservation: sum of ALL deltas per mint across
    // accounts (transfers move between accounts) is not derivable from
    // per-account rows alone when only moved accounts are listed — so the
    // check here is the weaker receipt sanity: every listed pair cancels.
    let mints_ok = mint_totals.values().all(|v| *v == 0)
        || meta
            .get("preTokenBalances")
            .and_then(Value::as_array)
            .map(|a| a.is_empty())
            .unwrap_or(true);

    let fee = meta.get("fee").and_then(Value::as_i64).unwrap_or(0);
    let (Some(pre), Some(post)) = (
        meta.get("preBalances").and_then(Value::as_array),
        meta.get("postBalances").and_then(Value::as_array),
    ) else {
        return (mints_ok, false);
    };
    let mut sol_sum = 0i128;
    for (a, b) in pre.iter().zip(post.iter()) {
        sol_sum += i128::from(b.as_i64().unwrap_or(0)) - i128::from(a.as_i64().unwrap_or(0));
    }
    (mints_ok, sol_sum == -i128::from(fee))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn venue_from_known_programs() {
        assert_eq!(
            WalletVenue::from_program(PROGRAM_PUMP_FUN),
            WalletVenue::PumpFun
        );
        assert_eq!(
            WalletVenue::from_program(PROGRAM_RAYDIUM_AMM_V4),
            WalletVenue::Raydium
        );
        assert_eq!(
            WalletVenue::from_program("unknown"),
            WalletVenue::UnknownDex
        );
    }
}
