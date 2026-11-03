//! A deliberately separate, strictly DEVNET-ONLY Pump.fun protocol pilot.
//!
//! This is NOT the M7 live trading path, nor an automatic sniping system.
//! It refuses mainnet/testnet/custom genesis hashes; uses a dedicated keypair
//! created by this tool; never loads the user's default Solana CLI identity.
//! An explicit --submit is required for any buy/sell broadcast. It can do
//! at most one buy and one sell, with no retries and a 0.005 SOL buy cap.
//! No profitability assertions: cashback/accumulator balances are unresolved.
//!
//! Example:
//! cargo run -p sniper-app --bin devnet-pump-pilot -- --mint <DEVNET_MINT>
//! cargo run -p sniper-app --bin devnet-pump-pilot -- --mint <DEVNET_MINT> --submit --roundtrip

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use clap::Parser;
use serde_json::{json, Value};
use sniper_protocols::builders::{buy_v3, sell_v3, CurveTradeContext};
use sniper_protocols::pda;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, write_keypair_file, Keypair, Signer};
use solana_sdk::transaction::Transaction;

const DEVNET_GENESIS: &str = "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG";
const MAX_BUY_LAMPORTS: u64 = 5_000_000; // faucet SOL, at most 0.005 SOL
const MAX_TOKEN_BASE_UNITS: u64 = 1_000_000;
const FUND_AIRDROP_LAMPORTS: u64 = 30_000_000; // 0.03 devnet SOL
const CU_LIMIT: u32 = 400_000;
const DEFAULT_RPC: &str = "https://api.devnet.solana.com";
const DEFAULT_KEY: &str = "harness/devnet-keys/pilot.json";

#[derive(Debug, Parser)]
#[command(name = "devnet-pump-pilot", about = "Opt-in Pump.fun V3 Devnet smoke, never mainnet")]
struct Args {
    /// Address of a live CURVE-TRADING SOL-paired Token-2022 mint on Devnet.
    #[arg(long)]
    mint: String,
    /// Devnet RPC. Cluster identity is checked using getGenesisHash.
    #[arg(long, env = "SNF_DEVNET_RPC", default_value = DEFAULT_RPC)]
    rpc_url: String,
    /// Target token base units (not UI tokens).
    #[arg(long, default_value_t = 100_000)]
    tokens: u64,
    /// Maximum total curve SOL cost of the buy (excludes ATA rent/tx fee).
    #[arg(long, default_value_t = 3_000_000)]
    max_sol_cost_lamports: u64,
    /// Minimum nominal SOL quote on sell; still not a wallet-equity guarantee.
    #[arg(long, default_value_t = 1)]
    min_sell_quote_lamports: u64,
    /// Explicitly permit a bounded single buy on DEVNET after simulation.
    #[arg(long)]
    submit: bool,
    /// After a successful buy, attempt one sale of exactly received tokens.
    #[arg(long)]
    roundtrip: bool,
    /// Separate dedicated local devnet-only identity. Never point to funded mainnet key.
    #[arg(long, default_value = DEFAULT_KEY)]
    pilot_keypair: PathBuf,
}

fn guard(genesis: &str, tokens: u64, max_cost: u64, min_sell: u64) -> Result<(), String> {
    if genesis != DEVNET_GENESIS {
        return Err("DEVNET_ONLY: RPC genesis is not Solana Devnet".into());
    }
    if tokens == 0 || tokens > MAX_TOKEN_BASE_UNITS {
        return Err("token quantity out of fixed pilot bounds".into());
    }
    if max_cost == 0 || max_cost > MAX_BUY_LAMPORTS {
        return Err("buy cap exceeds hard 0.005 SOL Devnet pilot limit".into());
    }
    if min_sell == 0 {
        return Err("zero minimum sell quote refused".into());
    }
    Ok(())
}

fn must_be_offline_mainnet() -> Result<(), String> {
    if std::env::var("LIVE_TRADING_ENABLED")
        .unwrap_or_default()
        .to_ascii_lowercase()
        == "true"
    {
        return Err("pilot requires LIVE_TRADING_ENABLED=false".into());
    }
    if std::env::var_os("SOLANA_PRIVATE_KEY").is_some()
        || std::env::var_os("PRIVATE_KEY").is_some()
    {
        return Err("pilot refuses private-key env variables".into());
    }
    Ok(())
}

fn local_pilot_key(path: &Path) -> Result<Keypair, String> {
    // This tool owns only a dedicated location; never the normal Solana CLI key.
    if path.components().any(|part| {
        matches!(part, std::path::Component::ParentDir)
    }) || !path.starts_with("harness/devnet-keys")
    {
        return Err("pilot key path must be under harness/devnet-keys/".into());
    }
    if path.exists() {
        return read_keypair_file(path).map_err(|_| "cannot read local pilot key".into());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| "cannot create pilot key directory")?;
    }
    let key = Keypair::new();
    write_keypair_file(&key, path).map_err(|_| "cannot store local pilot key")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|_| "cannot secure pilot key file")?;
    }
    Ok(key)
}

fn create_ata_idempotent(user: &Pubkey, mint: &Pubkey) -> Instruction {
    let token_program = pda::token_2022_program();
    Instruction {
        program_id: pda::associated_token_program(),
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(pda::spl_associated(user, &token_program, mint), false),
            AccountMeta::new_readonly(*user, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(pda::system_program(), false),
            AccountMeta::new_readonly(token_program, false),
        ],
        data: vec![1],
    }
}

fn transaction(wallet: &Keypair, rpc: &RpcClient, ixs: Vec<Instruction>) -> Result<Transaction, String> {
    let mut all = vec![ComputeBudgetInstruction::set_compute_unit_limit(CU_LIMIT)];
    all.extend(ixs);
    let msg = Message::new(&all, Some(&wallet.pubkey()));
    let hash = rpc.get_latest_blockhash().map_err(|_| "cannot obtain fresh blockhash")?;
    Ok(Transaction::new(&[wallet], msg, hash))
}

fn token_balance(rpc: &RpcClient, ata: &Pubkey) -> Result<u64, String> {
    match rpc.get_token_account_balance(ata) {
        Ok(b) => b.amount.parse::<u64>().map_err(|_| "invalid token balance".into()),
        Err(_) => Ok(0), // ATA absent before first buy
    }
}

fn simulate(rpc: &RpcClient, tx: &Transaction) -> Result<(), String> {
    let sim = rpc.simulate_transaction(tx).map_err(|_| "simulation RPC call failed")?;
    if sim.value.err.is_some() {
        return Err("protocol transaction simulation failed; no trade submitted".into());
    }
    Ok(())
}

fn send_once(rpc: &RpcClient, tx: &Transaction) -> Result<String, String> {
    rpc.send_and_confirm_transaction(tx)
        .map(|sig| sig.to_string())
        .map_err(|_| "submit/confirm uncertain: inspect chain before any retry".into())
}

/// A durable, single-run journal for any signing attempt. An existing file
/// blocks another buy from the same dedicated pilot identity until the
/// operator manually reconciles it; no blind resubmissions.
fn start_journal(user: &Pubkey, mint: &Pubkey) -> Result<PathBuf, String> {
    let dir = Path::new("harness/runs/devnet-pilot");
    std::fs::create_dir_all(dir).map_err(|_| "cannot create pilot journal dir")?;
    let path = dir.join(format!("{user}.jsonl"));
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| "pilot journal already exists; reconcile previous order before retry")?;
    let line = json!({
        "state": "BUY_INTENT_DURABLE", "cluster": "devnet",
        "wallet": user.to_string(), "mint": mint.to_string(),
    });
    writeln!(f, "{line}").map_err(|_| "pilot journal write failed")?;
    f.sync_all().map_err(|_| "pilot journal sync failed")?;
    Ok(path)
}

fn journal(path: &Path, state: &str, signature: Option<&str>) -> Result<(), String> {
    let mut f = OpenOptions::new().append(true).open(path)
        .map_err(|_| "cannot reopen pilot journal")?;
    writeln!(f, "{}", json!({"state": state, "signature": signature}))
        .map_err(|_| "journal append failed")?;
    f.sync_all().map_err(|_| "journal fsync failed")?;
    Ok(())
}

fn run(args: Args) -> Result<Value, String> {
    must_be_offline_mainnet()?;
    if !args.rpc_url.starts_with("https://")
        || args.rpc_url.contains('@')
        || args.rpc_url.contains('#')
    {
        return Err("RPC URL must be HTTPS with no credentials/userinfo or fragment".into());
    }
    if args.roundtrip && !args.submit {
        return Err("--roundtrip requires --submit".into());
    }
    if args.pilot_keypair == Path::new(DEFAULT_KEY)
        || args.pilot_keypair.starts_with("harness/devnet-keys/")
    {
        // dedicated path only, enforced again in local_pilot_key
    } else {
        return Err("refusing non-pilot keypair".into());
    }
    let mint = Pubkey::from_str(&args.mint).map_err(|_| "invalid mint pubkey")?;
    let rpc = RpcClient::new_with_commitment(args.rpc_url, CommitmentConfig::confirmed());
    let genesis = rpc.get_genesis_hash().map_err(|_| "cannot verify network genesis")?;
    guard(
        &genesis.to_string(),
        args.tokens,
        args.max_sol_cost_lamports,
        args.min_sell_quote_lamports,
    )?;
    let pump = rpc.get_account(&pda::pump_program())
        .map_err(|_| "Pump.fun program absent on this cluster")?;
    if !pump.executable {
        return Err("Pump.fun account is not executable".into());
    }
    let token = rpc.get_account(&mint).map_err(|_| "devnet token mint not found")?;
    if token.owner != pda::token_2022_program() {
        return Err("pilot supports SOL-paired Token-2022 mints only".into());
    }
    let curve = pda::bonding_curve(&mint);
    let bc = rpc.get_account(&curve).map_err(|_| "bonding curve missing")?;
    if bc.owner != pda::pump_program() {
        return Err("curve not owned by expected Pump.fun program".into());
    }

    if !args.submit {
        return Ok(json!({
            "evidence_class":"DEVNET_READ_ONLY_PRECHECK",
            "genesis":genesis.to_string(), "mint":mint.to_string(),
            "pump_program_executable":true, "curve_owner_verified":true,
            "submitted":false,
            "limitations":"No pricing, signing, landing or profitability proof"
        }));
    }

    // Local dedicated identity is persisted before any send: failed sells can
    // be reconciled manually. Never use a user's mainnet CLI identity.
    let key = local_pilot_key(&args.pilot_keypair)?;
    let starting_balance = rpc.get_balance(&key.pubkey())
        .map_err(|_| "cannot read pilot balance")?;
    if starting_balance < FUND_AIRDROP_LAMPORTS {
        let sig = rpc.request_airdrop(&key.pubkey(), FUND_AIRDROP_LAMPORTS)
            .map_err(|_| "devnet faucet unavailable")?;
        if !rpc.confirm_transaction(&sig).map_err(|_| "airdrop confirmation failed")? {
            return Err("airdrop unconfirmed; no trade".into());
        }
    }
    let context = CurveTradeContext::sol_paired(key.pubkey(), mint);
    let ata = pda::spl_associated(&key.pubkey(), &pda::token_2022_program(), &mint);
    let before = token_balance(&rpc, &ata)?;

    let buy = transaction(
        &key, &rpc,
        vec![create_ata_idempotent(&key.pubkey(), &mint),
             buy_v3(&context, args.tokens, args.max_sol_cost_lamports, 0, false)],
    )?;
    simulate(&rpc, &buy)?;
    let journal_path = start_journal(&key.pubkey(), &mint)?;
    let buy_signature = match send_once(&rpc, &buy) {
        Ok(sig) => {
            journal(&journal_path, "BUY_CONFIRMED", Some(&sig))?;
            sig
        }
        Err(message) => {
            journal(&journal_path, "BUY_UNKNOWN_RECONCILE", None)?;
            return Err(message);
        }
    };
    let after = token_balance(&rpc, &ata)?;
    let filled = after.checked_sub(before).ok_or("token balance unexpectedly decreased")?;
    if filled == 0 {
        return Err("buy confirmation returned but no token delta; reconcile manually".into());
    }
    let mut receipt = json!({
        "evidence_class":"DEVNET_PROGRAM_TRANSACTION",
        "genesis":genesis.to_string(),
        "mint":mint.to_string(),
        "wallet_pubkey":key.pubkey().to_string(),
        "pilot_key_path":args.pilot_keypair.to_string_lossy(),
        "journal_path":journal_path.to_string_lossy(),
        "submitted":true,
        "buy_signature":buy_signature,
        "tokens_received":filled,
        "sell_signature":null,
        "roundtrip_requested":args.roundtrip,
        "profitability_claim":false,
        "warning":"claimable volume accumulator, fees and net account deltas need independent reconciliation"
    });

    if args.roundtrip {
        let sell = transaction(
            &key, &rpc,
            vec![sell_v3(&context, filled, args.min_sell_quote_lamports, 0)],
        )?;
        if let Err(message) = simulate(&rpc, &sell) {
            // Preserve evidence of the successful buy. No unverified retry.
            receipt["sell_status"] = json!("SIMULATION_REJECTED");
            receipt["sell_error"] = json!(message);
            journal(&journal_path, "SELL_SIMULATION_REJECTED", None)?;
            return Ok(receipt);
        }
        match send_once(&rpc, &sell) {
            Ok(sig) => {
                journal(&journal_path, "SELL_CONFIRMED", Some(&sig))?;
                receipt["sell_signature"] = json!(sig);
                receipt["remaining_tokens"] = json!(token_balance(&rpc, &ata)?);
            }
            Err(message) => {
                journal(&journal_path, "SELL_UNKNOWN_RECONCILE", None)?;
                receipt["sell_status"] = json!("SUBMISSION_UNCERTAIN");
                receipt["sell_error"] = json!(message);
            }
        }
    } else {
        journal(&journal_path, "BUY_ONLY_OPEN_POSITION", Some(&buy_signature))?;
    }
    Ok(receipt)
}

fn main() {
    let args = Args::parse();
    match run(args) {
        Ok(value) => println!("{}", serde_json::to_string_pretty(&value).unwrap()),
        Err(error) => {
            eprintln!("PILOT_REFUSED: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_devnet_genesis_passes() {
        assert!(guard(DEVNET_GENESIS, 1, 1, 1).is_ok());
        assert!(guard("5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d", 1, 1, 1).is_err());
        assert!(guard("4uhcVJyU9pJkvQyS88uRDiswHXSCkY3zQawwpjk2NsNY", 1, 1, 1).is_err());
    }

    #[test]
    fn cost_caps_are_fail_closed() {
        assert!(guard(DEVNET_GENESIS, 1_000_001, 1, 1).is_err());
        assert!(guard(DEVNET_GENESIS, 1, 5_000_001, 1).is_err());
        assert!(guard(DEVNET_GENESIS, 1, 5_000_000, 0).is_err());
        assert!(guard(DEVNET_GENESIS, 1, 5_000_000, 1).is_ok());
    }
}
