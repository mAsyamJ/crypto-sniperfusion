//! Event-sourced paper broker (H2). Real virtual orders, fills priced with
//! the M3 curve math, fee/rent/network/tip accounting, positions by
//! (mint, venue), realized/unrealized P&L with mark-to-EXECUTABLE-exit,
//! migration handling, deterministic closed-trade export for
//! `harness/scripts/evaluate_paper.py`, and crash recovery by ledger
//! replay. NEVER a real broadcaster: this module holds no signing keys and
//! touches no network.
//!
//! No-lookahead rule: an order carries the point-in-time market snapshot
//! the decision was made on; `settle` prices ONLY against that snapshot
//! (plus the staleness gate). Newer observations never retroactively
//! improve a fill.
//!
//! Event sourcing: the ledger is a COMMAND log (`place`, `settle`,
//! `observe`, `migrate`). Recovery replays the commands through the same
//! deterministic code, so a rebuilt broker is bit-identical in state and
//! exports.

use std::collections::HashMap;

use sniper_core::event::ChainEvent;
use sniper_core::primitives::{BasisPoints, Lamports};
use sniper_core::venue::Venue;
use solana_sdk::pubkey::Pubkey;

pub use crate::paper::CurveSnapshot;
use crate::paper::{paper_buy, paper_sell};

/// Static broker parameters (from config, never hardcoded protocol rates).
#[derive(Debug, Clone, PartialEq)]
pub struct PaperBrokerConfig {
    pub starting_cash: Lamports,
    /// Curve/pool fee applied on entry and exit legs.
    pub fee_bps: BasisPoints,
    /// Network fee charged per fill transaction.
    pub network_fee_lamports: Lamports,
    /// Tip charged per fill transaction.
    pub tip_lamports: Lamports,
    /// One-time (per mint) ATA rent charged when a position first opens.
    pub ata_rent_lamports: Lamports,
    /// Adverse slippage applied to fills on top of the curve math.
    pub slippage_bps: BasisPoints,
    /// Snapshots older than this (slots) never fill.
    pub max_snapshot_age_slots: u64,
}

impl Default for PaperBrokerConfig {
    fn default() -> Self {
        Self {
            starting_cash: Lamports(1_000_000_000),
            fee_bps: BasisPoints(100),
            network_fee_lamports: Lamports(5_000),
            tip_lamports: Lamports(1_000),
            ata_rent_lamports: Lamports(1_500_000),
            slippage_bps: BasisPoints(25),
            max_snapshot_age_slots: 30,
        }
    }
}

/// Point-in-time market state the broker may price against.
#[derive(Debug, Clone, Copy)]
pub struct MarketSnapshot {
    pub slot: u64,
    pub at_ms: u64,
    pub reserves: CurveSnapshot,
}

impl PartialEq for MarketSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot
            && self.at_ms == other.at_ms
            && self.reserves.virtual_token_reserves == other.reserves.virtual_token_reserves
            && self.reserves.virtual_quote_reserves == other.reserves.virtual_quote_reserves
    }
}

/// Decision provenance carried by every order (evaluator contract fields).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DecisionMeta {
    pub strategy: String,
    pub decided_at_ms: u64,
    pub features_observed_at_ms: u64,
    pub features_observed_slot: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "side", rename_all = "snake_case")]
pub enum OrderRequest {
    #[serde(rename = "buy")]
    Buy {
        spend: Lamports,
        min_tokens_out: u64,
    },
    #[serde(rename = "sell")]
    Sell {
        tokens: u64,
        min_quote_out: Lamports,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrderStatus {
    Planned,
    Filled,
    Unfilled(NoFillReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum NoFillReason {
    #[serde(rename = "stale_snapshot")]
    StaleSnapshot { age: u64 },
    #[serde(rename = "min_out_unmet")]
    MinOutUnmet { got: u64, min: u64 },
    #[serde(rename = "insufficient_cash")]
    InsufficientCash { have: u64, need: u64 },
    #[serde(rename = "insufficient_position")]
    InsufficientPosition { have: u64, want: u64 },
    #[serde(rename = "unknown_order")]
    UnknownOrder,
}

/// One virtual order in the lifecycle PLANNED → FILLED/UNFILLED.
#[derive(Debug, Clone)]
struct VirtualOrder {
    id: u64,
    mint: Pubkey,
    venue: Venue,
    request: OrderRequest,
    meta: DecisionMeta,
    /// The decision-time snapshot; the ONLY state a fill may price against.
    snapshot: MarketSnapshot,
    status: OrderStatus,
}

/// Result of settling one order.
#[derive(Debug, Clone, PartialEq)]
pub struct Settlement {
    pub order_id: u64,
    pub filled: bool,
    pub reason: Option<NoFillReason>,
    pub tokens_out: u64,
    pub quote_out: Lamports,
}

impl Settlement {
    pub fn filled(&self) -> bool {
        self.filled
    }

    pub fn reason(&self) -> Option<NoFillReason> {
        self.reason
    }
}

/// Open position per mint — venue switches on migration, quantity and
/// average cost survive.
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub mint: Pubkey,
    pub venue: Venue,
    pub quantity: u64,
    /// Total quote debited (pool fee + slippage included) to build it.
    pub cost_basis: Lamports,
}

/// A closed round trip in exactly the evaluator's contract.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ClosedTrade {
    pub trade_id: u64,
    pub strategy: String,
    pub mint: String,
    pub decision_at_ms: u64,
    pub features_observed_at_ms: u64,
    pub entry_at_ms: u64,
    pub exit_at_ms: u64,
    pub entry_quote_debited: u64,
    pub exit_quote_received: u64,
    pub network_fees_lamports: u64,
    pub tips_lamports: u64,
    pub entry_slot: u64,
    pub exit_slot: u64,
}

/// Command-log record (one JSON line per broker mutation).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum BrokerCommand {
    Started {
        starting_cash: u64,
    },
    Observed {
        mint: String,
        venue: String,
        slot: u64,
        at_ms: u64,
        virtual_token_reserves: u64,
        virtual_quote_reserves: u64,
    },
    OrderPlaced {
        id: u64,
        mint: String,
        venue: String,
        request: OrderRequest,
        meta: DecisionMeta,
        snapshot_slot: u64,
        snapshot_at_ms: u64,
        virtual_token_reserves: u64,
        virtual_quote_reserves: u64,
    },
    SettleRequested {
        id: u64,
        current_slot: u64,
    },
    MigrationApplied {
        mint: String,
        to: String,
        slot: u64,
        at_ms: u64,
        virtual_token_reserves: u64,
        virtual_quote_reserves: u64,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum BrokerError {
    #[error("ledger decode failed at line {line}: {message}")]
    LedgerDecode { line: usize, message: String },
    #[error("ledger mint invalid at line {line}: {mint}")]
    LedgerMint { line: usize, mint: String },
}

/// Deterministic fill-landing latency baked into paper timestamps so
/// features_observed <= decided <= entry <= exit always holds (the
/// independent evaluator enforces this ordering).
const FILL_LAND_MS: u64 = 70;

pub struct PaperBroker {
    config: PaperBrokerConfig,
    cash: Lamports,
    positions: HashMap<Pubkey, Position>,
    orders: Vec<VirtualOrder>,
    closed: Vec<ClosedTrade>,
    /// Latest observed snapshot per mint (marking only — never fills).
    market: HashMap<Pubkey, (Venue, MarketSnapshot)>,
    next_order_id: u64,
    next_trade_id: u64,
    realized_pnl: i128,
    peak_equity: i128,
    max_drawdown: i128,
    ledger: Vec<BrokerCommand>,
    /// Unmatched buy lots; exits match FIFO WITHIN THE SAME MINT, including
    /// partial exits. The global insertion order is not a global cost basis.
    open_entries: Vec<ClosedEntry>,
}

/// Entry side of a future ClosedTrade, waiting for its exit.
#[derive(Debug, Clone)]
struct ClosedEntry {
    #[allow(dead_code)] // audit provenance
    order_id: u64,
    remaining_tokens: u64,
    strategy: String,
    mint: Pubkey,
    decision_at_ms: u64,
    features_observed_at_ms: u64,
    entry_at_ms: u64,
    entry_slot: u64,
    entry_quote_debited: u64,
    entry_network_fee: u64,
    entry_tip: u64,
}

fn venue_str(v: Venue) -> String {
    format!("{v:?}")
}

fn venue_from(s: &str) -> Venue {
    match s {
        "PumpFun" => Venue::PumpFun,
        "PumpSwap" => Venue::PumpSwap,
        "LetsBonk" => Venue::LetsBonk,
        "StonkFun" => Venue::StonkFun,
        "RaydiumAmm" => Venue::RaydiumAmm,
        _ => Venue::Unknown,
    }
}

fn parse_pubkey(s: &str) -> Option<Pubkey> {
    s.parse().ok()
}

fn apply_slippage_down(amount: u64, bps: BasisPoints) -> u64 {
    let cut = (amount as u128 * bps.0 as u128 / 10_000) as u64;
    amount - cut
}

impl PaperBroker {
    pub fn new(config: PaperBrokerConfig) -> Self {
        let mut b = Self {
            cash: config.starting_cash,
            config: config.clone(),
            positions: HashMap::new(),
            orders: Vec::new(),
            closed: Vec::new(),
            market: HashMap::new(),
            next_order_id: 1,
            next_trade_id: 1,
            realized_pnl: 0,
            peak_equity: config.starting_cash.0 as i128,
            max_drawdown: 0,
            ledger: Vec::new(),
            open_entries: Vec::new(),
        };
        b.record(BrokerCommand::Started {
            starting_cash: b.config.starting_cash.0,
        });
        b
    }

    fn record(&mut self, event: BrokerCommand) {
        self.ledger.push(event);
    }

    /// Update the latest market view (marking only — fills never read it).
    pub fn observe(&mut self, mint: Pubkey, venue: Venue, snap: MarketSnapshot) {
        self.record(BrokerCommand::Observed {
            mint: mint.to_string(),
            venue: venue_str(venue),
            slot: snap.slot,
            at_ms: snap.at_ms,
            virtual_token_reserves: snap.reserves.virtual_token_reserves,
            virtual_quote_reserves: snap.reserves.virtual_quote_reserves,
        });
        self.market.insert(mint, (venue, snap));
        self.update_drawdown();
    }

    /// Place a virtual order carrying its OWN decision-time snapshot.
    pub fn place(
        &mut self,
        mint: Pubkey,
        venue: Venue,
        request: OrderRequest,
        meta: DecisionMeta,
        snapshot: MarketSnapshot,
    ) -> u64 {
        let id = self.next_order_id;
        self.next_order_id += 1;
        self.record(BrokerCommand::OrderPlaced {
            id,
            mint: mint.to_string(),
            venue: venue_str(venue),
            request,
            meta: meta.clone(),
            snapshot_slot: snapshot.slot,
            snapshot_at_ms: snapshot.at_ms,
            virtual_token_reserves: snapshot.reserves.virtual_token_reserves,
            virtual_quote_reserves: snapshot.reserves.virtual_quote_reserves,
        });
        self.orders.push(VirtualOrder {
            id,
            mint,
            venue,
            request,
            meta,
            snapshot,
            status: OrderStatus::Planned,
        });
        // Trading on a snapshot implies observing it: register the market
        // view so mark-to-exit works from the decision state onward.
        self.market.insert(mint, (venue, snapshot));
        id
    }

    /// Settle one order against its decision snapshot. Deterministic.
    pub fn settle(&mut self, order_id: u64, current_slot: u64) -> Settlement {
        self.record(BrokerCommand::SettleRequested {
            id: order_id,
            current_slot,
        });
        let s = self.settle_inner(order_id, current_slot);
        self.update_drawdown();
        s
    }

    fn settle_inner(&mut self, order_id: u64, current_slot: u64) -> Settlement {
        let Some(order) = self.orders.iter().find(|o| o.id == order_id) else {
            return self.unfilled_settlement(order_id, NoFillReason::UnknownOrder);
        };
        if order.status != OrderStatus::Planned {
            // Idempotent settle: return the recorded outcome without
            // double-applying effects.
            return match order.status {
                OrderStatus::Filled => Settlement {
                    order_id,
                    filled: true,
                    reason: None,
                    tokens_out: 0,
                    quote_out: Lamports(0),
                },
                OrderStatus::Unfilled(r) => self.unfilled_settlement(order_id, r),
                OrderStatus::Planned => unreachable!(),
            };
        }

        // Staleness gate — old state never fills.
        let age = current_slot.saturating_sub(order.snapshot.slot);
        if age > self.config.max_snapshot_age_slots {
            let reason = NoFillReason::StaleSnapshot { age };
            self.mark_unfilled(order_id, reason);
            return self.unfilled_settlement(order_id, reason);
        }

        match order.request {
            OrderRequest::Buy {
                spend,
                min_tokens_out,
            } => self.settle_buy(order_id, spend, min_tokens_out),
            OrderRequest::Sell {
                tokens,
                min_quote_out,
            } => self.settle_sell(order_id, tokens, min_quote_out),
        }
    }

    fn settle_buy(&mut self, order_id: u64, spend: Lamports, min_tokens_out: u64) -> Settlement {
        let (meta, mint, venue, slot, _at_ms) = self.order_parts(order_id);
        let reserves = self.order_snapshot(order_id).reserves;

        // Costs: spend + network + tip (+ rent on first open of the mint).
        let rent = if self.positions.contains_key(&mint) {
            0
        } else {
            self.config.ata_rent_lamports.0
        };
        let need = spend.0 + self.config.network_fee_lamports.0 + self.config.tip_lamports.0 + rent;
        if need > self.cash.0 {
            let reason = NoFillReason::InsufficientCash {
                have: self.cash.0,
                need,
            };
            self.mark_unfilled(order_id, reason);
            return self.unfilled_settlement(order_id, reason);
        }

        // Price on the DECISION snapshot with adverse slippage.
        let slipped = Lamports(apply_slippage_down(spend.0, self.config.slippage_bps));
        let Ok(fill) = paper_buy(&reserves, slipped, self.config.fee_bps) else {
            let reason = NoFillReason::MinOutUnmet {
                got: 0,
                min: min_tokens_out,
            };
            self.mark_unfilled(order_id, reason);
            return self.unfilled_settlement(order_id, reason);
        };
        if fill.tokens_out < min_tokens_out {
            let reason = NoFillReason::MinOutUnmet {
                got: fill.tokens_out,
                min: min_tokens_out,
            };
            self.mark_unfilled(order_id, reason);
            return self.unfilled_settlement(order_id, reason);
        }

        self.cash.0 -= need;
        let entry = self.positions.entry(mint).or_insert(Position {
            mint,
            venue,
            quantity: 0,
            cost_basis: Lamports(0),
        });
        entry.venue = venue;
        entry.quantity += fill.tokens_out;
        entry.cost_basis.0 += spend.0;

        self.open_entries.push(ClosedEntry {
            order_id,
            remaining_tokens: fill.tokens_out,
            strategy: meta.strategy,
            mint,
            decision_at_ms: meta.decided_at_ms,
            features_observed_at_ms: meta.features_observed_at_ms,
            entry_at_ms: meta.decided_at_ms + FILL_LAND_MS,
            entry_slot: slot,
            entry_quote_debited: spend.0,
            entry_network_fee: self.config.network_fee_lamports.0,
            entry_tip: self.config.tip_lamports.0,
        });
        if let Some(o) = self.orders.iter_mut().find(|o| o.id == order_id) {
            o.status = OrderStatus::Filled;
        }
        Settlement {
            order_id,
            filled: true,
            reason: None,
            tokens_out: fill.tokens_out,
            quote_out: Lamports(0),
        }
    }

    fn settle_sell(&mut self, order_id: u64, tokens: u64, min_quote_out: Lamports) -> Settlement {
        let (meta_of_sell, mint, _venue, slot, _at_ms) = self.order_parts(order_id);
        let reserves = self.order_snapshot(order_id).reserves;

        let Some(pos) = self.positions.get(&mint) else {
            let reason = NoFillReason::InsufficientPosition {
                have: 0,
                want: tokens,
            };
            self.mark_unfilled(order_id, reason);
            return self.unfilled_settlement(order_id, reason);
        };
        if pos.quantity < tokens {
            let reason = NoFillReason::InsufficientPosition {
                have: pos.quantity,
                want: tokens,
            };
            self.mark_unfilled(order_id, reason);
            return self.unfilled_settlement(order_id, reason);
        }

        let Ok(fill) = paper_sell(&reserves, tokens, self.config.fee_bps) else {
            let reason = NoFillReason::MinOutUnmet {
                got: 0,
                min: min_quote_out.0,
            };
            self.mark_unfilled(order_id, reason);
            return self.unfilled_settlement(order_id, reason);
        };
        let net = apply_slippage_down(fill.quote_out_net.0, self.config.slippage_bps);
        if net < min_quote_out.0 {
            let reason = NoFillReason::MinOutUnmet {
                got: net,
                min: min_quote_out.0,
            };
            self.mark_unfilled(order_id, reason);
            return self.unfilled_settlement(order_id, reason);
        }

        self.cash.0 +=
            net.saturating_sub(self.config.network_fee_lamports.0 + self.config.tip_lamports.0);

        // Match lots only for this mint. Split both entry economics and
        // exit proceeds/fees proportionally; the final matched segment gets
        // the integer-division remainder. Partial exits retain an open lot.
        let mut unmatched = tokens;
        let mut cost_share = 0u64;
        let mut distributed_proceeds = 0u64;
        let mut distributed_network = 0u64;
        let mut distributed_tip = 0u64;
        while unmatched > 0 {
            let idx = self
                .open_entries
                .iter()
                .position(|entry| entry.mint == mint && entry.remaining_tokens > 0)
                .expect("filled position must have an unmatched mint-specific lot");
            let entry = &mut self.open_entries[idx];
            let matched = unmatched.min(entry.remaining_tokens);
            let last_part = matched == unmatched;
            let allocate = |amount: u64, denominator: u64| -> u64 {
                (amount as u128 * matched as u128 / denominator as u128) as u64
            };
            let entry_cost = allocate(entry.entry_quote_debited, entry.remaining_tokens);
            let entry_network = allocate(entry.entry_network_fee, entry.remaining_tokens);
            let entry_tip = allocate(entry.entry_tip, entry.remaining_tokens);
            let proceeds = if last_part {
                net - distributed_proceeds
            } else {
                (net as u128 * matched as u128 / tokens as u128) as u64
            };
            let exit_network = if last_part {
                self.config.network_fee_lamports.0 - distributed_network
            } else {
                (self.config.network_fee_lamports.0 as u128 * matched as u128
                    / tokens as u128) as u64
            };
            let exit_tip = if last_part {
                self.config.tip_lamports.0 - distributed_tip
            } else {
                (self.config.tip_lamports.0 as u128 * matched as u128 / tokens as u128)
                    as u64
            };
            self.closed.push(ClosedTrade {
                trade_id: self.next_trade_id,
                strategy: entry.strategy.clone(),
                mint: mint.to_string(),
                decision_at_ms: entry.decision_at_ms,
                features_observed_at_ms: entry.features_observed_at_ms,
                entry_at_ms: entry.entry_at_ms,
                exit_at_ms: meta_of_sell.decided_at_ms + FILL_LAND_MS,
                entry_quote_debited: entry_cost,
                exit_quote_received: proceeds,
                network_fees_lamports: entry_network + exit_network,
                tips_lamports: entry_tip + exit_tip,
                entry_slot: entry.entry_slot,
                exit_slot: slot,
            });
            self.next_trade_id += 1;
            entry.remaining_tokens -= matched;
            entry.entry_quote_debited -= entry_cost;
            entry.entry_network_fee -= entry_network;
            entry.entry_tip -= entry_tip;
            if entry.remaining_tokens == 0 {
                self.open_entries.remove(idx);
            }
            cost_share += entry_cost;
            distributed_proceeds += proceeds;
            distributed_network += exit_network;
            distributed_tip += exit_tip;
            unmatched -= matched;
        }

        // Align position cost basis and broker realized P&L with FIFO lots.
        let pos = self.positions.get_mut(&mint).expect("checked above");
        pos.quantity -= tokens;
        pos.cost_basis.0 = pos.cost_basis.0.saturating_sub(cost_share);
        self.realized_pnl += net as i128 - cost_share as i128;

        if let Some(o) = self.orders.iter_mut().find(|o| o.id == order_id) {
            o.status = OrderStatus::Filled;
        }
        Settlement {
            order_id,
            filled: true,
            reason: None,
            tokens_out: tokens,
            quote_out: Lamports(net),
        }
    }

    fn mark_unfilled(&mut self, order_id: u64, reason: NoFillReason) {
        if let Some(o) = self.orders.iter_mut().find(|o| o.id == order_id) {
            o.status = OrderStatus::Unfilled(reason);
        }
    }

    fn order_parts(&self, order_id: u64) -> (DecisionMeta, Pubkey, Venue, u64, u64) {
        let o = self
            .orders
            .iter()
            .find(|o| o.id == order_id)
            .expect("caller checked");
        (
            o.meta.clone(),
            o.mint,
            o.venue,
            o.snapshot.slot,
            o.snapshot.at_ms,
        )
    }

    fn order_snapshot(&self, order_id: u64) -> MarketSnapshot {
        self.orders
            .iter()
            .find(|o| o.id == order_id)
            .expect("caller checked")
            .snapshot
    }

    fn unfilled_settlement(&self, order_id: u64, reason: NoFillReason) -> Settlement {
        Settlement {
            order_id,
            filled: false,
            reason: Some(reason),
            tokens_out: 0,
            quote_out: Lamports(0),
        }
    }

    fn update_drawdown(&mut self) {
        let eq = self.equity_mark_to_exit();
        if eq > self.peak_equity {
            self.peak_equity = eq;
        }
        let dd = self.peak_equity - eq;
        if dd > self.max_drawdown {
            self.max_drawdown = dd;
        }
    }

    /// Curve completion mid-position: venue switches, quantity survives,
    /// pricing/marking moves to the pool snapshot.
    pub fn apply_migration(&mut self, mint: Pubkey, to: Venue, pool: MarketSnapshot) {
        self.record(BrokerCommand::MigrationApplied {
            mint: mint.to_string(),
            to: venue_str(to),
            slot: pool.slot,
            at_ms: pool.at_ms,
            virtual_token_reserves: pool.reserves.virtual_token_reserves,
            virtual_quote_reserves: pool.reserves.virtual_quote_reserves,
        });
        if let Some(pos) = self.positions.get_mut(&mint) {
            pos.venue = to;
        }
        self.market.insert(mint, (to, pool));
        self.update_drawdown();
    }

    pub fn position(&self, mint: &Pubkey) -> Option<&Position> {
        self.positions.get(mint)
    }

    pub fn cash(&self) -> Lamports {
        self.cash
    }

    /// Open (nonzero-quantity) positions.
    pub fn position_count(&self) -> usize {
        self.positions.values().filter(|p| p.quantity > 0).count()
    }

    pub fn realized_pnl(&self) -> i128 {
        self.realized_pnl
    }

    pub fn max_drawdown(&self) -> i128 {
        self.max_drawdown
    }

    /// Cash plus every open position marked at its EXECUTABLE exit:
    /// selling the whole position into the latest observed state and
    /// paying exit fees/slippage/network costs.
    pub fn equity_mark_to_exit(&self) -> i128 {
        let mut eq = self.cash.0 as i128;
        for (mint, pos) in &self.positions {
            if pos.quantity == 0 {
                continue;
            }
            if let Some((_, snap)) = self.market.get(mint) {
                if let Ok(fill) = paper_sell(&snap.reserves, pos.quantity, self.config.fee_bps) {
                    let net = apply_slippage_down(fill.quote_out_net.0, self.config.slippage_bps);
                    eq += net as i128
                        - (self.config.network_fee_lamports.0 + self.config.tip_lamports.0) as i128;
                }
            }
        }
        eq
    }

    /// Deterministic closed-trade export (evaluator JSONL contract).
    pub fn export_closed_trades_jsonl(&self) -> String {
        self.closed
            .iter()
            .map(|t| serde_json::to_string(t).expect("trade serializes"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    }

    /// The full command log as JSONL (crash-recovery artifact).
    pub fn ledger_jsonl(&self) -> String {
        self.ledger
            .iter()
            .map(|e| serde_json::to_string(e).expect("event serializes"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    }

    /// Rebuild a broker by replaying a ledger produced by `ledger_jsonl`.
    /// Replays the same deterministic commands, so all derived state
    /// (positions, cash, P&L, drawdown, closed trades) is reconstructed.
    pub fn replay_ledger(config: &PaperBrokerConfig, ledger: &str) -> Result<Self, BrokerError> {
        let mut broker = Self {
            cash: config.starting_cash,
            config: config.clone(),
            positions: HashMap::new(),
            orders: Vec::new(),
            closed: Vec::new(),
            market: HashMap::new(),
            next_order_id: 1,
            next_trade_id: 1,
            realized_pnl: 0,
            peak_equity: config.starting_cash.0 as i128,
            max_drawdown: 0,
            ledger: Vec::new(),
            open_entries: Vec::new(),
        };
        for (i, line) in ledger.lines().filter(|l| !l.trim().is_empty()).enumerate() {
            let cmd: BrokerCommand =
                serde_json::from_str(line).map_err(|e| BrokerError::LedgerDecode {
                    line: i + 1,
                    message: e.to_string(),
                })?;
            broker
                .apply_command(cmd)
                .map_err(|m| BrokerError::LedgerMint {
                    line: i + 1,
                    mint: m,
                })?;
        }
        Ok(broker)
    }

    /// Apply one command WITHOUT re-recording it (replay path). Records are
    /// already in `self.ledger` from the primary path; replay seeds them
    /// wholesale to keep the rebuilt ledger byte-identical.
    fn apply_command(&mut self, cmd: BrokerCommand) -> Result<(), String> {
        self.ledger.push(cmd.clone());
        match cmd {
            BrokerCommand::Started { starting_cash } => {
                self.cash = Lamports(starting_cash);
                Ok(())
            }
            BrokerCommand::Observed {
                mint,
                venue,
                slot,
                at_ms,
                virtual_token_reserves,
                virtual_quote_reserves,
            } => {
                let mint = parse_pubkey(&mint).ok_or_else(|| mint.clone())?;
                self.market.insert(
                    mint,
                    (
                        venue_from(&venue),
                        MarketSnapshot {
                            slot,
                            at_ms,
                            reserves: CurveSnapshot {
                                virtual_token_reserves,
                                virtual_quote_reserves,
                            },
                        },
                    ),
                );
                self.update_drawdown();
                Ok(())
            }
            BrokerCommand::OrderPlaced {
                id,
                mint,
                venue,
                request,
                meta,
                snapshot_slot,
                snapshot_at_ms,
                virtual_token_reserves,
                virtual_quote_reserves,
            } => {
                let mint_key = parse_pubkey(&mint).ok_or_else(|| mint.clone())?;
                self.next_order_id = self.next_order_id.max(id + 1);
                let snap = MarketSnapshot {
                    slot: snapshot_slot,
                    at_ms: snapshot_at_ms,
                    reserves: CurveSnapshot {
                        virtual_token_reserves,
                        virtual_quote_reserves,
                    },
                };
                self.orders.push(VirtualOrder {
                    id,
                    mint: mint_key,
                    venue: venue_from(&venue),
                    request,
                    meta,
                    snapshot: snap,
                    status: OrderStatus::Planned,
                });
                self.market.insert(mint_key, (venue_from(&venue), snap));
                Ok(())
            }
            BrokerCommand::SettleRequested { id, current_slot } => {
                let _ = self.settle_inner(id, current_slot);
                self.update_drawdown();
                Ok(())
            }
            BrokerCommand::MigrationApplied {
                mint,
                to,
                slot,
                at_ms,
                virtual_token_reserves,
                virtual_quote_reserves,
            } => {
                let mint_key = parse_pubkey(&mint).ok_or_else(|| mint.clone())?;
                let to = venue_from(&to);
                if let Some(pos) = self.positions.get_mut(&mint_key) {
                    pos.venue = to;
                }
                self.market.insert(
                    mint_key,
                    (
                        to,
                        MarketSnapshot {
                            slot,
                            at_ms,
                            reserves: CurveSnapshot {
                                virtual_token_reserves,
                                virtual_quote_reserves,
                            },
                        },
                    ),
                );
                self.update_drawdown();
                Ok(())
            }
        }
    }
}

// ---- replay-driven paper strategy hook -------------------------------------

/// pump.fun launch-time virtual reserves (protocol constants; the same
/// values the paper-engine tests price against).
pub const PUMP_LAUNCH_VIRTUAL_TOKEN_RESERVES: u64 = 1_073_000_000_000_000;
pub const PUMP_LAUNCH_VIRTUAL_QUOTE_RESERVES: u64 = 30_000_000_000;

/// DecisionHook driving a [`PaperBroker`] over replay events:
/// - `TokenCreated` → buy `spend` at the launch-constant curve state;
/// - `TradeObserved` → track curve movement; the first observed SELL of a
///   mint we hold exits the whole position at the PRE-trade snapshot
///   (no lookahead on that trade);
/// - `MigrationObserved` → switch venue, quantity/pricing survive.
///
/// All timestamps derive from the event SLOT (slot × 400 ms) so runs are
/// byte-deterministic for identical fixtures.
pub struct PaperBrokerHook {
    broker: PaperBroker,
    buy_spend: Lamports,
    strategy: String,
    curves: HashMap<Pubkey, CurveSnapshot>,
    fills: u64,
    unfilled: u64,
    last_decisions: Vec<crate::runner::Decision>,
}

impl PaperBrokerHook {
    pub fn new(config: PaperBrokerConfig, buy_spend: Lamports, strategy: String) -> Self {
        Self {
            broker: PaperBroker::new(config),
            buy_spend,
            strategy,
            curves: HashMap::new(),
            fills: 0,
            unfilled: 0,
            last_decisions: Vec::new(),
        }
    }

    pub fn broker(&self) -> &PaperBroker {
        &self.broker
    }

    pub fn fills(&self) -> u64 {
        self.fills
    }

    pub fn unfilled(&self) -> u64 {
        self.unfilled
    }

    /// Open (nonzero-quantity) positions held by the broker.
    pub fn position_count(&self) -> usize {
        self.broker.position_count()
    }

    fn snapshot_at(&self, mint: &Pubkey, slot: u64) -> MarketSnapshot {
        let reserves = self.curves.get(mint).copied().unwrap_or(CurveSnapshot {
            virtual_token_reserves: PUMP_LAUNCH_VIRTUAL_TOKEN_RESERVES,
            virtual_quote_reserves: PUMP_LAUNCH_VIRTUAL_QUOTE_RESERVES,
        });
        MarketSnapshot {
            slot,
            at_ms: slot.saturating_mul(400),
            reserves,
        }
    }

    fn meta(&self, slot: u64) -> DecisionMeta {
        DecisionMeta {
            strategy: self.strategy.clone(),
            decided_at_ms: slot.saturating_mul(400) + 50,
            features_observed_at_ms: slot.saturating_mul(400),
            features_observed_slot: slot,
        }
    }

    fn settle_now(&mut self, order: u64, slot: u64) -> bool {
        let s = self.broker.settle(order, slot);
        if s.filled {
            self.fills += 1;
        } else {
            self.unfilled += 1;
        }
        s.filled
    }
}

impl crate::runner::DecisionHook for PaperBrokerHook {
    fn on_event(&mut self, event: &ChainEvent) -> Vec<crate::runner::Decision> {
        use crate::runner::{Decision, DecisionKind};
        use sniper_core::event::EventPayload;

        self.last_decisions.clear();
        match &event.payload {
            EventPayload::TokenCreated { mint, .. } => {
                if self.buy_spend.0 == 0 {
                    return Vec::new(); // zero spend = no buys (safe default)
                }
                let snap = self.snapshot_at(mint, event.slot);
                let order = self.broker.place(
                    *mint,
                    event.venue,
                    OrderRequest::Buy {
                        spend: self.buy_spend,
                        min_tokens_out: 0,
                    },
                    self.meta(event.slot),
                    snap,
                );
                self.settle_now(order, event.slot);
                self.last_decisions.push(Decision {
                    event_signature: event.id.signature,
                    venue: event.venue,
                    kind: DecisionKind::PaperIntentBuy {
                        spend: self.buy_spend,
                    },
                    priority: crate::bus::Priority::Critical,
                });
            }
            EventPayload::TradeObserved {
                mint,
                is_buy,
                quote_amount,
                token_amount,
                ..
            } => {
                // Exit rule: first observed SELL of a held mint closes the
                // position against the PRE-trade tracked state.
                if !*is_buy {
                    if let Some(pos) = self.broker.position(mint) {
                        if pos.quantity > 0 {
                            let snap = self.snapshot_at(mint, event.slot);
                            let order = self.broker.place(
                                *mint,
                                pos.venue,
                                OrderRequest::Sell {
                                    tokens: pos.quantity,
                                    min_quote_out: Lamports(0),
                                },
                                self.meta(event.slot),
                                snap,
                            );
                            self.settle_now(order, event.slot);
                        }
                    }
                }
                // Track curve movement so later marks are realistic.
                let entry = self.curves.entry(*mint).or_insert(CurveSnapshot {
                    virtual_token_reserves: PUMP_LAUNCH_VIRTUAL_TOKEN_RESERVES,
                    virtual_quote_reserves: PUMP_LAUNCH_VIRTUAL_QUOTE_RESERVES,
                });
                if *is_buy {
                    entry.virtual_token_reserves = entry
                        .virtual_token_reserves
                        .saturating_sub(token_amount.amount as u64);
                    entry.virtual_quote_reserves =
                        entry.virtual_quote_reserves.saturating_add(quote_amount.0);
                } else {
                    entry.virtual_token_reserves = entry
                        .virtual_token_reserves
                        .saturating_add(token_amount.amount as u64);
                    entry.virtual_quote_reserves =
                        entry.virtual_quote_reserves.saturating_sub(quote_amount.0);
                }
            }
            EventPayload::MigrationObserved { mint, to } => {
                let snap = self.snapshot_at(mint, event.slot);
                self.broker.apply_migration(*mint, *to, snap);
            }
            _ => {}
        }
        self.last_decisions.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slippage_reduces_by_bps_and_never_underflows() {
        assert_eq!(apply_slippage_down(1_000_000, BasisPoints(25)), 997_500);
        assert_eq!(apply_slippage_down(1, BasisPoints(25)), 1);
        assert_eq!(apply_slippage_down(0, BasisPoints(25)), 0);
    }

    #[test]
    fn venue_strings_roundtrip() {
        for v in [
            Venue::PumpFun,
            Venue::PumpSwap,
            Venue::LetsBonk,
            Venue::StonkFun,
            Venue::RaydiumAmm,
            Venue::Unknown,
        ] {
            assert_eq!(venue_from(&venue_str(v)), v);
        }
    }
}
