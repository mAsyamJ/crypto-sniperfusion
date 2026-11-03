---
name: devnet-protocol-pilot
description: Verify a SOL-paired Pump.fun token and perform strictly bounded opt-in Devnet V3 buy/sell protocol tests with isolated faucet wallet, durable journal and reconciliation.
---
# Devnet Protocol Pilot

Read \`AGENTS.md\`, \`docs/DEVNET_EXECUTION_PILOT.md\`, official Pump.fun V3 IDL, and \`crates/sniper-app/src/bin/devnet_pump_pilot.rs\`.

## Mandatory research sequence
1. Read-only probe \`python3 harness/cluster_probe.py --cluster devnet\`; reject non-Devnet genesis even if RPC hostname claims Devnet. Testnet is not a documented Pump.fun target.
2. Inspect specified Devnet mint/account owner, Token-2022 compatibility, SOL quote, bonding-curve active state, current fee config, pool readiness and sell path.
3. Run \`cargo run -p sniper-app --bin devnet-pump-pilot -- --mint <DEVNET_MINT>\` in no-send mode. This is the default.
4. For an explicit user-authorized Devnet program smoke only, run with \`--submit --roundtrip\`; use the tool-generated dedicated faucet wallet and fixed caps. This does **not** authorize arbitrary agent spending or automatic mint selection.
5. Inspect \`harness/runs/devnet-pilot/<pubkey>.jsonl\` and fetch actual buy/sell signatures and authoritative balances. Halt if any UNKNOWN state; do not retry automatically.
6. Classify evidence as DEVNET_PROGRAM_TRANSACTION only if real RPC confirmed receipts exist; otherwise NOT_EXECUTED.
7. Send results to separate accountant/security reviewer before making *mechanical correctness* claims.

Never load the default Solana CLI mainnet wallet or any user-funded signer. Never copy key JSON into the prompt or repository.
