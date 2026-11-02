//! PaperBroker red→green contract tests (H2). The broker is event-sourced:
//! every mutation appends to a JSONL ledger, and a broker can be rebuilt
//! from that ledger (crash recovery). Fills price ONLY against the
//! point-in-time snapshot carried by the decision — never a newer
//! observation (no lookahead). Closed trades export to the exact JSONL
//! contract `harness/scripts/evaluate_paper.py` consumes.

use sniper_core::primitives::{BasisPoints, Lamports};
use sniper_core::venue::Venue;
use sniper_runtime::broker::{
    CurveSnapshot, DecisionMeta, MarketSnapshot, NoFillReason, OrderRequest, PaperBroker,
    PaperBrokerConfig,
};
use solana_sdk::pubkey::Pubkey;

fn config() -> PaperBrokerConfig {
    PaperBrokerConfig {
        starting_cash: Lamports(1_000_000_000),
        fee_bps: BasisPoints(100), // 1%
        network_fee_lamports: Lamports(5_000),
        tip_lamports: Lamports(1_000),
        ata_rent_lamports: Lamports(1_500_000),
        slippage_bps: BasisPoints(25), // 0.25%
        max_snapshot_age_slots: 30,
    }
}

fn snapshot(slot: u64) -> MarketSnapshot {
    MarketSnapshot {
        slot,
        at_ms: 1_700_000_000_000 + slot * 400,
        reserves: CurveSnapshot {
            virtual_token_reserves: 1_073_000_000_000_000,
            virtual_quote_reserves: 30_000_000_000,
        },
    }
}

fn depressed_snapshot(slot: u64) -> MarketSnapshot {
    // Same pool after heavy buying: far fewer tokens per lamport (worse
    // for buyers).
    MarketSnapshot {
        slot,
        at_ms: 1_700_000_000_000 + slot * 400,
        reserves: CurveSnapshot {
            virtual_token_reserves: 400_000_000_000_000,
            virtual_quote_reserves: 80_000_000_000,
        },
    }
}

fn dumped_snapshot(slot: u64) -> MarketSnapshot {
    // Same pool after heavy selling INTO it: more tokens, less quote —
    // worse for sellers (executable exit marked down).
    MarketSnapshot {
        slot,
        at_ms: 1_700_000_000_000 + slot * 400,
        reserves: CurveSnapshot {
            virtual_token_reserves: 1_200_000_000_000_000,
            virtual_quote_reserves: 25_000_000_000,
        },
    }
}

fn meta() -> DecisionMeta {
    DecisionMeta {
        strategy: "snipe-v1".into(),
        decided_at_ms: 1_700_000_000_123,
        features_observed_at_ms: 1_700_000_000_100,
        features_observed_slot: 100,
    }
}

const STRATEGY_LABEL: &str = "snipe-v1";

#[test]
fn reported_buy_has_nonzero_position_and_pays_costs() {
    let mut b = PaperBroker::new(config());
    let mint = Pubkey::new_unique();
    let id = b.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Buy {
            spend: Lamports(50_000_000),
            min_tokens_out: 0,
        },
        meta(),
        snapshot(100),
    );
    let s = b.settle(id, 100);
    assert!(s.filled(), "buy with fresh snapshot fills: {s:?}");
    let pos = b.position(&mint).expect("position exists after fill");
    assert!(pos.quantity > 0, "filled buy must open a NONZERO position");
    // Cash outflow = spend + network fee + tip + first-open rent.
    let expected_cash = 1_000_000_000u64 - 50_000_000 - 5_000 - 1_000 - 1_500_000;
    assert_eq!(b.cash().0, expected_cash);
}

#[test]
fn profitable_round_trip_realizes_positive_pnl() {
    let mut b = PaperBroker::new(config());
    let mint = Pubkey::new_unique();
    // Buy at healthy reserves…
    let buy = b.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Buy {
            spend: Lamports(50_000_000),
            min_tokens_out: 0,
        },
        meta(),
        snapshot(100),
    );
    b.settle(buy, 100);
    let qty = b.position(&mint).unwrap().quantity;
    // …sell after the pool got richer in quote (pump scenario).
    let richer = MarketSnapshot {
        slot: 130,
        at_ms: 1_700_000_013_000,
        reserves: CurveSnapshot {
            virtual_token_reserves: 900_000_000_000_000,
            virtual_quote_reserves: 35_000_000_000,
        },
    };
    let sell = b.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Sell {
            tokens: qty,
            min_quote_out: Lamports(0),
        },
        meta(),
        richer,
    );
    let s = b.settle(sell, 130);
    assert!(s.filled());
    assert!(b.realized_pnl() > 0, "pump scenario must realize profit");
    // Position flattened.
    assert!(b.position(&mint).unwrap().quantity == 0);
}

#[test]
fn round_trip_at_flat_state_loses_both_fees_and_slippage() {
    let mut b = PaperBroker::new(config());
    let mint = Pubkey::new_unique();
    let buy = b.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Buy {
            spend: Lamports(50_000_000),
            min_tokens_out: 0,
        },
        meta(),
        snapshot(100),
    );
    b.settle(buy, 100);
    let qty = b.position(&mint).unwrap().quantity;
    let sell = b.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Sell {
            tokens: qty,
            min_quote_out: Lamports(0),
        },
        meta(),
        snapshot(101), // same reserves: pure cost round trip
    );
    b.settle(sell, 101);
    let pnl = b.realized_pnl();
    assert!(
        pnl < 0,
        "flat round trip must lose fees+slippage+network, got {pnl}"
    );
    // Rough magnitude: > 2% (entry+exit fee) of the 50M spend.
    assert!(
        pnl < -1_000_000,
        "loss magnitude must reflect real costs: {pnl}"
    );
}

#[test]
fn stale_snapshot_never_fills() {
    let mut b = PaperBroker::new(config());
    let mint = Pubkey::new_unique();
    let id = b.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Buy {
            spend: Lamports(10_000_000),
            min_tokens_out: 0,
        },
        meta(),
        snapshot(100),
    );
    // Settle 31 slots later (max age 30) — stale, no fill.
    let s = b.settle(id, 131);
    assert!(!s.filled());
    assert!(
        matches!(s.reason(), Some(NoFillReason::StaleSnapshot { age: 31 })),
        "expected stale no-fill, got {s:?}"
    );
    assert!(b.position(&mint).is_none());
    // And a fresh one still works at exactly max age.
    let id2 = b.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Buy {
            spend: Lamports(10_000_000),
            min_tokens_out: 0,
        },
        meta(),
        snapshot(101),
    );
    assert!(b.settle(id2, 131).filled(), "age 30 == max must fill");
}

#[test]
fn min_out_unmet_is_no_fill_not_a_worse_fill() {
    let mut b = PaperBroker::new(config());
    let mint = Pubkey::new_unique();
    // Demand an absurd number of tokens: no fill, nothing booked.
    let id = b.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Buy {
            spend: Lamports(10_000_000),
            min_tokens_out: 900_000_000_000_000,
        },
        meta(),
        snapshot(100),
    );
    let s = b.settle(id, 100);
    assert!(!s.filled());
    assert!(matches!(s.reason(), Some(NoFillReason::MinOutUnmet { .. })));
    assert!(b.position(&mint).is_none());

    // Sell side: min quote out above achievable → no fill (exit
    // infeasibility gate).
    let buy = b.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Buy {
            spend: Lamports(50_000_000),
            min_tokens_out: 0,
        },
        meta(),
        snapshot(100),
    );
    b.settle(buy, 100);
    let qty = b.position(&mint).unwrap().quantity;
    let sell = b.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Sell {
            tokens: qty,
            min_quote_out: Lamports(50_000_000_000),
        },
        meta(),
        snapshot(101),
    );
    let s = b.settle(sell, 101);
    assert!(!s.filled());
    assert!(matches!(s.reason(), Some(NoFillReason::MinOutUnmet { .. })));
}

#[test]
fn insufficient_cash_is_no_fill() {
    let mut cfg = config();
    cfg.starting_cash = Lamports(1_000_000);
    let mut b = PaperBroker::new(cfg);
    let id = b.place(
        Pubkey::new_unique(),
        Venue::PumpFun,
        OrderRequest::Buy {
            spend: Lamports(50_000_000),
            min_tokens_out: 0,
        },
        meta(),
        snapshot(100),
    );
    let s = b.settle(id, 100);
    assert!(!s.filled());
    assert!(matches!(
        s.reason(),
        Some(NoFillReason::InsufficientCash { .. })
    ));
}

#[test]
fn settle_uses_decision_snapshot_not_newer_observation() {
    let mut b = PaperBroker::new(config());
    let mint = Pubkey::new_unique();
    let id = b.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Buy {
            spend: Lamports(50_000_000),
            min_tokens_out: 0,
        },
        meta(),
        snapshot(100), // decision-time state
    );
    // A newer, much WORSE observation arrives before settlement.
    b.observe(mint, Venue::PumpFun, depressed_snapshot(105));
    b.settle(id, 105);
    // The fill must have priced on snapshot(100): tokens equal a
    // snapshot(100) buy, not a depressed_snapshot buy.
    let pos = b.position(&mint).unwrap();
    let mut check = PaperBroker::new(config());
    let cid = check.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Buy {
            spend: Lamports(50_000_000),
            min_tokens_out: 0,
        },
        meta(),
        snapshot(100),
    );
    check.settle(cid, 100);
    assert_eq!(pos.quantity, check.position(&mint).unwrap().quantity);
}

#[test]
fn migration_mid_position_switches_venue_and_pricing() {
    let mut b = PaperBroker::new(config());
    let mint = Pubkey::new_unique();
    let buy = b.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Buy {
            spend: Lamports(50_000_000),
            min_tokens_out: 0,
        },
        meta(),
        snapshot(100),
    );
    b.settle(buy, 100);
    let qty = b.position(&mint).unwrap().quantity;
    assert_eq!(b.position(&mint).unwrap().venue, Venue::PumpFun);

    // Curve completes → position migrates to PumpSwap with pool pricing.
    let pool = MarketSnapshot {
        slot: 120,
        at_ms: 1_700_000_048_000,
        reserves: CurveSnapshot {
            virtual_token_reserves: 200_000_000_000_000,
            virtual_quote_reserves: 45_000_000_000,
        },
    };
    b.apply_migration(mint, Venue::PumpSwap, pool);
    let pos = b.position(&mint).unwrap();
    assert_eq!(pos.venue, Venue::PumpSwap, "position follows the migration");
    assert_eq!(pos.quantity, qty, "tokens survive migration");

    // Sell now prices on the POOL snapshot and books against PumpSwap.
    let sell = b.place(
        mint,
        Venue::PumpSwap,
        OrderRequest::Sell {
            tokens: qty,
            min_quote_out: Lamports(0),
        },
        meta(),
        pool,
    );
    let s = b.settle(sell, 120);
    assert!(s.filled());
    assert_eq!(b.position(&mint).unwrap().venue, Venue::PumpSwap);
}

#[test]
fn crash_recovery_rebuilds_from_ledger_and_continues() {
    let mut b = PaperBroker::new(config());
    let mint = Pubkey::new_unique();
    let buy = b.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Buy {
            spend: Lamports(50_000_000),
            min_tokens_out: 0,
        },
        meta(),
        snapshot(100),
    );
    b.settle(buy, 100);
    let pre_qty = b.position(&mint).unwrap().quantity;
    let pre_cash = b.cash();
    let pre_realized = b.realized_pnl();

    // "Crash": rebuild purely from the ledger bytes.
    let ledger = b.ledger_jsonl();
    let mut restored =
        PaperBroker::replay_ledger(&config(), &ledger).expect("ledger replays into a broker");
    assert_eq!(restored.cash(), pre_cash);
    assert_eq!(restored.position(&mint).unwrap().quantity, pre_qty);
    assert_eq!(restored.realized_pnl(), pre_realized);

    // Continue trading after recovery: sell closes cleanly.
    let qty = restored.position(&mint).unwrap().quantity;
    let sell = restored.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Sell {
            tokens: qty,
            min_quote_out: Lamports(0),
        },
        meta(),
        snapshot(110),
    );
    assert!(restored.settle(sell, 110).filled());
}

#[test]
fn rerun_is_deterministic_identical_jsonl() {
    let run = || {
        let mut b = PaperBroker::new(config());
        let mint = Pubkey::from([7u8; 32]);
        let buy = b.place(
            mint,
            Venue::PumpFun,
            OrderRequest::Buy {
                spend: Lamports(50_000_000),
                min_tokens_out: 0,
            },
            meta(),
            snapshot(100),
        );
        b.settle(buy, 100);
        let qty = b.position(&mint).unwrap().quantity;
        let sell = b.place(
            mint,
            Venue::PumpFun,
            OrderRequest::Sell {
                tokens: qty / 2,
                min_quote_out: Lamports(0),
            },
            meta(),
            snapshot(110),
        );
        b.settle(sell, 110);
        (
            b.export_closed_trades_jsonl(),
            b.ledger_jsonl(),
            b.realized_pnl(),
        )
    };
    let a = run();
    let b_ = run();
    assert_eq!(a.0, b_.0, "closed-trade export must be byte-identical");
    assert_eq!(a.1, b_.1, "ledger must be byte-identical");
    assert_eq!(a.2, b_.2);
}

#[test]
fn closed_trade_export_satisfies_evaluator_contract() {
    let mut b = PaperBroker::new(config());
    let mint = Pubkey::from([9u8; 32]);
    let buy = b.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Buy {
            spend: Lamports(50_000_000),
            min_tokens_out: 0,
        },
        meta(),
        snapshot(100),
    );
    b.settle(buy, 100);
    let qty = b.position(&mint).unwrap().quantity;
    let sell = b.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Sell {
            tokens: qty,
            min_quote_out: Lamports(0),
        },
        meta(),
        snapshot(115),
    );
    b.settle(sell, 115);

    let jsonl = b.export_closed_trades_jsonl();
    let line = jsonl.lines().next().expect("one closed trade");
    let row: serde_json::Value = serde_json::from_str(line).unwrap();
    for key in [
        "trade_id",
        "strategy",
        "mint",
        "decision_at_ms",
        "features_observed_at_ms",
        "entry_at_ms",
        "exit_at_ms",
        "entry_quote_debited",
        "exit_quote_received",
        "network_fees_lamports",
        "tips_lamports",
        "entry_slot",
        "exit_slot",
    ] {
        assert!(row.get(key).is_some(), "missing evaluator field {key}");
    }
    for key in [
        "decision_at_ms",
        "features_observed_at_ms",
        "entry_at_ms",
        "exit_at_ms",
        "entry_quote_debited",
        "exit_quote_received",
        "network_fees_lamports",
        "tips_lamports",
        "entry_slot",
        "exit_slot",
    ] {
        assert!(
            row[key].is_u64(),
            "evaluator requires integer {key}, got {}",
            row[key]
        );
    }
    assert_eq!(row["strategy"], STRATEGY_LABEL);
    assert_eq!(row["mint"], mint.to_string());
    assert!(row["exit_quote_received"].as_u64().unwrap() > 0);
}

#[test]
fn equity_mark_to_exit_and_drawdown_track_state() {
    let mut b = PaperBroker::new(config());
    assert_eq!(b.equity_mark_to_exit(), 1_000_000_000);
    let mint = Pubkey::new_unique();
    let buy = b.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Buy {
            spend: Lamports(50_000_000),
            min_tokens_out: 0,
        },
        meta(),
        snapshot(100),
    );
    b.settle(buy, 100);
    // Open position marked at executable exit (sell-all into the SAME
    // observed state): equity reflects round-trip costs immediately.
    let eq = b.equity_mark_to_exit();
    assert!(eq < 1_000_000_000, "open position carries spread+fees");
    assert!(eq > 900_000_000, "but the position retains most value");

    // Mark down hard (dump into the pool) → drawdown grows.
    b.observe(mint, Venue::PumpFun, dumped_snapshot(105));
    let eq2 = b.equity_mark_to_exit();
    assert!(eq2 < eq, "worse reserves must mark equity down");
    assert!(b.max_drawdown() > 0);
}

#[test]
fn sell_more_than_position_is_no_fill() {
    let mut b = PaperBroker::new(config());
    let mint = Pubkey::new_unique();
    let sell = b.place(
        mint,
        Venue::PumpFun,
        OrderRequest::Sell {
            tokens: 1_000,
            min_quote_out: Lamports(0),
        },
        meta(),
        snapshot(100),
    );
    let s = b.settle(sell, 100);
    assert!(!s.filled());
    assert!(matches!(
        s.reason(),
        Some(NoFillReason::InsufficientPosition { .. })
    ));
}
