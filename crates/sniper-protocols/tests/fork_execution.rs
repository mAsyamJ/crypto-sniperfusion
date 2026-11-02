//! REAL local program execution against a Surfpool mainnet-state fork.
//!
//! Safety contract (mirrors harness/scripts/start_fork.sh):
//! - Skipped unless `SNF_FORK_RPC` is set AND points at a loopback host.
//! - Refuses to run when `LIVE_TRADING_ENABLED` is anything but unset/false.
//! - Signs ONLY with a throwaway `Keypair::new()` funded by the fork's local
//!   faucet; there is no code path here that can reach a non-loopback RPC.
//!
//! The non-ignored test below pins our PDA derivations to a REAL mainnet
//! production transaction (captured 2026-11-02); the `#[ignore]`d tests
//! execute our builders against the actual deployed pump.fun program ELF
//! loaded by the fork. Run them with:
//!
//! ```text
//! SNF_SOURCE_RPC=https://api.mainnet-beta.solana.com \
//!   bash harness/scripts/start_fork.sh &
//! SNF_FORK_RPC=http://127.0.0.1:18899 \
//!   cargo test -p sniper-protocols --test fork_execution -- --ignored --nocapture
//! ```

use sniper_protocols::builders::{buy_v3, global_volume_accumulator, sell_v3, CurveTradeContext};
use sniper_protocols::events::{PumpEvent, TradeEvent};
use sniper_protocols::logs;
use sniper_protocols::pda;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::hash::Hash;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;

// ---- Ground truth captured from a live mainnet pump.fun buy (2026-11-02).
// Tx 2yCcufczQBbvccL9Mf3M3VYDimvPXMPtA38BpGvFqosTLxDw8FWTypFsg8cqVqaXzfmz2La7dkXXQ4te45JmjoKD
// (router -> 6EF8 "Instruction: Buy" CPI, Token-2022 mint).
const LIVE_MINT: &str = "Ah7ghbBJVfjKWDCyGgQmDiPiqWP2XDZQ3nj1Axzupump";
const LIVE_BONDING_CURVE: &str = "7sTXzyg7pnTk5wm28bcWgWc3sQubMJojgv4YuvqtnKFY";
const LIVE_ASSOC_BONDING_CURVE: &str = "6JSaxGfusAaz37eDxT9PJJLhAGGNgPHZBHKsr1enEYG9";
const LIVE_USER: &str = "BwWK17cbHxwWBKZkUYvzxLcNQ1YVyaFezduWbtm2de6s";
const LIVE_USER_ATA: &str = "8FTzgxAREYBKF2shiJiq1B1LpHMKQhowDyji6QwUvwHK";
const LIVE_USER_VOLUME_ACC: &str = "FGFrX2q1iAjyAojjeyFDxXqdmvegjPpSWsrPmrJjeQ2f";
const LIVE_GLOBAL_VOLUME_ACC: &str = "Hq2wp8uJ9jCPsYgNHex8RtqdvMPfVGoYwjvF1ATiwn2Y";

/// Our PDA derivations must reproduce the accounts a real production buy
/// touched. Pure offline check — runs in CI.
#[test]
fn pda_derivations_match_live_mainnet_production_tx() {
    let mint = pda::parse(LIVE_MINT);
    let bc = pda::bonding_curve(&mint);
    assert_eq!(bc, pda::parse(LIVE_BONDING_CURVE), "bonding_curve PDA");
    assert_eq!(
        pda::associated_bonding_curve(&mint, &bc, &pda::token_2022_program()),
        pda::parse(LIVE_ASSOC_BONDING_CURVE),
        "associated bonding curve (Token-2022 base)"
    );
    let user = pda::parse(LIVE_USER);
    assert_eq!(
        pda::spl_associated(&user, &pda::token_2022_program(), &mint),
        pda::parse(LIVE_USER_ATA),
        "user base ATA"
    );
    assert_eq!(
        pda::user_volume_accumulator(&user),
        pda::parse(LIVE_USER_VOLUME_ACC),
        "user volume accumulator PDA"
    );
    assert_eq!(
        global_volume_accumulator(),
        pda::parse(LIVE_GLOBAL_VOLUME_ACC),
        "global volume accumulator PDA"
    );
}

// ---- opt-in fork execution below -------------------------------------------------

struct Fork {
    rpc: RpcClient,
    user: Keypair,
}

fn fork_or_panic() -> Fork {
    let url = std::env::var("SNF_FORK_RPC").unwrap_or_default();
    assert!(
        !url.is_empty(),
        "SNF_FORK_RPC not set — start the fork via harness/scripts/start_fork.sh"
    );
    let host = url.split("//").nth(1).unwrap_or_default();
    let hostname = host.split(':').next().unwrap_or_default();
    assert!(
        hostname == "127.0.0.1" || hostname == "localhost",
        "refusing non-loopback fork RPC {url}"
    );
    assert!(
        std::env::var("LIVE_TRADING_ENABLED").unwrap_or_default() != "true",
        "live trading must stay disabled"
    );
    Fork {
        rpc: RpcClient::new_with_commitment(url, CommitmentConfig::confirmed()),
        user: Keypair::new(),
    }
}

fn fund(fork: &Fork, lamports: u64) {
    let sig = fork
        .rpc
        .request_airdrop(&fork.user.pubkey(), lamports)
        .expect("local faucet airdrop");
    fork.rpc
        .confirm_transaction(&sig)
        .expect("airdrop confirms on the fork");
}

/// `AssociatedTokenProgram.CreateIdempotent` (data `[1]`). The deployed
/// pump.fun buy_v3 REQUIRES a pre-existing `associated_base_user` — the
/// program itself does not create it (verified on the fork: anchor error
/// 3012 AccountNotInitialized otherwise).
fn create_ata_idempotent(
    payer: &Pubkey,
    owner: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    let ata = pda::spl_associated(owner, token_program, mint);
    Instruction {
        program_id: pda::associated_token_program(),
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(ata, false),
            AccountMeta::new_readonly(*owner, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(pda::system_program(), false),
            AccountMeta::new_readonly(*token_program, false),
            AccountMeta::new_readonly(pda::associated_token_program(), false),
        ],
        data: vec![1],
    }
}

fn blockhash(fork: &Fork) -> Hash {
    fork.rpc.get_latest_blockhash().expect("blockhash")
}

fn submit(fork: &Fork, ixs: Vec<Instruction>) -> solana_sdk::signature::Signature {
    let mut all = vec![ComputeBudgetInstruction::set_compute_unit_limit(400_000)];
    all.extend(ixs);
    let msg = Message::new(&all, Some(&fork.user.pubkey()));
    let tx = Transaction::new(&[&fork.user], msg, blockhash(fork));
    // Simulate first so failures come back with logs even if the fork would
    // also accept the send.
    let sim = fork.rpc.simulate_transaction(&tx).expect("simulation runs");
    if let Some(errs) = sim.value.logs.clone() {
        for l in errs {
            eprintln!("SIM| {l}");
        }
    }
    assert!(
        sim.value.err.is_none(),
        "simulation failed: {:?}",
        sim.value.err
    );
    fork.rpc
        .send_and_confirm_transaction(&tx)
        .expect("transaction confirms on the fork")
}

/// Minimal JSON view of a confirmed transaction (what the fork returns).
struct TxDelta {
    logs: Vec<String>,
    pre_balances: Vec<u64>,
    post_balances: Vec<u64>,
    token_deltas: Vec<(String, String, i128)>, // (mint, owner, delta)
    cu_consumed: Option<u64>,
    fee: u64,
}

fn fetch_delta(fork: &Fork, sig: &solana_sdk::signature::Signature) -> TxDelta {
    let json: serde_json::Value = fork
        .rpc
        .send(
            solana_client::rpc_request::RpcRequest::GetTransaction,
            serde_json::json!([
                sig.to_string(),
                {
                    "encoding": "json",
                    "maxSupportedTransactionVersion": 0,
                    "commitment": "confirmed"
                }
            ]),
        )
        .expect("getTransaction");
    // RpcClient::send returns the unwrapped JSON-RPC `result` object.
    assert!(
        json["meta"].is_object(),
        "confirmed tx not yet queryable: {json}"
    );
    let meta = &json["meta"];
    // All token deltas; accounts absent from preTokenBalances (freshly
    // created ATAs) start from zero.
    let mut token_deltas: Vec<(String, String, i128)> = Vec::new(); // (mint, owner, delta)
    let pre_tok = &meta["preTokenBalances"];
    let post_tok = &meta["postTokenBalances"];
    if let Some(post) = post_tok.as_array() {
        for p in post {
            let idx = p["accountIndex"].as_u64().unwrap_or(u64::MAX);
            let pre_amt: i128 = pre_tok
                .as_array()
                .and_then(|pre| pre.iter().find(|b| b["accountIndex"].as_u64() == Some(idx)))
                .and_then(|b| b["uiTokenAmount"]["amount"].as_str())
                .and_then(|s| s.parse::<i128>().ok())
                .unwrap_or(0);
            let post_amt: i128 = p["uiTokenAmount"]["amount"]
                .as_str()
                .and_then(|s| s.parse::<i128>().ok())
                .unwrap_or(0);
            let d = post_amt - pre_amt;
            if d != 0 {
                token_deltas.push((
                    p["mint"].as_str().unwrap_or_default().to_string(),
                    p["owner"].as_str().unwrap_or_default().to_string(),
                    d,
                ));
            }
        }
    }
    TxDelta {
        logs: meta["logMessages"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|l| l.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        pre_balances: meta["preBalances"]
            .as_array()
            .map(|a| a.iter().filter_map(|b| b.as_u64()).collect())
            .unwrap_or_default(),
        post_balances: meta["postBalances"]
            .as_array()
            .map(|a| a.iter().filter_map(|b| b.as_u64()).collect())
            .unwrap_or_default(),
        token_deltas,
        cu_consumed: meta["computeUnitsConsumed"].as_u64(),
        fee: meta["fee"].as_u64().unwrap_or(0),
    }
}

fn decode_trade_event(delta: &TxDelta) -> TradeEvent {
    let pump = pda::pump_program();
    let log_refs: Vec<&str> = delta.logs.iter().map(String::as_str).collect();
    let frames = logs::extract_program_data(&log_refs, &pump);
    for f in &frames {
        let hex_head: String = f.payload[..f.payload.len().min(105)]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        eprintln!(
            "FRAME from {} len={} hex_head={}",
            f.emitter,
            f.payload.len(),
            hex_head
        );
    }
    let ev = frames
        .iter()
        .filter_map(|f| match PumpEvent::decode(&f.payload) {
            Some(PumpEvent::Trade(t)) => Some(t),
            _ => None,
        })
        .next()
        .expect("a TradeEvent frame emitted by the pump program");
    eprintln!(
        "TradeEvent: mint={} is_buy={} token_amount={} sol_amount={} quote_mint={} quote_amount={} fee={} buyback_fee={} cu={:?} txfee={}",
        ev.mint,
        ev.is_buy,
        ev.token_amount,
        ev.sol_amount,
        ev.quote_mint,
        ev.quote_amount,
        ev.fee,
        ev.buyback_fee,
        delta.cu_consumed,
        delta.fee
    );
    ev
}

/// ONE real local pump.fun buy_v3 against the deployed program ELF, with a
/// throwaway wallet. Verifies token + SOL deltas and decodes the emitted
/// TradeEvent with OUR decoder.
#[test]
#[ignore = "opt-in: needs loopback Surfpool fork (see module docs)"]
fn fork_buy_v3_executes_against_real_program() {
    let fork = fork_or_panic();
    let mint = pda::parse(&std::env::var("SNF_FORK_MINT").unwrap_or_else(|_| LIVE_MINT.into()));

    // The curve account must exist on the fork (JIT-fetched from mainnet).
    let bc = pda::bonding_curve(&mint);
    let bc_account = fork.rpc.get_account(&bc).expect("bonding curve account");
    eprintln!(
        "curve account: len={} lamports={} owner={}",
        bc_account.data.len(),
        bc_account.lamports,
        bc_account.owner
    );

    fund(&fork, 10_000_000_000);
    let ctx = CurveTradeContext::sol_paired(fork.user.pubkey(), mint);
    let amount: u64 = 100_000; // tokens out
    let max_sol_cost: u64 = 200_000_000; // 0.2 SOL cap

    let ata_create = create_ata_idempotent(
        &fork.user.pubkey(),
        &fork.user.pubkey(),
        &mint,
        &pda::token_2022_program(),
    );
    let ix = buy_v3(&ctx, amount, max_sol_cost, 0, false);
    let sig = submit(&fork, vec![ata_create, ix]);
    let d = fetch_delta(&fork, &sig);

    let ev = decode_trade_event(&d);
    assert_eq!(ev.mint, mint, "event mint");
    assert!(ev.is_buy, "buy event");
    assert_eq!(
        ev.user,
        fork.user.pubkey(),
        "event user is our ephemeral wallet"
    );
    assert_eq!(
        ev.token_amount, amount,
        "exact fill expected with partial_fill=false"
    );

    // Token delta on OUR ATA must equal the event's token_amount.
    let user_delta = d
        .token_deltas
        .iter()
        .find(|(_, owner, _)| owner == &fork.user.pubkey().to_string())
        .expect("a token delta on the user's ATA");
    assert_eq!(user_delta.0, mint.to_string());
    assert_eq!(
        user_delta.2, amount as i128,
        "user ATA delta == event token_amount"
    );
    // The curve vault must have sent exactly that many tokens out.
    let curve_delta = d
        .token_deltas
        .iter()
        .find(|(_, owner, _)| owner == &bc.to_string())
        .expect("a token delta on the bonding-curve vault");
    assert_eq!(
        curve_delta.2,
        -(amount as i128),
        "curve vault delta mirrors the buy"
    );

    // SOL accounting from balances (event sol_amount semantics in the
    // quote-era IDL need clarification — see M3c evidence; we assert on
    // chain state, not the field): rent for the fresh T22 ATA (1_513_920)
    // plus network fee plus the SOL leg actually paid to the curve.
    let sol_spent = d.pre_balances[0] - d.post_balances[0];
    let rent = 1_513_920u64; // Token-2022 account with immutable owner, 6-decimals mint
    eprintln!(
        "sol spent={sol_spent} (fee={} rent={rent}) event.sol_amount={}",
        d.fee, ev.sol_amount
    );
    assert!(
        sol_spent > d.fee + rent,
        "more than fee+rent went to the curve"
    );
    assert!(
        sol_spent < 10_000_000,
        "buy cost sanity bound (< 0.01 SOL): {sol_spent}"
    );
}

/// Round trip: sell back what the buy delivered; positions must flatten.
#[test]
#[ignore = "opt-in: needs loopback Surfpool fork (see module docs)"]
fn fork_sell_v3_round_trip_flattens_position() {
    let fork = fork_or_panic();
    let mint = pda::parse(&std::env::var("SNF_FORK_MINT").unwrap_or_else(|_| LIVE_MINT.into()));
    fund(&fork, 10_000_000_000);
    let ctx = CurveTradeContext::sol_paired(fork.user.pubkey(), mint);

    let buy_amount: u64 = 100_000;
    let ata_create = create_ata_idempotent(
        &fork.user.pubkey(),
        &fork.user.pubkey(),
        &mint,
        &pda::token_2022_program(),
    );
    let sig = submit(
        &fork,
        vec![ata_create, buy_v3(&ctx, buy_amount, 200_000_000, 0, false)],
    );
    let buy_ev = decode_trade_event(&fetch_delta(&fork, &sig));
    assert_eq!(buy_ev.token_amount, buy_amount);

    let sig = submit(&fork, vec![sell_v3(&ctx, buy_amount, 0, 0)]);
    let d = fetch_delta(&fork, &sig);
    let ev = decode_trade_event(&d);
    assert!(!ev.is_buy, "sell event");
    assert_eq!(ev.token_amount, buy_amount, "sold exactly the position");

    let user_delta = d
        .token_deltas
        .iter()
        .find(|(_, owner, _)| owner == &fork.user.pubkey().to_string())
        .expect("token delta on sell");
    assert_eq!(user_delta.0, mint.to_string());
    assert_eq!(
        user_delta.2,
        -(buy_amount as i128),
        "ATA delta is negative-of-position"
    );

    // Signed net change: the sell credits SOL and debits the network fee
    // in the same balance; event sol_amount uses the quote-era sentinel
    // (see M3c evidence), so assert on chain state with sanity bounds.
    let sol_net: i128 = d.post_balances[0] as i128 - d.pre_balances[0] as i128;
    eprintln!(
        "sell net SOL change={sol_net} (tx fee={} event sol_amount={})",
        d.fee, ev.sol_amount
    );
    // Deployed-program economics (observed on the fork, 2026-11-02): trade
    // SOL legs are routed to the per-user volume-accumulator PDA, not paid
    // straight to the wallet — the wallet's only sell-leg loss is the
    // network fee. Proceeds are later claimable (close_user_volume_
    // accumulator / claim_cashback family). M3c evidence documents this.
    assert!(
        sol_net >= -(d.fee as i128),
        "sell must not cost the wallet more than the network fee: {sol_net}"
    );
}

/// Negative control: swapped accounts must be rejected — proof the program
/// really validates the metas we send (execution is genuine, not a stub).
#[test]
#[ignore = "opt-in: needs loopback Surfpool fork (see module docs)"]
fn fork_rejects_swapped_accounts() {
    let fork = fork_or_panic();
    let mint = pda::parse(LIVE_MINT);
    fund(&fork, 10_000_000_000);
    let ctx = CurveTradeContext::sol_paired(fork.user.pubkey(), mint);

    let mut ix = buy_v3(&ctx, 100_000, 200_000_000, 0, false);
    ix.accounts.swap(0, 1); // global <-> base_mint
    let msg = Message::new(
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(400_000),
            ix,
        ],
        Some(&fork.user.pubkey()),
    );
    let tx = Transaction::new(&[&fork.user], msg, blockhash(&fork));
    let sim = fork.rpc.simulate_transaction(&tx).expect("simulation");
    assert!(
        sim.value.err.is_some(),
        "swapped accounts must fail, got success"
    );
    eprintln!("swapped-account rejection: {:?}", sim.value.err);
    if let Some(ls) = sim.value.logs {
        for l in ls.iter().take(6) {
            eprintln!("NEG| {l}");
        }
    }
}
