# Devnet-only Pump.fun V3 execution pilot — operator runbook

**Date:** 2026-11-04. **Status:** source implementation under review in PR #6. The pilot has not been run against the public network from this GitHub integration. **No actual Devnet buy/sell signature, token selection, economic win or profit is claimed.**

## Why Devnet instead of Solana Testnet?

Official Solana docs describe Testnet primarily as validator/stress infrastructure and Devnet as the app-development environment. Official Pump.fun/PumpSwap docs currently document the deployed programs on **Mainnet and Devnet**, not Testnet. Always call the cluster probe and verify actual executable accounts and genesis before assuming that venue exists; documentation does not guarantee your specific mint has liquidity or supports current builders.

- Devnet genesis: \`EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG\`.
- Testnet genesis: \`4uhcVJyU9pJkvQyS88uRDiswHXSCkY3zQawwpjk2NsNY\`.
- Pump.fun program: \`6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P\`.
- PumpSwap: \`pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA\`.

## Critical security boundary

\`devnet-pump-pilot\` is a **separate test-only binary**. It does not enable, reuse or relax \`sniper-app live\`. It refuses every genesis other than Devnet, and \`LIVE_TRADING_ENABLED=true\`. It refuses private-key environment variables, ignores the Solana CLI default wallet and writes its *own* faucet-funded keypair only under \`harness/devnet-keys/\` (gitignored). Do not paste a seed phrase or production signer into an agent terminal. The pilot key file is valuable solely for recovering a failed test trade and should not be mounted into LLM process environments.

Limits hardcoded in source: maximum 1,000,000 token base units, 0.005 SOL nominal **curve buy cap**, 0.03 devnet SOL faucet request when needed, exactly one optional buy and at most one optional sell per run, 400,000 CU limit. Account-creation rent, transaction fees, and any on-chain program side effects are additional; this cap is **not** a complete SOL total-spend guarantee. Do not reuse this test code for mainnet.

A durable write-ahead journal is written to \`harness/runs/devnet-pilot/<public-key>.jsonl\` before submitting. An existing journal refuses a new pilot buy for that identity to avoid blind resubmission after uncertain results. A confirmed buy followed by a failed sell leaves a position in the persisted test wallet. Do not delete the journal or key while unresolved. Never infer sell profits solely from wallet SOL balances: V3 account semantics include claimable volume accumulator effects.

## Reproduce locally

Prerequisites: pinned Rust toolchain, network access to Solana Devnet, an actual valid **Devnet** SOL-quoted Token-2022 Pump.fun mint whose curve is presently tradeable. Do not use a Mainnet mint.

\`\`\`bash
# Once the PR is merged: git pull --ff-only
# Until then:
git fetch origin codex/devnet-pilot-research-loop-20261104
git switch --track origin/codex/devnet-pilot-research-loop-20261104

# Read-only, no key
python3 harness/cluster_probe.py --cluster devnet
python3 harness/cluster_probe.py --cluster testnet

# Validate the selected Devnet mint without signing
CARGO_BUILD_JOBS=2 cargo run -p sniper-app --bin devnet-pump-pilot -- \
  --mint YOUR_VERIFIED_DEVNET_PUMP_MINT

# Only if Devnet preflight succeeds, an isolated faucet wallet can be used.
# This submits REAL Devnet program transactions, but NO valuable mainnet tokens:
CARGO_BUILD_JOBS=2 cargo run -p sniper-app --bin devnet-pump-pilot -- \
  --mint YOUR_VERIFIED_DEVNET_PUMP_MINT \
  --tokens 100000 --max-sol-cost-lamports 3000000 \
  --min-sell-quote-lamports 1 \
  --submit --roundtrip
\`\`\`

Use \`SNF_DEVNET_RPC\` for a private HTTPS Devnet RPC when necessary, not an API secret in a command argument. The tool checks **genesis**, not hostname. Public RPC rate limits, faucet denial, missing mint liquidity and unsupported/cashback quote variants are honest blockers.

The binary will not select a token, guarantee successful program simulation, fabricate confirmed signatures, automatically retry ambiguous orders, or evaluate an exit route without operator-provided devnet mint. This is **protocol mechanics testing**, not a competitive sniping strategy.

## Receipt and reconciliation rubric

If the pilot actually executes, collect from the user terminal:
1. Expected Devnet genesis; Pump executable; curve owner; token program.
2. Pilot public key and Devnet-only buy/sell transaction signatures (never private-key material).
3. Durable journal and exact before/after SOL, token ATA, curve and user volume accumulator balances.
4. Simulated and actual instruction logs, CU/fee and failure reason.
5. Independent check of token holdings and claimable proceeds after the transaction; document any discrepancy.

If the token account balance changed but the confirmed signature cannot be independently fetched, mark \`UNKNOWN_RECONCILE\`, not a winner. **Stop new orders until reconciliation.**

## No independent mainnet alpha from a testnet

Even successful Devnet buying and selling cannot measure adverse selection, real Pump.fun mainnet tip markets, market manipulation or realistic competing queue priority. Keep a separate read-only mainnet dataset and zero live capital for economics research.

Sources:
- https://solana.com/docs/references/clusters
- https://github.com/pump-fun/pump-public-docs/blob/main/docs/PUMP_PROGRAM_README.md
- https://github.com/pump-fun/pump-public-docs/blob/main/docs/PUMP_SWAP_README.md
- https://github.com/pump-fun/pump-public-docs/blob/main/docs/instructions/TRADE_V3.md
