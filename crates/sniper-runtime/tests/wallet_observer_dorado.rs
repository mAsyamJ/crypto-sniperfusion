//! Dorado differential cross-checks for the WA1 wallet normalizer.
//!
//! DoradoDevs/solana-wallet-pnl-tracker (MIT, upstream/dorado @ 1aaeda86)
//! parses wallet trades by NET BALANCE DELTAS per mint (pre-map/post-map
//! diff) rather than instruction decoding, "regardless of inner
//! instruction layout". These tests drive OUR Rust normalizer with
//! fixtures shaped by their documented semantics and assert agreement.

use sniper_runtime::wallet_observer::{normalize_transaction, Side};

fn wallet() -> String {
    "Wa11etWa11etWa11etWa11etWa11etWa11etWa11etWa".into()
}

fn tx(
    pre_tokens: Vec<serde_json::Value>,
    post_tokens: Vec<serde_json::Value>,
    pre_sol: u64,
    post_sol: u64,
    fee: u64,
) -> serde_json::Value {
    let w = wallet();
    serde_json::json!({
        "slot": 700,
        "transaction": {
            "signatures": ["DoradoDiff00000000000000000000000000000000000000000000000000"],
            "message": {"accountKeys": [w,
                "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"],
                "instructions": []},
        },
        "meta": {
            "err": null, "fee": fee,
            "preBalances": [pre_sol, 0], "postBalances": [post_sol, 0],
            "loadedAddresses": [],
            "preTokenBalances": pre_tokens,
            "postTokenBalances": post_tokens,
            "logMessages": [],
        },
    })
}

fn row(idx: u64, mint: &str, owner: &str, amount: u64) -> serde_json::Value {
    serde_json::json!({
        "accountIndex": idx, "mint": mint, "owner": owner,
        "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
        "uiTokenAmount": {"amount": amount.to_string(), "decimals": 6},
    })
}

const MINT: &str = "MintMintMintMintMintMintMintMintMintMintMintMi";

#[test]
fn dorado_semantics_consumed_mint_present_pre_absent_post_is_negative_pre() {
    // Dorado: fully-consumed mint present pre / absent post => delta = -pre
    // (postBalance treated as 0), NOT "unknown". Our normalizer agrees.
    let pre = vec![row(2, MINT, &wallet(), 1_000)];
    let post: Vec<serde_json::Value> = Vec::new();
    let events = normalize_transaction(
        &tx(pre, post, 2_000_000_000, 2_499_995_000, 5_000),
        &wallet(),
    )
    .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].side, Side::Sell);
    assert_eq!(events[0].raw_token_delta, -1_000);
    assert_eq!(events[0].raw_quote_delta, 500_000_000); // fee excluded
}

#[test]
fn dorado_semantics_per_mint_pre_post_map_diffing() {
    // Same mint in two accounts: delta is the SUM of per-account diffs.
    let pre = vec![row(2, MINT, &wallet(), 400), row(3, MINT, &wallet(), 300)];
    let post = vec![row(2, MINT, &wallet(), 100), row(3, MINT, &wallet(), 500)];
    let events = normalize_transaction(
        &tx(pre, post, 2_000_000_000, 1_999_995_000, 5_000),
        &wallet(),
    )
    .unwrap();
    assert_eq!(events.len(), 1);
    // Per-mint map diff: (100-400) + (500-300) = -100 tokens. The only SOL
    // movement is the fee, so the economic quote delta is 0 -> a
    // token-only movement Dorado would also classify as non-swap.
    assert_eq!(events[0].raw_token_delta, -100);
    // Tokens OUT with fee-only SOL movement: same-direction shape — our
    // classifier conservatively refuses to call it a swap (LpFlow), while
    // Dorado's raw delta view simply reports both deltas. The DELTAS agree
    // cross-provider; the side label is our stricter policy.
    assert_eq!(events[0].side, Side::LpFlow);
}

#[test]
fn dorado_semantics_sol_delta_is_a_quote_leg_with_fee_separate() {
    // Dorado adds lamport pre/post diff as a SOL_MINT delta (fees/rent
    // included). WE separate: economic quote delta + network_fee reported
    // once — the PF1 invariant. Cross-check both views agree in total.
    let pre = vec![row(2, MINT, &wallet(), 0)];
    let post = vec![row(2, MINT, &wallet(), 250)];
    let t = tx(pre, post, 1_500_000_000, 999_995_000, 5_000);
    let events = normalize_transaction(&t, &wallet()).unwrap();
    let e = &events[0];
    assert_eq!(e.side, Side::Buy);
    // Raw inclusive SOL delta (Dorado's view) = post - pre = -500_005_000.
    // Our fee-excluded quote = that + fee = -500_000_000; fee reported once.
    assert_eq!(e.raw_quote_delta, -500_000_000, "quote excludes fee");
    assert_eq!(e.network_fee, 5_000);
    assert_eq!(
        e.raw_quote_delta - e.network_fee as i128,
        -500_005_000,
        "inclusive view reconciles: quote - fee == raw delta"
    );
}
