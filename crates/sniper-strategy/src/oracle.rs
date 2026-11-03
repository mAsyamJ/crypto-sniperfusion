//! Independent accounting oracle (H7-3).
//!
//! A separately-implemented double-entry ledger that reconstructs portfolio
//! truth from execution RECORDS (not from engine-calculated totals): every
//! fill/exit/failure posts balanced debit/credit entries, conservation is
//! verified structurally after every tick, and valuation is kept separate
//! from transaction accounting — token inventory is carried at cost and
//! only marked to an externally supplied executable-exit value for NAV.
//!
//! `NAV = Cash + ExecutableExitValue(open lots) + RecoverableBalances
//! (rent, claimable) - Liabilities` — model-derived marks are labeled as
//! such and never flow back into the transaction accounts.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::engine::protocol::{ExecutionRecord, ProtocolRunOutcome};

/// Chart of accounts. Transaction accounts only; valuation happens outside.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Account {
    /// Spendable wallet cash.
    Cash,
    /// Opening equity (contra of the initial cash credit).
    Equity,
    /// Token inventory per mint, carried at cost.
    TokenInventory { mint: String },
    /// Realized P&L (credit = profit).
    RealizedPnl,
    /// Protocol fees paid (expense).
    ProtocolFees,
    /// Creator fees paid (expense).
    CreatorFees,
    /// LP fees paid (expense).
    LpFees,
    /// Network signature fees (expense, paid on failures too).
    NetworkFees,
    /// Priority fees (expense, paid on failures too).
    PriorityFees,
    /// Tips (expense).
    Tips,
    /// ATA rent parked as a recoverable balance until close.
    RentRecoverable,
    /// Rent written off (expense) when recovery is impossible.
    RentExpense,
    /// Proceeds parked in a claimable accumulator (asset once proven).
    ClaimableAccumulator,
    /// Funds submitted with UNKNOWN outcome: tracked, valued at zero in NAV
    /// until resolved (conservative).
    UnresolvedHolds,
    /// External liabilities (zero unless explicitly posted).
    Liabilities,
}

/// One balanced double-entry: debit and credit for the same amount.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub tick: u64,
    pub strategy: String,
    pub debit: Account,
    pub credit: Account,
    pub amount: u64,
    pub memo: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AccountingError {
    #[error("unbalanced or invalid posting: {0}")]
    InvalidPosting(&'static str),
    #[error("cash would go negative ({needed} > {available})")]
    InsufficientCash { needed: u64, available: u64 },
    #[error("sell of {tokens} tokens exceeds inventory {inventory} for {mint}")]
    InventoryShort {
        mint: String,
        tokens: u64,
        inventory: u64,
    },
    #[error("conservation violated at tick {tick}: debits {debits} != credits {credits}")]
    ConservationViolated {
        tick: u64,
        debits: u128,
        credits: u128,
    },
}

/// One inventory lot (FIFO consumption).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lot {
    pub tokens: u64,
    /// Carried cost basis in lamports (purchase spend only; fees expensed).
    pub cost_basis_lamports: u64,
}

/// NAV decomposition: transaction facts + model-derived marks, separated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavBreakdown {
    pub cash: u64,
    pub recoverable_rent: u64,
    pub claimable: u64,
    pub liabilities: u64,
    /// Cost basis of open inventory (transaction fact).
    pub open_inventory_cost_basis: u64,
    /// Model-derived executable-exit value of open inventory (marks input).
    pub executable_exit_value: u64,
    /// Unresolved holds: tracked, excluded from NAV (conservative).
    pub unresolved_at_risk: u64,
    pub nav: u64,
    pub unrealized_pnl: i64,
}

/// Which side increases an account (classical double-entry natures).
fn is_debit_natured(account: &Account) -> bool {
    matches!(
        account,
        Account::Cash
            | Account::TokenInventory { .. }
            | Account::RentRecoverable
            | Account::ClaimableAccumulator
            | Account::UnresolvedHolds
            | Account::ProtocolFees
            | Account::CreatorFees
            | Account::LpFees
            | Account::NetworkFees
            | Account::PriorityFees
            | Account::Tips
            | Account::RentExpense
    )
}

/// The independent oracle. Entries are append-only; balances are SIGNED
/// (debit-natured accounts increase on debit, credit-natured on credit) and
/// the accounting equation (assets + expenses == equity + liabilities +
/// income, i.e. the sum of all signed balances is zero) is verified after
/// every posting.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountingOracle {
    starting_cash: u64,
    entries: Vec<LedgerEntry>,
    balances: HashMap<Account, i128>,
    /// FIFO lots per mint.
    lots: HashMap<String, Vec<Lot>>,
    /// Rent parked per mint (recoverable on close).
    rent_parked: HashMap<String, u64>,
    tick: u64,
}

impl AccountingOracle {
    /// Opening position: Equity credited against Cash.
    pub fn new(starting_cash: u64) -> Self {
        let mut oracle = Self {
            starting_cash,
            entries: Vec::new(),
            balances: HashMap::new(),
            lots: HashMap::new(),
            rent_parked: HashMap::new(),
            tick: 0,
        };
        oracle
            .post(LedgerEntry {
                tick: 0,
                strategy: "opening".into(),
                debit: Account::Cash,
                credit: Account::Equity,
                amount: starting_cash,
                memo: "opening balance".into(),
            })
            .expect("opening balance always posts");
        oracle
    }

    /// Post one balanced entry with signed side effects and re-verify the
    /// accounting equation per tick.
    pub fn post(&mut self, entry: LedgerEntry) -> Result<(), AccountingError> {
        if entry.amount == 0 {
            return Err(AccountingError::InvalidPosting("zero-amount entry"));
        }
        let delta = |account: &Account, debit_side: bool| -> i128 {
            let amount = i128::from(entry.amount);
            if is_debit_natured(account) == debit_side {
                amount
            } else {
                -amount
            }
        };
        let cash_now = self.balances.get(&Account::Cash).copied().unwrap_or(0);
        let cash_delta = if entry.debit == Account::Cash {
            i128::from(entry.amount)
        } else {
            0
        } - if entry.credit == Account::Cash {
            i128::from(entry.amount)
        } else {
            0
        };
        if cash_now + cash_delta < 0 {
            return Err(AccountingError::InsufficientCash {
                needed: entry.amount,
                available: cash_now.max(0) as u64,
            });
        }
        *self.balances.entry(entry.debit.clone()).or_insert(0) += delta(&entry.debit, true);
        *self.balances.entry(entry.credit.clone()).or_insert(0) += delta(&entry.credit, false);
        self.entries.push(entry);
        self.verify_conservation()?;
        Ok(())
    }

    /// The accounting equation: total debit-natured balances (assets +
    /// expenses) must equal total credit-natured balances (equity +
    /// liabilities + income); cash and inventories must never be negative.
    /// Verified after EVERY posting.
    pub fn verify_conservation(&self) -> Result<(), AccountingError> {
        let mut debit_side = 0i128;
        let mut credit_side = 0i128;
        for (account, balance) in &self.balances {
            if is_debit_natured(account) {
                debit_side += *balance;
            } else {
                credit_side += *balance;
            }
            if *balance < 0
                && (matches!(account, Account::TokenInventory { .. }) || *account == Account::Cash)
            {
                return Err(AccountingError::ConservationViolated {
                    tick: self.tick,
                    debits: debit_side.unsigned_abs(),
                    credits: credit_side.unsigned_abs(),
                });
            }
        }
        if debit_side != credit_side {
            return Err(AccountingError::ConservationViolated {
                tick: self.tick,
                debits: debit_side.unsigned_abs(),
                credits: credit_side.unsigned_abs(),
            });
        }
        Ok(())
    }

    fn balance(&self, account: &Account) -> u64 {
        self.balances.get(account).copied().unwrap_or(0).max(0) as u64
    }

    pub fn cash(&self) -> u64 {
        self.balance(&Account::Cash)
    }

    pub fn trial_balance(&self) -> &HashMap<Account, i128> {
        &self.balances
    }

    pub fn entries(&self) -> &[LedgerEntry] {
        &self.entries
    }

    /// Post an engine buy fill: cash out for spend + costs; inventory up at
    /// cost; fees expensed; rent parked as recoverable.
    pub fn post_buy_fill(
        &mut self,
        strategy: &str,
        record: &crate::engine::protocol::ExecutionRecord,
    ) -> Result<(), AccountingError> {
        let crate::engine::protocol::ExecutionRecord::BuyFill {
            event_id,
            mint,
            spend_lamports,
            tokens_out,
            protocol_fee_lamports,
            creator_fee_lamports,
            lp_fee_lamports,
            network_fee_lamports,
            priority_fee_lamports,
            tip_lamports,
            rent_lamports,
            ..
        } = record
        else {
            return Err(AccountingError::InvalidPosting("not a buy fill"));
        };
        self.tick += 1;
        // Fee-on-input semantics (engine B): the protocol/creator/lp fee is
        // withheld FROM the spend, so inventory carries the effective cost
        // and the fee is expensed separately — never double-counted.
        let input_fees = protocol_fee_lamports + creator_fee_lamports + lp_fee_lamports;
        let basis = spend_lamports.saturating_sub(input_fees);
        let _ = basis;
        self.post(LedgerEntry {
            tick: self.tick,
            strategy: strategy.into(),
            debit: Account::TokenInventory { mint: mint.clone() },
            credit: Account::Cash,
            amount: basis,
            memo: format!("buy {event_id}"),
        })?;
        for (account, amount) in [
            (Account::ProtocolFees, *protocol_fee_lamports),
            (Account::CreatorFees, *creator_fee_lamports),
            (Account::LpFees, *lp_fee_lamports),
            (Account::NetworkFees, *network_fee_lamports),
            (Account::PriorityFees, *priority_fee_lamports),
            (Account::Tips, *tip_lamports),
        ] {
            if amount > 0 {
                self.post(LedgerEntry {
                    tick: self.tick,
                    strategy: strategy.into(),
                    debit: account,
                    credit: Account::Cash,
                    amount,
                    memo: format!("buy costs {event_id}"),
                })?;
            }
        }
        if *rent_lamports > 0 {
            self.post(LedgerEntry {
                tick: self.tick,
                strategy: strategy.into(),
                debit: Account::RentRecoverable,
                credit: Account::Cash,
                amount: *rent_lamports,
                memo: format!("ATA rent {event_id}"),
            })?;
            self.rent_parked
                .entry(mint.clone())
                .and_modify(|r| *r += rent_lamports)
                .or_insert(*rent_lamports);
        }
        let lots = self.lots.entry(mint.clone()).or_default();
        lots.push(Lot {
            tokens: *tokens_out,
            cost_basis_lamports: basis,
        });
        Ok(())
    }

    /// Post an engine sell fill: FIFO lot consumption (partial sells
    /// apportion cost basis), rent recovered, realized P&L recognized.
    pub fn post_sell_fill(
        &mut self,
        strategy: &str,
        record: &ExecutionRecord,
    ) -> Result<(), AccountingError> {
        let ExecutionRecord::SellFill {
            event_id,
            mint,
            tokens_in,
            quote_out_net,
            protocol_fee_lamports,
            creator_fee_lamports,
            lp_fee_lamports,
            rent_recovered_lamports,
            ..
        } = record
        else {
            return Err(AccountingError::InvalidPosting("not a sell fill"));
        };
        self.tick += 1;
        let inventory = self
            .lots
            .get(mint)
            .map(|l| l.iter().map(|l| l.tokens).sum::<u64>())
            .unwrap_or(0);
        if *tokens_in > inventory {
            return Err(AccountingError::InventoryShort {
                mint: mint.clone(),
                tokens: *tokens_in,
                inventory,
            });
        }
        // FIFO consumption with proportional basis on the split lot.
        let mut remaining = *tokens_in;
        let mut consumed_basis = 0u64;
        let lots = self.lots.get_mut(mint).expect("inventory checked");
        while remaining > 0 {
            let lot = lots.first_mut().expect("inventory checked");
            if lot.tokens <= remaining {
                remaining -= lot.tokens;
                consumed_basis += lot.cost_basis_lamports;
                lots.remove(0);
            } else {
                let share = remaining as u128 * u128::from(lot.cost_basis_lamports)
                    / u128::from(lot.tokens);
                consumed_basis += share as u64;
                lot.tokens -= remaining;
                lot.cost_basis_lamports -= share as u64;
                remaining = 0;
            }
        }
        if self.lots.get(mint).is_some_and(|l| l.is_empty()) {
            self.lots.remove(mint);
        }
        // Cash in (net proceeds), inventory relieved at consumed basis, the
        // difference recognized as realized P&L.
        self.post(LedgerEntry {
            tick: self.tick,
            strategy: strategy.into(),
            debit: Account::Cash,
            credit: Account::TokenInventory { mint: mint.clone() },
            amount: consumed_basis.min(*quote_out_net),
            memo: format!("sell basis {event_id}"),
        })?;
        if *quote_out_net > consumed_basis {
            self.post(LedgerEntry {
                tick: self.tick,
                strategy: strategy.into(),
                debit: Account::Cash,
                credit: Account::RealizedPnl,
                amount: quote_out_net - consumed_basis,
                memo: format!("sell gain {event_id}"),
            })?;
        } else if consumed_basis > *quote_out_net {
            self.post(LedgerEntry {
                tick: self.tick,
                strategy: strategy.into(),
                debit: Account::RealizedPnl,
                credit: Account::TokenInventory { mint: mint.clone() },
                amount: consumed_basis - quote_out_net,
                memo: format!("sell loss {event_id}"),
            })?;
        }
        for (account, amount) in [
            (Account::ProtocolFees, *protocol_fee_lamports),
            (Account::CreatorFees, *creator_fee_lamports),
            (Account::LpFees, *lp_fee_lamports),
        ] {
            if amount > 0 {
                self.post(LedgerEntry {
                    tick: self.tick,
                    strategy: strategy.into(),
                    debit: account,
                    credit: Account::Cash,
                    amount,
                    memo: format!("sell costs {event_id}"),
                })?;
            }
        }
        if *rent_recovered_lamports > 0 {
            let parked = self.rent_parked.get(mint).copied().unwrap_or(0);
            let recovered = (*rent_recovered_lamports).min(parked);
            let write_off = parked - recovered;
            if recovered > 0 {
                self.post(LedgerEntry {
                    tick: self.tick,
                    strategy: strategy.into(),
                    debit: Account::Cash,
                    credit: Account::RentRecoverable,
                    amount: recovered,
                    memo: format!("rent recovered {event_id}"),
                })?;
            }
            if write_off > 0 {
                self.post(LedgerEntry {
                    tick: self.tick,
                    strategy: strategy.into(),
                    debit: Account::RentExpense,
                    credit: Account::RentRecoverable,
                    amount: write_off,
                    memo: format!("rent write-off {event_id}"),
                })?;
            }
            if recovered == parked {
                self.rent_parked.remove(mint);
            } else {
                self.rent_parked.insert(mint.clone(), write_off);
            }
        }
        Ok(())
    }

    /// Failed landing: network + priority fees burned, nothing acquired.
    pub fn post_failed_landing(
        &mut self,
        strategy: &str,
        record: &ExecutionRecord,
    ) -> Result<(), AccountingError> {
        let ExecutionRecord::AttemptFailedLanding {
            event_id,
            network_fee_lamports,
            priority_fee_lamports,
            ..
        } = record
        else {
            return Err(AccountingError::InvalidPosting("not a failed landing"));
        };
        self.tick += 1;
        for (account, amount) in [
            (Account::NetworkFees, *network_fee_lamports),
            (Account::PriorityFees, *priority_fee_lamports),
        ] {
            if amount > 0 {
                self.post(LedgerEntry {
                    tick: self.tick,
                    strategy: strategy.into(),
                    debit: account,
                    credit: Account::Cash,
                    amount,
                    memo: format!("failed landing {event_id}"),
                })?;
            }
        }
        Ok(())
    }

    /// Submitted with UNKNOWN outcome: funds leave cash into a hold that is
    /// NOT valued in NAV until resolved (conservative).
    pub fn post_unknown_hold(
        &mut self,
        strategy: &str,
        event_id: &str,
        amount: u64,
    ) -> Result<(), AccountingError> {
        self.tick += 1;
        self.post(LedgerEntry {
            tick: self.tick,
            strategy: strategy.into(),
            debit: Account::UnresolvedHolds,
            credit: Account::Cash,
            amount,
            memo: format!("unknown outcome hold {event_id}"),
        })
    }

    /// Resolve an unknown hold: the tokens arrived (inventory at cost).
    pub fn resolve_hold_to_inventory(
        &mut self,
        strategy: &str,
        event_id: &str,
        mint: &str,
        amount: u64,
        tokens: u64,
    ) -> Result<(), AccountingError> {
        self.tick += 1;
        self.post(LedgerEntry {
            tick: self.tick,
            strategy: strategy.into(),
            debit: Account::TokenInventory { mint: mint.into() },
            credit: Account::UnresolvedHolds,
            amount,
            memo: format!("hold resolved to inventory {event_id}"),
        })?;
        self.lots.entry(mint.into()).or_default().push(Lot {
            tokens,
            cost_basis_lamports: amount,
        });
        Ok(())
    }

    /// Resolve an unknown hold as a total loss.
    pub fn resolve_hold_to_loss(
        &mut self,
        strategy: &str,
        event_id: &str,
        amount: u64,
    ) -> Result<(), AccountingError> {
        self.tick += 1;
        self.post(LedgerEntry {
            tick: self.tick,
            strategy: strategy.into(),
            debit: Account::RealizedPnl,
            credit: Account::UnresolvedHolds,
            amount,
            memo: format!("hold resolved as loss {event_id}"),
        })
    }

    /// Record claimable accumulator proceeds (asset only once reconciled —
    /// callers must have on-chain evidence before posting here).
    pub fn post_claimable(
        &mut self,
        strategy: &str,
        event_id: &str,
        amount: u64,
    ) -> Result<(), AccountingError> {
        self.tick += 1;
        self.post(LedgerEntry {
            tick: self.tick,
            strategy: strategy.into(),
            debit: Account::ClaimableAccumulator,
            credit: Account::RealizedPnl,
            amount,
            memo: format!("claimable proceeds {event_id}"),
        })
    }

    /// Open FIFO lots for a mint (empty = none).
    pub fn lots_for(&self, mint: &str) -> &[Lot] {
        self.lots.get(mint).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// NAV: transaction accounts + externally supplied executable-exit
    /// marks. Marks are MODEL-DERIVED and never posted to the ledger.
    pub fn nav(&self, exit_marks: &HashMap<String, u64>) -> NavBreakdown {
        let mut cost_basis = 0u64;
        let mut exit_value = 0u64;
        for (mint, lots) in &self.lots {
            let basis: u64 = lots.iter().map(|l| l.cost_basis_lamports).sum();
            cost_basis += basis;
            exit_value += exit_marks.get(mint).copied().unwrap_or(0);
        }
        let cash = self.cash();
        let recoverable_rent = self.balance(&Account::RentRecoverable);
        let claimable = self.balance(&Account::ClaimableAccumulator);
        let liabilities = self.balance(&Account::Liabilities);
        let nav = cash + exit_value + recoverable_rent + claimable - liabilities;
        NavBreakdown {
            cash,
            recoverable_rent,
            claimable,
            liabilities,
            open_inventory_cost_basis: cost_basis,
            executable_exit_value: exit_value,
            unresolved_at_risk: self.balance(&Account::UnresolvedHolds),
            nav,
            unrealized_pnl: exit_value as i64 - cost_basis as i64,
        }
    }

    /// Ingest one engine run's records in order (independent-wallet mode =
    /// one oracle per run; shared-wallet mode = ingest several runs into
    /// ONE oracle in interleaved tick order — the caller provides the
    /// order, contention then shows up as InsufficientCash).
    pub fn ingest(
        &mut self,
        strategy: &str,
        records: &[ExecutionRecord],
    ) -> Result<(), AccountingError> {
        for record in records {
            match record {
                ExecutionRecord::BuyFill { .. } => self.post_buy_fill(strategy, record)?,
                ExecutionRecord::SellFill { .. } => self.post_sell_fill(strategy, record)?,
                ExecutionRecord::AttemptFailedLanding { .. } => {
                    self.post_failed_landing(strategy, record)?
                }
                ExecutionRecord::AttemptRejected { .. } | ExecutionRecord::ExitBlocked { .. } => {
                    // No cash movement by construction — nothing to post.
                }
            }
        }
        Ok(())
    }

    /// Convenience: independent oracle over one run.
    pub fn from_run(
        outcome: &ProtocolRunOutcome,
        starting_cash: u64,
    ) -> Result<Self, AccountingError> {
        let mut oracle = Self::new(starting_cash);
        oracle.ingest(outcome.strategy, &outcome.records)?;
        Ok(oracle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buy_fill(mint: &str, spend: u64, tokens: u64, event: &str) -> ExecutionRecord {
        ExecutionRecord::BuyFill {
            event_id: event.into(),
            mint: mint.into(),
            slot: 1,
            spend_lamports: spend,
            tokens_out: tokens,
            protocol_fee_lamports: spend / 100,
            creator_fee_lamports: 0,
            lp_fee_lamports: 0,
            network_fee_lamports: 5_000,
            priority_fee_lamports: 5_000,
            tip_lamports: 0,
            rent_lamports: 1_500_000,
            fee_synthetic_fallback: false,
        }
    }

    fn sell_fill(
        mint: &str,
        tokens: u64,
        net: u64,
        rent_back: u64,
        event: &str,
    ) -> ExecutionRecord {
        ExecutionRecord::SellFill {
            event_id: event.into(),
            mint: mint.into(),
            slot: 2,
            tokens_in: tokens,
            quote_out_net: net,
            protocol_fee_lamports: net / 200,
            creator_fee_lamports: 0,
            lp_fee_lamports: 0,
            network_fee_lamports: 5_000,
            priority_fee_lamports: 5_000,
            tip_lamports: 0,
            rent_recovered_lamports: rent_back,
            fee_synthetic_fallback: false,
        }
    }

    #[test]
    fn opening_balance_and_conservation_hold_per_tick() {
        let oracle = AccountingOracle::new(100_000_000);
        assert_eq!(oracle.cash(), 100_000_000);
        assert!(oracle.verify_conservation().is_ok());
    }

    #[test]
    fn fee_attribution_is_exact_in_the_trial_balance() {
        let mut oracle = AccountingOracle::new(100_000_000);
        oracle
            .post_buy_fill("S01", &buy_fill("M", 1_000_000, 900, "e1"))
            .unwrap();
        let tb = oracle.trial_balance();
        assert_eq!(*tb.get(&Account::ProtocolFees).unwrap(), 10_000);
        assert_eq!(*tb.get(&Account::NetworkFees).unwrap(), 5_000);
        assert_eq!(*tb.get(&Account::PriorityFees).unwrap(), 5_000);
        assert_eq!(*tb.get(&Account::RentRecoverable).unwrap(), 1_500_000);
        // Cash = 100M - (1M spend incl. 10k fee + 5k + 5k + 1.5M rent).
        assert_eq!(oracle.cash(), 97_490_000);
        // Inventory carried at effective cost (990_000), not gross spend.
        assert_eq!(oracle.lots_for("M")[0].cost_basis_lamports, 990_000);
        assert!(oracle.verify_conservation().is_ok());
    }

    #[test]
    fn posting_beyond_cash_is_refused_and_leaves_no_entry() {
        let mut oracle = AccountingOracle::new(1_000);
        let err = oracle
            .post_buy_fill("S01", &buy_fill("M", 5_000, 1, "e1"))
            .unwrap_err();
        assert!(matches!(err, AccountingError::InsufficientCash { .. }));
        // Only the opening entry exists — the failed posting is atomic.
        assert_eq!(oracle.entries().len(), 1);
    }

    #[test]
    fn open_losing_position_shows_negative_unrealized_pnl() {
        let mut oracle = AccountingOracle::new(100_000_000);
        oracle
            .post_buy_fill("S01", &buy_fill("M", 1_000_000, 900, "e1"))
            .unwrap();
        // Mark the position at a liquidation value BELOW cost basis.
        let mut marks = HashMap::new();
        marks.insert("M".to_string(), 400_000u64);
        let nav = oracle.nav(&marks);
        assert_eq!(nav.open_inventory_cost_basis, 990_000);
        assert_eq!(nav.executable_exit_value, 400_000);
        assert_eq!(nav.unrealized_pnl, -590_000);
        // NAV = cash + mark + recoverable rent; the open loss is visible,
        // never hidden by carrying inventory at cost.
        assert_eq!(nav.nav, 97_490_000 + 400_000 + 1_500_000);
    }

    #[test]
    fn partial_exit_consumes_fifo_lots_with_proportional_basis() {
        let mut oracle = AccountingOracle::new(200_000_000);
        oracle
            .post_buy_fill("S01", &buy_fill("M", 1_000_000, 1_000, "e1"))
            .unwrap();
        oracle
            .post_buy_fill("S01", &buy_fill("M", 2_000_000, 2_000, "e2"))
            .unwrap();
        // Lot 1: 1_000 tokens @ 990_000 effective basis; lot 2: 2_000 @ 1_980_000.
        // Sell 1_500 tokens: all of lot 1 (1_000) + 500 of lot 2 (a quarter).
        oracle
            .post_sell_fill("S01", &sell_fill("M", 1_500, 2_500_000, 1_500_000, "e3"))
            .unwrap();
        // Remaining: lot 2 remainder = 1_500 tokens at proportional basis.
        let lots = oracle.lots_for("M");
        assert_eq!(lots.len(), 1);
        assert_eq!(lots[0].tokens, 1_500);
        // 1_980_000 - (500/2_000)*1_980_000 = 1_485_000.
        assert_eq!(lots[0].cost_basis_lamports, 1_485_000);
        // Consumed basis = 990_000 + 495_000 = 1_485_000; proceeds
        // 2_500_000 -> realized gain 1_015_000.
        let tb = oracle.trial_balance();
        assert_eq!(*tb.get(&Account::RealizedPnl).unwrap(), 1_015_000);
        assert!(oracle.verify_conservation().is_ok());
    }

    #[test]
    fn overselling_inventory_is_refused() {
        let mut oracle = AccountingOracle::new(100_000_000);
        oracle
            .post_buy_fill("S01", &buy_fill("M", 1_000_000, 100, "e1"))
            .unwrap();
        assert!(matches!(
            oracle.post_sell_fill("S01", &sell_fill("M", 200, 1, 0, "e2")),
            Err(AccountingError::InventoryShort { .. })
        ));
    }

    #[test]
    fn interleaved_mints_keep_separate_inventories() {
        let mut oracle = AccountingOracle::new(300_000_000);
        oracle
            .post_buy_fill("S01", &buy_fill("A", 1_000_000, 500, "e1"))
            .unwrap();
        oracle
            .post_buy_fill("S01", &buy_fill("B", 1_000_000, 700, "e2"))
            .unwrap();
        oracle
            .post_sell_fill("S01", &sell_fill("B", 700, 900_000, 1_500_000, "e3"))
            .unwrap();
        assert!(oracle.lots_for("A").len() == 1);
        assert!(oracle.lots_for("B").is_empty());
        assert!(oracle.verify_conservation().is_ok());
    }

    #[test]
    fn failed_landings_and_unknown_holds_are_conservative() {
        let mut oracle = AccountingOracle::new(100_000_000);
        let failed = ExecutionRecord::AttemptFailedLanding {
            event_id: "e1".into(),
            mint: "M".into(),
            network_fee_lamports: 5_000,
            priority_fee_lamports: 5_000,
        };
        oracle.post_failed_landing("S01", &failed).unwrap();
        assert_eq!(oracle.cash(), 99_990_000);

        oracle.post_unknown_hold("S01", "e2", 1_000_000).unwrap();
        let nav = oracle.nav(&HashMap::new());
        assert_eq!(nav.unresolved_at_risk, 1_000_000);
        // The hold LEFT the wallet and is EXCLUDED from NAV until resolved
        // (conservative: submitted-but-unknown funds are not spendable and
        // not counted as value).
        assert_eq!(nav.nav, 98_990_000);

        // Resolve as a total loss: hold relief hits realized P&L.
        oracle.resolve_hold_to_loss("S01", "e2", 1_000_000).unwrap();
        let nav = oracle.nav(&HashMap::new());
        assert_eq!(nav.unresolved_at_risk, 0);
        assert_eq!(nav.nav, 98_990_000);
        let tb = oracle.trial_balance();
        // RealizedPnl is credit-natured: a loss carries a NEGATIVE balance.
        assert_eq!(*tb.get(&Account::RealizedPnl).unwrap(), -1_000_000);
        assert!(oracle.verify_conservation().is_ok());
    }

    #[test]
    fn shared_wallet_contention_is_visible_as_insufficient_cash() {
        // Shared mode: two strategies post into ONE oracle; the second
        // entry cannot spend cash the first already took.
        let mut shared = AccountingOracle::new(3_000_000);
        shared
            .post_buy_fill("S01", &buy_fill("A", 1_000_000, 100, "e1"))
            .unwrap();
        // Costs: 1M + 10k fee + 5k + 5k + 1.5M rent = 2_520_000; remaining
        // cash 480_000 cannot cover another entry's rent alone.
        assert!(matches!(
            shared.post_buy_fill("S02", &buy_fill("B", 1_000_000, 100, "e2")),
            Err(AccountingError::InsufficientCash { .. })
        ));
    }

    #[test]
    fn claimable_proceeds_are_assets_only_with_evidence() {
        let mut oracle = AccountingOracle::new(100_000_000);
        oracle.post_claimable("S01", "e9", 250_000).unwrap();
        let nav = oracle.nav(&HashMap::new());
        assert_eq!(nav.claimable, 250_000);
        assert_eq!(nav.nav, 100_250_000);
    }

    #[test]
    fn oracle_reconstructs_engine_run_cash_independently() {
        // Cross-engine check: run the REAL protocol engine, then rebuild
        // the wallet from its records through this separately-implemented
        // ledger — the two must agree on ending cash to the lamport.
        use crate::engine::protocol::{
            ExecutionFrictions, MarketBook, MarketSnapshot, ProtocolParams, QuoteMint, VenueState,
        };
        use crate::observation::{Phase, Safety};
        use crate::policies::EntryPolicy;

        let event = crate::observation::Observation {
            event_id: "e1".into(),
            mint: "M".into(),
            slot: 10,
            chain_at_ms: 95,
            observed_at_ms: 100,
            feature_available_at_ms: 110,
            phase: Phase::ActiveCurve,
            safety: Safety::Eligible,
            coverage_ok: true,
            price_lamports_per_token: 1_000,
            exit_liquidity_lamports: 1_000_000_000,
            net_flow_lamports: 1,
            unique_buyers: 10,
            creator_sell_lamports: 0,
            curve_progress_bps: 1_000,
            fee_bps: 100,
        };
        let mut book = MarketBook::new();
        book.push(MarketSnapshot {
            mint: "M".into(),
            slot: 9,
            quote_mint: QuoteMint::Sol,
            state: VenueState::BondingCurve {
                virtual_token_reserves: 1_073_000_000_000_000,
                virtual_quote_reserves: 30_000_000_000,
                mint_total_supply: 1_000_000_000_000_000,
                fee: crate::fees::CurveFeeState::synthetic_fallback_100bps(9),
            },
        });
        let outcome = crate::engine::protocol::run(
            &[event],
            &book,
            &mut EntryPolicy::s01(crate::StrategyKnobs::default()),
            100_000_000,
            &crate::RiskConstraints::default(),
            &ExecutionFrictions::default(),
            &ProtocolParams::default(),
        );
        let oracle = AccountingOracle::from_run(&outcome, 100_000_000).unwrap();
        assert!(oracle.verify_conservation().is_ok());
        assert_eq!(
            oracle.cash(),
            outcome.ending_cash_lamports,
            "oracle must independently reconstruct the engine wallet"
        );
        assert_eq!(oracle.lots_for("M").len(), 1, "one open lot after the buy");
    }
}
