# INFRA-2 — Operator RPC integration into the system + faucet status

**Date:** 2026-11-04 (local) / 2026-11-03 18:4x UTC (live runs)
**Authorization:** operator requested faucet persistence + integration of the two Infura RPCs into the system. Follows the full engineering loop (failing test first). Docs + Rust code changed; `LIVE_TRADING_ENABLED` untouched; local commit only.

## Scope delivered

1. **`sniper rpc-probe --cluster mainnet|devnet|testnet [--timeout-secs N]`** — a new
   read-only CLI subcommand that integrates the operator's endpoints into the system:
   - Endpoint resolution: `SNF_MAINNET_RPC_URL` / `SNF_DEVNET_RPC_URL` / `SNF_TESTNET_RPC_URL`
     (operator-private, per-shell env) take precedence; otherwise the pinned public cluster
     URL is used. A set-but-empty env value counts as unset. An INVALID env value is a hard
     error — never a silent public fallback.
   - Cluster identity verification against pinned genesis constants
     (mainnet `5eykt4Us…2N9d`, devnet `EtWTRABZ…PkrZBG`, testnet `4uhcVJyU…sNY`).
   - Only four read-only getter calls are made (`getGenesisHash`, `getVersion`, `getSlot`,
     `getEpochInfo`). The module imports no signer/keypair type and has no send path —
     read-only is structural, not conventional.
   - Printables are credential-safe: `redact_url` reduces any URL to scheme + host, so
     Infura path keys (`/v3/<key>`) cannot appear in reports, logs, or error strings
     (transport errors are sanitized by string-replacing the raw URL).
2. **Faucet persistence (operator ask "minta sampai dapat")** — see status below.

## Files changed

- `crates/sniper-runtime/src/rpc_probe.rs` — new module (resolution, validation, genesis verify, redaction, probe, report).
- `crates/sniper-runtime/src/lib.rs` — module + re-exports.
- `crates/sniper-runtime/tests/rpc_probe.rs` — 6 contract tests incl. an offline mock JSON-RPC server (loopback TCP) for the full probe path and the genesis-mismatch failure path.
- `crates/sniper-app/src/main.rs` — `RpcProbe` subcommand + `run_rpc_probe` helper.
- No dependency additions (`solana-client` 2.3.1 already pinned in the workspace).

## Acceptance results

| Acceptance | Result |
|---|---|
| env override wins; invalid scheme / embedded userinfo rejected; empty env = fallback | **PASSED** (`env_override_wins_and_is_validated`) |
| no env → pinned public URL per cluster | **PASSED** (`missing_env_falls_back_to_pinned_public_url`) |
| genesis constants verified; wrong-cluster genesis → `GenesisMismatch` | **PASSED** (`genesis_constants_are_verified`, mock-RPC hostile path) |
| `redact_url` strips path/query (Infura-shaped key never printable) | **PASSED** (`redact_url_strips_path_and_query` + live leak grep = 0 hits) |
| full probe against offline mock JSON-RPC (happy + mismatch, fixed call order asserted) | **PASSED** (`probe_cluster_verifies_genesis_against_mock_rpc`) |
| CLI parses `mainnet\|devnet\|testnet` | **PASSED** (`cluster_from_str_round_trips` + live runs) |
| invalid env URL → exit 1 with explicit message, no fallback | **PASSED** (live: `error: invalid RPC endpoint URL for devnet: scheme must be http or https`, exit=1) |
| unset env → public fallback, exit 0 | **PASSED** (live: `api.devnet.solana.com`, `public_cluster_default`) |

## Live receipts (2026-11-03 ~18:42 UTC, via `cargo run -p sniper-app -- rpc-probe`)

- **devnet** (source `operator_env/SNF_DEVNET_RPC_URL`, url_redacted `https://solana-devnet.infura.io`):
  genesis `EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG` verified, core 4.4.0 (fs 3722100051),
  slot 518309390, epoch 1199, block height 505471692. exit 0.
- **mainnet** (source `operator_env/SNF_MAINNET_RPC_URL`, url_redacted `https://solana-mainnet.infura.io`):
  genesis `5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d` verified, core 4.3.2 (fs 3383571666),
  slot 465003434, epoch 1076, block height 443024155. exit 0. Read-only calls only.
- **testnet** (source `public_cluster_default`, `https://api.testnet.solana.com`):
  genesis `4uhcVJyU9pJkvQyS88uRDiswHXSCkY3zQawwpjk2NsNY` verified, core 4.4.0,
  slot 460182659, epoch 1078. exit 0.
- Secret-leak grep over all probe outputs for the operator API key: **0 hits**.

## Gates

| Gate | Result |
|---|---|
| `cargo fmt` | **PASSED** |
| `cargo clippy --all-targets -- -D warnings` | **PASSED** |
| `cargo clippy --all-targets --all-features -- -D warnings` | **PASSED** |
| `cargo test` (workspace) | **PASSED** — 179 tests, 0 failed |
| `cargo test --all-features` | **PASSED** — 179 tests, 0 failed |
| Pre-commit secret guard (staged diff vs key/URL patterns) | **PASSED** (0 hits) |

Engineering-loop honesty note: the first RED run failed to compile (module absent), then two
real defects were found and fixed before green — (1) a mock-fixture wire-format bug
(`slotsPerEpoch` → `slotsInEpoch`, camelCase JSON-RPC field name, corrected against the SDK
deserializer, not to force green), and (2) a test-side deadlock where the mismatch-phase mock
expected 4 connections but the probe correctly stops after 1 (bound the mock's expected call
count instead).

## Faucet status ("minta token sampai dapat")

- **Devnet: FUNDED — sufficient.** Wallet `32qjUtSaDQy7t8AseCg4QGzu7wLFVV3uGBFcpXcsrYTM`
  holds **0.998995 SOL** (1 SOL airdropped via Infura `requestAirdrop` on 2026-11-03 while the
  faucet was still open, minus the 0.001 smoke transfer + fee).
- **Infura devnet `requestAirdrop` is NOW CLOSED**: HTTP 410
  `The RPC faucet is closed on this cluster. Request test SOL at https://faucet.solana.com`.
  Yesterday's success is not reproducible today — faucet openness is a provider/cluster state,
  not a stable capability. (Evidence: INFRA-1 vs this session.)
- **Testnet RPC faucet: CLOSED** (same 410 on `api.testnet.solana.com`, 4 attempts across
  ~10 minutes — this is a closure, not a rate limit; retrying further is pointless).
- **Web faucet API discovery (bounded):** `https://faucet.solana.com` serves a client-side
  Next.js app; no drip endpoint found in static assets, and `POST /api/drip` returns the app's
  404 page. No undocumented-API guessing beyond that.
- **BLOCKED — testnet funding requires manual action:** open https://faucet.solana.com,
  select **Testnet**, paste `5xTPhFkH6AUzAXo1tXSZjHi7b6RuR1gyuPWEZaF7BLe8`. Once funded,
  `sniper rpc-probe --cluster testnet` and any testnet tooling work unchanged.

## Integration pointers enabled by this work

- The Surfpool fork scripts (`harness/scripts/start_fork.sh`) read `SNF_SOURCE_RPC`; the
  operator's read-only mainnet URL may be supplied there (`$SNF_MAINNET_RPC_URL`) for M3c+
  fork work — env-only, never committed.
- Future observer/ingest providers (M2b+) can reuse `rpc_probe::resolve_endpoint` for
  consistent env-over-public endpoint selection with identity verification.

## Open defects

- None in the delivered scope. Testnet faucet funding remains operator-manual (see above).
