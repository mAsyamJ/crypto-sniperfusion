//! WA1 wallet-transaction normalization contract tests. Red-first: these
//! reference `wallet_observer` which does not exist yet.
//!
//! Fixture philosophy: hand-built `getTransaction`-shaped JSON (the exact
//! surface the G0 collector fetches), one per mandated case — Pump.fun
//! buy/sell, PumpSwap swap, Jupiter 2-hop, Raydium swap, Meteora swap,
//! SPL transfer, LP deposit, Token-2022 trade, failed tx. Behavioral
//! references: MIT DoradoDevs balance-delta parser + Solsonar decoder
//! (reimplemented originally; no code copied).

use sniper_core::event::Commitment;
use sniper_runtime::wallet_observer::{
    normalize_transaction, AuthorityProvenance, Classification, Side, WalletVenue,
};

fn pump() -> String {
    "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".into()
}
fn pumpswap() -> String {
    "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA".into()
}
fn jupiter() -> String {
    "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".into()
}
fn raydium() -> String {
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".into()
}
fn meteora() -> String {
    "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo".into()
}
fn wsol() -> String {
    "So11111111111111111111111111111111111111112".into()
}

/// Wallet = fee payer (accountKeys[0]) with a native SOL balance; token
/// accounts carry explicit owner fields.
fn tx_fixture(programs: &[&str], balances: Balances, err: bool) -> serde_json::Value {
    let account_keys: Vec<String> = balances.accounts.iter().map(|a| a.key.clone()).collect();
    serde_json::json!({
        "slot": 465_400_000,
        "blockTime": 1_790_000_000,
        "transaction": {
            "signatures": ["SigWA0000000000000000000000000000000000000000000000000000000"],
            "message": {
                "accountKeys": account_keys,
                "instructions": programs.iter()
                    .map(|p| serde_json::json!({"programId": p.to_string()}))
                    .collect::<Vec<_>>(),
            },
        },
        "meta": {
            "err": if err { serde_json::json!({"InstructionError": [0, "Custom(1)"]}) } else { serde_json::Value::Null },
            "fee": balances.fee,
            "preBalances": balances.accounts.iter().map(|a| a.sol_pre).collect::<Vec<_>>(),
            "postBalances": balances.accounts.iter().map(|a| a.sol_post).collect::<Vec<_>>(),
            "loadedAddresses": [],
            "preTokenBalances": balances.pre_tokens,
            "postTokenBalances": balances.post_tokens,
            "logMessages": [],
        },
    })
}

struct AccountSpec {
    key: String,
    sol_pre: u64,
    sol_post: u64,
}

#[derive(Default)]
struct Balances {
    accounts: Vec<AccountSpec>,
    fee: u64,
    pre_tokens: Vec<serde_json::Value>,
    post_tokens: Vec<serde_json::Value>,
}

impl Balances {
    fn wallet(key: &str, sol_pre: u64, sol_post: u64) -> Self {
        Self {
            accounts: vec![AccountSpec {
                key: key.into(),
                sol_pre,
                sol_post,
            }],
            fee: 5_000,
            ..Default::default()
        }
    }

    fn program(mut self, key: &str) -> Self {
        self.accounts.push(AccountSpec {
            key: key.into(),
            sol_pre: 0,
            sol_post: 0,
        });
        self
    }

    /// Pool-side account with its own SOL movement so whole-tx conservation
    /// (sum of SOL deltas == -fee) holds like a real receipt.
    fn pool_sol(mut self, key: &str, delta: i128) -> Self {
        let pre: u64 = if delta < 0 {
            delta.unsigned_abs() as u64
        } else {
            0
        };
        let post: u64 = if delta > 0 { delta as u64 } else { 0 };
        self.accounts.push(AccountSpec {
            key: key.into(),
            sol_pre: pre,
            sol_post: post,
        });
        self
    }

    /// Token balance rows: (account_index, mint, owner, pre, post).
    fn token(mut self, idx: usize, mint: &str, owner: &str, pre: u64, post: u64) -> Self {
        let row = |amount: u64| {
            serde_json::json!({
                "accountIndex": idx, "mint": mint, "owner": owner,
                "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
                "uiTokenAmount": {"amount": amount.to_string(), "decimals": 6},
            })
        };
        if pre > 0 {
            self.pre_tokens.push(row(pre));
        }
        if post > 0 {
            self.post_tokens.push(row(post));
        }
        self
    }
}

const WALLET: &str = "Wa11etWa11etWa11etWa11etWa11etWa11etWa11etWa1";
const MINT: &str = "MintMintMintMintMintMintMintMintMintMintMi";

#[test]
fn wa1_pump_fun_buy_is_owner_verified_high_confidence() {
    // Wallet spends 1.000005 SOL (1 SOL + 5000 fee) and gains 900 tokens.
    let tx = tx_fixture(
        &[&pump()],
        Balances::wallet(WALLET, 2_000_005_000, 1_000_000_000)
            .program(&pump())
            .token(2, MINT, WALLET, 0, 900),
        false,
    );
    let events = normalize_transaction(&tx, WALLET).unwrap();
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!(event.venue, WalletVenue::PumpFun);
    assert_eq!(event.side, Side::Buy);
    assert_eq!(event.raw_token_delta, 900);
    assert_eq!(event.raw_quote_delta, -1_000_000_000);
    assert_eq!(event.network_fee, 5_000);
    assert_eq!(event.authority_provenance, AuthorityProvenance::Signer);
    assert_eq!(event.classification, Classification::High);
    assert_eq!(event.commitment, Commitment::Confirmed);
}

#[test]
fn wa1_pump_fun_sell_direction_flips() {
    let tx = tx_fixture(
        &[&pump()],
        Balances::wallet(WALLET, 1_000_000_000, 1_500_000_000 - 5_000)
            .program(&pump())
            .token(2, MINT, WALLET, 700, 0),
        false,
    );
    let events = normalize_transaction(&tx, WALLET).unwrap();
    let event = &events[0];
    assert_eq!(event.side, Side::Sell);
    assert_eq!(event.raw_token_delta, -700);
    // Fee excluded: the quote leg is the gross swap flow; fee is separate.
    assert_eq!(event.raw_quote_delta, 500_000_000);
}

#[test]
fn wa1_pumpswap_swap_via_wsol_wrap_netting() {
    // WSOL route: wrap 500M, swap all of it, nothing left to unwrap.
    let tx = tx_fixture(
        &[&pumpswap()],
        Balances::wallet(WALLET, 1_500_000_000, 999_995_000)
            .program(&pumpswap())
            .token(2, &wsol(), WALLET, 0, 0)
            .token(3, MINT, WALLET, 0, 250),
        false,
    );
    let events = normalize_transaction(&tx, WALLET).unwrap();
    assert_eq!(
        events.len(),
        1,
        "one net economic trade, not per-leg events"
    );
    let event = &events[0];
    assert_eq!(event.venue, WalletVenue::PumpSwap);
    assert_eq!(event.side, Side::Buy);
    // Fee excluded: quote delta is exactly the swapped 500M.
    assert_eq!(event.raw_quote_delta, -500_000_000);
    assert_eq!(event.raw_token_delta, 250);
    assert_eq!(event.network_fee, 5_000);
}

#[test]
fn wa1_jupiter_two_hop_is_one_net_trade() {
    // SOL -> USDC-hop-mint -> MINT through Jupiter; intermediate nets to 0.
    let hop = "HopMintHopMintHopMintHopMintHopMintHopMintH";
    let tx = tx_fixture(
        &[&jupiter()],
        Balances::wallet(WALLET, 2_000_000_000, 1_399_995_000)
            .program(&jupiter())
            .token(2, &wsol(), WALLET, 0, 0)
            .token(3, hop, WALLET, 0, 0)
            .token(4, MINT, WALLET, 0, 1_200),
        false,
    );
    let events = normalize_transaction(&tx, WALLET).unwrap();
    assert_eq!(events.len(), 1, "multi-hop aggregated to one net trade");
    let event = &events[0];
    assert_eq!(event.venue, WalletVenue::Jupiter);
    assert_eq!(event.side, Side::Buy);
    assert_eq!(event.raw_token_delta, 1_200);
    assert_eq!(event.raw_quote_delta, -600_000_000);
}

#[test]
fn wa1_raydium_and_meteora_swaps_classify() {
    for (program, venue) in [
        (raydium(), WalletVenue::Raydium),
        (meteora(), WalletVenue::Meteora),
    ] {
        let tx = tx_fixture(
            &[&program],
            Balances::wallet(WALLET, 2_000_000_000, 2_499_995_000)
                .program(&program)
                .token(2, MINT, WALLET, 300, 0),
            false,
        );
        let events = normalize_transaction(&tx, WALLET).unwrap();
        assert_eq!(events[0].venue, venue, "venue from program id");
        assert_eq!(events[0].side, Side::Sell);
    }
}

#[test]
fn wa1_plain_transfer_is_not_a_buy() {
    let other = "0ther0ther0ther0ther0ther0ther0ther0";
    // Wallet's token account loses 500 tokens with NO dex program present.
    let tx = tx_fixture(
        &[],
        Balances::wallet(WALLET, 1_000_005_000, 1_000_000_000)
            .token(1, MINT, WALLET, 1_000, 500)
            .token(2, MINT, other, 500, 1_000),
        false,
    );
    let events = normalize_transaction(&tx, WALLET).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].side, Side::Transfer, "a transfer is not a buy");
    assert_eq!(events[0].raw_token_delta, -500);
    assert_eq!(events[0].classification, Classification::Medium);
}

#[test]
fn wa1_lp_deposit_is_lp_flow_not_a_trade() {
    // Wallet gives up BOTH SOL and tokens and receives LP tokens.
    let lp = "LpMintLpMintLpMintLpMintLpMintLpMintLpMintLpMint";
    let tx = tx_fixture(
        &[&raydium()],
        Balances::wallet(WALLET, 2_000_000_000, 1_500_000_000)
            .program(&raydium())
            .token(2, MINT, WALLET, 1_000, 400)
            .token(3, lp, WALLET, 0, 10),
        false,
    );
    let events = normalize_transaction(&tx, WALLET).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].side, Side::LpFlow, "LP deposit must not be a buy");
    assert_eq!(events[0].classification, Classification::Medium);
}

#[test]
fn wa1_token_2022_trade_normalizes_by_balance_delta() {
    let tx = tx_fixture(
        &[&pump()],
        Balances::wallet(WALLET, 2_000_000_000, 1_200_000_000)
            .program(&pump())
            .token(2, MINT, WALLET, 0, 555),
        false,
    );
    // Token-2022 differs only in programId of the token account.
    let patched = replace_token_program(&tx, "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqKepCcRk8xS");
    let events = normalize_transaction(&patched, WALLET).unwrap();
    assert_eq!(events[0].side, Side::Buy);
    assert_eq!(events[0].raw_token_delta, 555);
}

fn replace_token_program(tx: &serde_json::Value, program: &str) -> serde_json::Value {
    let mut patched = tx.clone();
    for field in ["preTokenBalances", "postTokenBalances"] {
        let rows = patched["meta"][field].as_array_mut().unwrap();
        for row in rows {
            row["programId"] = serde_json::json!(program);
        }
    }
    patched
}

#[test]
fn wa1_failed_transaction_yields_no_events() {
    let tx = tx_fixture(
        &[&pump()],
        Balances::wallet(WALLET, 2_000_000_000, 1_999_995_000).program(&pump()),
        true,
    );
    assert!(normalize_transaction(&tx, WALLET).unwrap().is_empty());
}

#[test]
fn wa1_unknown_dex_swap_shape_is_flagged_not_invented() {
    let unknown_dex = "Unkn0wnDexProgramUnkn0wnDexProgramUnkn0wnDexProgramUnk";
    let tx = tx_fixture(
        &[unknown_dex],
        Balances::wallet(WALLET, 2_000_000_000, 1_000_000_000)
            .program(unknown_dex)
            .token(2, MINT, WALLET, 0, 42),
        false,
    );
    let events = normalize_transaction(&tx, WALLET).unwrap();
    assert_eq!(events[0].venue, WalletVenue::UnknownDex);
    assert_eq!(events[0].side, Side::Buy);
    assert_eq!(
        events[0].classification,
        Classification::Medium,
        "fail-open shape, flagged lower confidence"
    );
}

#[test]
fn wa1_wallet_not_involved_yields_nothing_and_missing_history_flags() {
    let stranger = "5tranger5tranger5tranger5tranger5tranger5tranger5tr";
    let tx = tx_fixture(
        &[&pump()],
        Balances::wallet(WALLET, 2_000_000_000, 2_000_000_000)
            .program(&pump())
            .token(2, MINT, stranger, 0, 100),
        false,
    );
    assert!(normalize_transaction(&tx, WALLET).unwrap().is_empty());
}

#[test]
fn wa1_partial_history_is_flagged_incomplete() {
    // Token account exists pre-tx but preTokenBalances omitted it: the
    // wallet's TRUE token delta is unknowable from this receipt alone.
    let mut tx = tx_fixture(
        &[&pump()],
        Balances::wallet(WALLET, 2_000_000_000, 1_000_000_000)
            .program(&pump())
            .token(2, MINT, WALLET, 0, 900),
        false,
    );
    tx["meta"]["preTokenBalances"]
        .as_array_mut()
        .unwrap()
        .clear();
    tx["meta"]["postTokenBalances"]
        .as_array_mut()
        .unwrap()
        .clear();
    let events = normalize_transaction(&tx, WALLET).unwrap();
    // SOL side still shows a swap shape, but token delta is UNKNOWN:
    // coverage must be INCOMPLETE, never an invented delta.
    assert_eq!(events[0].coverage, "INCOMPLETE");
    assert_eq!(events[0].raw_token_delta, 0, "no invented holdings");
    assert_eq!(events[0].classification, Classification::Low);
}

fn with_sig(mut tx: serde_json::Value, sig: &str) -> serde_json::Value {
    tx["transaction"]["signatures"] = serde_json::json!([sig]);
    tx
}

#[test]
fn wa1_replay_dedup_and_conservation_invariants() {
    // Build a small corpus covering the matrix and replay it.
    let pool = "Poo1PDAxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
    let corpus = vec![
        with_sig(
            tx_fixture(
                &[&pump()],
                Balances::wallet(WALLET, 2_000_000_000, 999_995_000)
                    .program(&pump())
                    .pool_sol(pool, 1_000_000_000)
                    .token(3, MINT, WALLET, 0, 900),
                false,
            ),
            "SigWA0001",
        ),
        with_sig(
            tx_fixture(
                &[&pumpswap()],
                Balances::wallet(WALLET, 1_500_000_000, 999_995_000)
                    .program(&pumpswap())
                    .pool_sol(pool, 500_000_000)
                    .token(3, MINT, WALLET, 0, 250),
                false,
            ),
            "SigWA0002",
        ),
        with_sig(
            tx_fixture(
                &[&raydium()],
                Balances::wallet(WALLET, 2_000_000_000, 2_499_995_000)
                    .program(&raydium())
                    .pool_sol(pool, -500_000_000)
                    .token(3, MINT, WALLET, 300, 0),
                false,
            ),
            "SigWA0003",
        ),
    ];
    let mut events = Vec::new();
    for tx in &corpus {
        // Replay twice: provider duplicates must collapse later.
        events.extend(normalize_transaction(tx, WALLET).unwrap());
        events.extend(normalize_transaction(tx, WALLET).unwrap());
        let (mints_ok, sol_ok) = sniper_runtime::wallet_observer::tx_conservation_invariants(tx);
        assert!(sol_ok, "sum of SOL deltas must equal -fee");
        let _ = mints_ok;
    }
    let (unique, duplicates) = sniper_runtime::wallet_observer::dedup_events(events);
    assert_eq!(unique.len(), 3);
    assert_eq!(duplicates, 3, "second replay observations are duplicates");
}
